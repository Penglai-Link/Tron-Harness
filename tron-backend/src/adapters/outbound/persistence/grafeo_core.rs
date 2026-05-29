use std::{
    collections::HashSet,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use grafeo::{GrafeoDB, Session};

use crate::domain::graph::ResolvedGraphScope;

pub(crate) const GRAPH_NODE_LABEL: &str = "TronGraphNode";
pub(crate) const ONTOLOGY_RESOURCE_LABEL: &str = "TronOntologyResource";

const NODE_CODE_INDEX: &str = "idx_tron_node_code";
const NODE_SCOPE_INDEX: &str = "idx_tron_node_scope";
const ONTOLOGY_CODE_INDEX: &str = "idx_tron_ontology_code";
const ONTOLOGY_TARGET_INDEX: &str = "idx_tron_ontology_target";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrafeoAdapterConfig {
    pub storage_path: String,
}

#[derive(Clone)]
pub struct GrafeoStoreCore {
    database: Arc<GrafeoDB>,
    bootstrapped_schemas: Arc<Mutex<HashSet<String>>>,
    bootstrapped_graphs: Arc<Mutex<HashSet<String>>>,
}

impl GrafeoStoreCore {
    pub fn connect(config: GrafeoAdapterConfig) -> Result<Self, grafeo::Error> {
        let database = open_database(&config.storage_path)?;

        Ok(Self {
            database: Arc::new(database),
            bootstrapped_schemas: Arc::new(Mutex::new(HashSet::new())),
            bootstrapped_graphs: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    pub(crate) fn session_for_scope(
        &self,
        scope: &ResolvedGraphScope,
    ) -> Result<Session, grafeo::Error> {
        let session = self.database.session();
        self.ensure_scope(&session, scope)?;
        Ok(session)
    }

    fn ensure_scope(
        &self,
        session: &Session,
        scope: &ResolvedGraphScope,
    ) -> Result<(), grafeo::Error> {
        let schema = encode_schema_name(&scope.schema);
        if remember_once(&self.bootstrapped_schemas, &schema) {
            session.execute(&format!("CREATE SCHEMA {schema} IF NOT EXISTS"))?;
        }

        session.execute(&format!("SESSION SET SCHEMA {schema}"))?;

        let named_graph = encode_graph_name(&scope.named_graph);
        let graph_key = format!("{schema}/{named_graph}");
        let bootstrap_graph = remember_once(&self.bootstrapped_graphs, &graph_key);
        if bootstrap_graph {
            session.execute(&format!("CREATE GRAPH {named_graph} IF NOT EXISTS"))?;
        }

        session.execute(&format!("USE GRAPH {named_graph}"))?;
        if bootstrap_graph {
            self.create_indexes(session)?;
        }

        Ok(())
    }

    fn create_indexes(&self, session: &Session) -> Result<(), grafeo::Error> {
        create_index(
            session,
            &format!("CREATE INDEX {NODE_CODE_INDEX} FOR (n:{GRAPH_NODE_LABEL}) ON (n.nodeCode)"),
        )?;
        create_index(
            session,
            &format!(
                "CREATE INDEX {NODE_SCOPE_INDEX} FOR (n:{GRAPH_NODE_LABEL}) ON (n.workspaceCode)"
            ),
        )?;
        create_index(
            session,
            &format!(
                "CREATE INDEX {ONTOLOGY_CODE_INDEX} FOR (n:{ONTOLOGY_RESOURCE_LABEL}) ON (n.resourceCode)"
            ),
        )?;
        create_index(
            session,
            &format!(
                "CREATE INDEX {ONTOLOGY_TARGET_INDEX} FOR (n:{ONTOLOGY_RESOURCE_LABEL}) ON (n.targetClassCode)"
            ),
        )?;

        Ok(())
    }
}

fn remember_once(cache: &Mutex<HashSet<String>>, key: &str) -> bool {
    let mut cache = cache.lock().expect("grafeo bootstrap cache mutex poisoned");
    cache.insert(key.to_owned())
}

fn create_index(session: &Session, query: &str) -> Result<(), grafeo::Error> {
    match session.execute(query) {
        Ok(_) => Ok(()),
        Err(error) if error.to_string().contains("already exists") => Ok(()),
        Err(error) => Err(error),
    }
}

fn open_database(storage_path: &str) -> Result<GrafeoDB, grafeo::Error> {
    if storage_path == ":memory:" {
        return Ok(GrafeoDB::new_in_memory());
    }

    ensure_parent_directory(storage_path)?;
    GrafeoDB::open(storage_path)
}

pub fn encode_schema_name(value: &str) -> String {
    encode_storage_name("ns", value)
}

pub fn encode_graph_name(value: &str) -> String {
    encode_storage_name("graph", value)
}

fn encode_storage_name(prefix: &str, value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut encoded = String::from(prefix);
    encoded.push('_');
    for byte in value.as_bytes() {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn ensure_parent_directory(storage_path: &str) -> Result<(), grafeo::Error> {
    let Some(parent) = Path::new(storage_path).parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }

    fs::create_dir_all(parent)
        .map_err(|error| grafeo::Error::InvalidValue(format!("graph storage path error: {error}")))
}
