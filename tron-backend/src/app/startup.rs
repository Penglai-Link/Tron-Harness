use std::{
    error::Error,
    fmt,
    sync::{Arc, Mutex},
};

use crate::{
    adapters::outbound::persistence::{
        GrafeoAdapterConfig, GrafeoGraphExplorerStore, GrafeoGraphStore, GrafeoOntologyStore,
        GrafeoStoreCore, GraphBackedRagRepository, sqlite::SqliteRagRepository,
    },
    application::{
        graph::{GraphExplorerService, GraphService, OntologyService, ScopeResolver},
        llm::LlmRuntimeService,
        model_router::LlmModelRouter,
        prompt::PromptService,
        rag::{DefaultRagRuntime, DefaultRagStrategyRegistry, RagService},
        workspace::WorkspaceService,
    },
    config::MineruConfig,
    domain::rag::RagError,
    shared::event_bus::EventBusHandle,
};

use super::AppRagService;

#[derive(Clone)]
pub struct AppReadiness {
    probe: Option<AppReadinessProbe>,
    startup_status: AppReadinessStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppReadinessStatus {
    pub storage_ready: bool,
    pub graph_ready: bool,
    pub rag_ready: bool,
}

#[derive(Clone)]
struct AppReadinessProbe {
    database_url: String,
    graph_storage_path: String,
}

impl AppReadiness {
    pub fn new(database_url: impl Into<String>, graph_storage_path: impl Into<String>) -> Self {
        Self {
            probe: Some(AppReadinessProbe {
                database_url: database_url.into(),
                graph_storage_path: graph_storage_path.into(),
            }),
            startup_status: AppReadinessStatus {
                storage_ready: false,
                graph_ready: false,
                rag_ready: false,
            },
        }
    }

    pub fn validated_ready() -> Self {
        Self {
            probe: None,
            startup_status: AppReadinessStatus {
                storage_ready: true,
                graph_ready: true,
                rag_ready: true,
            },
        }
    }

    pub fn check(&self) -> AppReadinessStatus {
        let Some(probe) = &self.probe else {
            return self.startup_status;
        };

        let storage_ready = SqliteRagRepository::connect(&probe.database_url).is_ok();
        let graph_ready = GrafeoStoreCore::connect(GrafeoAdapterConfig {
            storage_path: probe.graph_storage_path.clone(),
        })
        .is_ok();
        AppReadinessStatus {
            storage_ready,
            graph_ready,
            rag_ready: storage_ready && graph_ready,
        }
    }
}

#[derive(Debug)]
pub enum AppBootstrapError {
    GraphInitFailed {
        graph_storage_path: String,
        source: grafeo::Error,
    },
    RagInitFailed {
        database_url: String,
        source: RagError,
    },
}

impl fmt::Display for AppBootstrapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GraphInitFailed {
                graph_storage_path, ..
            } => write!(
                formatter,
                "graph dependency failed to initialize for storage path `{graph_storage_path}`"
            ),
            Self::RagInitFailed { database_url, .. } => write!(
                formatter,
                "RAG dependency failed to initialize for database `{database_url}`"
            ),
        }
    }
}

impl Error for AppBootstrapError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::GraphInitFailed { source, .. } => Some(source),
            Self::RagInitFailed { source, .. } => Some(source),
        }
    }
}

pub(super) fn build_rag_service(
    database_url: &str,
    graph_core: GrafeoStoreCore,
    workspaces: WorkspaceService,
    llm_runtime: LlmRuntimeService,
    llm_router: LlmModelRouter,
    prompt_service: PromptService,
    project_doc_root: &str,
    mineru: MineruConfig,
    document_parsing_client: reqwest::Client,
    event_bus: EventBusHandle,
) -> Result<AppRagService, AppBootstrapError> {
    let sqlite = SqliteRagRepository::connect(database_url).map_err(|source| {
        AppBootstrapError::RagInitFailed {
            database_url: database_url.to_owned(),
            source,
        }
    })?;
    let repository = GraphBackedRagRepository::new(
        sqlite,
        Arc::new(GrafeoGraphStore::new(graph_core)),
        Arc::new(ScopeResolver::new(workspaces)),
    );
    Ok(RagService::new(
        Arc::new(Mutex::new(repository)),
        DefaultRagStrategyRegistry::default(),
        DefaultRagRuntime::new(llm_runtime, llm_router, project_doc_root)
            .with_document_parser(mineru, document_parsing_client),
    )
    .with_event_bus(event_bus)
    .with_prompt_service(prompt_service))
}

pub(super) fn build_graph_core(
    graph_storage_path: &str,
) -> Result<GrafeoStoreCore, AppBootstrapError> {
    GrafeoStoreCore::connect(GrafeoAdapterConfig {
        storage_path: graph_storage_path.to_owned(),
    })
    .map_err(|source| AppBootstrapError::GraphInitFailed {
        graph_storage_path: graph_storage_path.to_owned(),
        source,
    })
}

pub(super) fn build_graph_services(
    core: GrafeoStoreCore,
    workspaces: WorkspaceService,
    llm_runtime: LlmRuntimeService,
) -> (GraphService, OntologyService, GraphExplorerService) {
    let graphs = GraphService::with_embedding_runtime(
        GrafeoGraphStore::new(core.clone()),
        ScopeResolver::new(workspaces.clone()),
        Arc::new(llm_runtime),
    );
    let ontologies = OntologyService::new(
        GrafeoOntologyStore::new(core.clone()),
        ScopeResolver::new(workspaces.clone()),
    );
    let explorer = GraphExplorerService::new(
        GrafeoGraphExplorerStore::new(core),
        ScopeResolver::new(workspaces.clone()),
        workspaces,
    );

    (graphs, ontologies, explorer)
}
