//! Source-generation visibility and SQLite FTS5/sqlite-vec node projections.
use super::framework_storage::{BusinessTransaction, SharedDatabase, business_transaction};
use crate::{
    application::rag::graph::*, domain::rag::graph::GraphPartition,
    shared::sqlite_extensions::default_sqlite_extension_context,
};
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::BTreeSet;
mod cleanup;
mod search;
mod snapshot;
mod vector_checkpoint;

#[derive(Clone)]
pub struct SqliteRagGraphIndexRepository {
    database: SharedDatabase,
}
fn err(e: impl std::fmt::Display) -> String {
    format!("RAG graph index: {e}")
}
fn vector_table(dimension: usize) -> Result<String, String> {
    if !(1..=8192).contains(&dimension) {
        return Err(err("invalid dimension"));
    }
    Ok(format!("rag_graph_nodes_vec_d{dimension}"))
}
impl SqliteRagGraphIndexRepository {
    pub fn open(path: &str) -> Result<Self, String> {
        let database = SharedDatabase::open(path).map_err(err)?;
        Self::from_shared(database)
    }
    pub(crate) fn from_shared(database: SharedDatabase) -> Result<Self, String> {
        database.call(|connection| {
            connection.prepare("SELECT generation_id,provider_model FROM rag_graph_generations LIMIT 0").map_err(err)?;
            default_sqlite_extension_context().prepare_connection(connection).map_err(err)?;
            connection.execute_batch("CREATE VIRTUAL TABLE IF NOT EXISTS rag_graph_nodes_fts USING fts5(text, tokenize='simple')").map_err(err)
        },||err("database worker"))?;
        Ok(Self { database })
    }
}
fn manifest_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GraphGenerationManifest> {
    Ok(GraphGenerationManifest {
        id: row.get(0)?,
        partition: GraphPartition {
            corpus_id: row.get(1)?,
            scope_type: row.get(2)?,
            scope_code: row.get(3)?,
        },
        logical_source_code: row.get(4)?,
        source_code: row.get(5)?,
        source_version: row.get(6)?,
        task_id: row.get(7)?,
        configuration_hash: row.get(8)?,
        model_code: row.get(9)?,
        model_fingerprint: row.get(10)?,
        dimension: row.get(11)?,
        expected_projections: row.get(12)?,
        provider_model: row.get(13)?,
    })
}
const MANIFEST_COLUMNS: &str = "generation_id,corpus_id,scope_type,scope_code,logical_source_code,source_code,source_version,task_id,configuration_hash,model_code,model_fingerprint,dimension,expected_projections,provider_model";
fn stored_manifest(
    connection: &Connection,
    id: &str,
) -> Result<Option<GraphGenerationManifest>, String> {
    connection
        .query_row(
            &format!("SELECT {MANIFEST_COLUMNS} FROM rag_graph_generations WHERE generation_id=?1"),
            [id],
            manifest_row,
        )
        .optional()
        .map_err(err)
}
impl RagGraphIndexRepository for SqliteRagGraphIndexRepository {
    fn load_node_vector_checkpoint(
        &self,
        task_id: &str,
        cache_scope: &str,
        key: &str,
    ) -> Result<Option<GraphNodeVectorCheckpoint>, String> {
        let (task, scope, key) = (task_id.to_owned(), cache_scope.to_owned(), key.to_owned());
        self.database.call(
            move |c| vector_checkpoint::load(c, &task, &scope, &key),
            || err("database worker"),
        )
    }
    fn save_node_vector_checkpoint(
        &self,
        task_id: &str,
        cache_scope: &str,
        key: &str,
        expected_completed: usize,
        manifest: &GraphNodeVectorManifest,
        batch: Option<&GraphNodeVectorBatch>,
    ) -> Result<(), String> {
        let (task, scope, key, manifest, batch) = (
            task_id.to_owned(),
            cache_scope.to_owned(),
            key.to_owned(),
            manifest.clone(),
            batch.cloned(),
        );
        self.database.call(
            move |c| {
                vector_checkpoint::save(
                    c,
                    &task,
                    &scope,
                    &key,
                    expected_completed,
                    &manifest,
                    batch.as_ref(),
                )
            },
            || err("database worker"),
        )
    }
    fn claim_cleanup(&self, now: i64, lease_ms: i64) -> Result<Option<GraphCleanupClaim>, String> {
        self.database.call(
            move |connection| cleanup::claim(connection, now, lease_ms),
            || err("database worker"),
        )
    }
    fn finish_cleanup(
        &self,
        claim: &GraphCleanupClaim,
        now: i64,
        failure: Option<&str>,
    ) -> Result<(), String> {
        let (claim, failure) = (claim.clone(), failure.map(str::to_owned));
        self.database.call(
            move |connection| cleanup::finish(connection, &claim, now, failure.as_deref()),
            || err("database worker"),
        )
    }
    fn sweep_computation(&self, now: i64, limit: usize) -> Result<GraphHousekeepingResult, String> {
        self.database.call(
            move |connection| cleanup::sweep(connection, now, limit),
            || err("database worker"),
        )
    }

    fn save_snapshot(
        &self,
        draft: GraphSnapshotDraft,
        now: i64,
    ) -> Result<GraphQuerySnapshot, GraphSnapshotError> {
        self.database.call(
            move |connection| snapshot::save(connection, draft, now),
            || GraphSnapshotError::Storage("database worker".into()),
        )
    }
    fn load_snapshot(
        &self,
        identity_hash: &str,
        cursor: &str,
        now: i64,
    ) -> Result<GraphQuerySnapshot, GraphSnapshotError> {
        let (identity, cursor) = (identity_hash.to_owned(), cursor.to_owned());
        self.database.call(
            move |connection| snapshot::load(connection, &identity, &cursor, now),
            || GraphSnapshotError::Storage("database worker".into()),
        )
    }
    fn search_nodes(&self, query: GraphNodeSearchQuery) -> Result<GraphNodeSearchPage, String> {
        self.database.call(
            move |connection| search::search_nodes(connection, query),
            || err("database worker"),
        )
    }
    fn load_checkpoint(
        &self,
        task_id: &str,
        cache_scope: &str,
        key: &str,
        reuse_published: bool,
    ) -> Result<Option<serde_json::Value>, String> {
        let (task_id, scope, key) = (task_id.to_owned(), cache_scope.to_owned(), key.to_owned());
        self.database.call(move |connection| {
            let value: Option<String> = connection.query_row(
                "SELECT c.payload_json FROM rag_graph_task_checkpoints c WHERE c.cache_scope=?2 AND c.checkpoint_key=?3 \
                 AND (c.task_id=?1 OR (?4 AND EXISTS (SELECT 1 FROM rag_graph_active_sources a \
                 JOIN rag_graph_generations g ON g.generation_id=a.generation_id WHERE g.task_id=c.task_id AND g.state='Active' AND (c.checkpoint_key NOT LIKE 'NodeVectors_%' OR g.vector_cache_invalidated=0)))) \
                 ORDER BY (c.task_id=?1) DESC,c.updated_at DESC LIMIT 1",params![task_id,scope,key,reuse_published],|r|r.get(0)).optional().map_err(err)?;
            value.map(|text|serde_json::from_str(&text).map_err(err)).transpose()
        },||err("database worker"))
    }
    fn save_checkpoint(
        &self,
        task_id: &str,
        cache_scope: &str,
        key: &str,
        value: &serde_json::Value,
    ) -> Result<(), String> {
        let (task_id, scope, key, value) = (
            task_id.to_owned(),
            cache_scope.to_owned(),
            key.to_owned(),
            serde_json::to_string(value).map_err(err)?,
        );
        self.database.call(move |connection| {
            connection.execute("INSERT INTO rag_graph_task_checkpoints(task_id,cache_scope,checkpoint_key,payload_json,updated_at) \
                VALUES (?1,?2,?3,?4,?5) ON CONFLICT(task_id,cache_scope,checkpoint_key) \
                DO UPDATE SET payload_json=excluded.payload_json,updated_at=excluded.updated_at",
                params![task_id,scope,key,value,chrono::Utc::now().timestamp_millis()]).map_err(err)?;Ok(())
        },||err("database worker"))
    }
    fn export_generation(
        &self,
        id: &str,
    ) -> Result<Option<(GraphGenerationManifest, Vec<PreparedGraphProjection>)>, String> {
        let id = id.to_owned();
        self.database.call(move|connection|{
            let Some(manifest)=stored_manifest(connection,&id)? else{return Ok(None);};
            let mut statement=connection.prepare("SELECT entity_id,chunk_codes_json,text,embedding FROM rag_graph_node_projections WHERE generation_id=?1 ORDER BY entity_id").map_err(err)?;
            let rows=statement.query_map([id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,Vec<u8>>(3)?))).map_err(err)?;
            let mut projections=Vec::new();
            for row in rows {
                let (entity_id,chunks,text,blob)=row.map_err(err)?;
                if blob.len()!=manifest.dimension*4 {return Err(err("invalid stored node vector geometry"));}
                let embedding=blob.chunks_exact(4).map(|b|f32::from_le_bytes(b.try_into().unwrap())).collect();
                let projection=PreparedGraphProjection{entity_id,chunk_codes:serde_json::from_str(&chunks).map_err(err)?,text,embedding};
                projection.validate(manifest.dimension)?;projections.push(projection);
            }
            if projections.len()!=manifest.expected_projections{return Err(err("exported node projection count differs"));}
            Ok(Some((manifest,projections)))
        },||err("database worker"))
    }
    fn begin_generation(&self, manifest: &GraphGenerationManifest) -> Result<(), String> {
        manifest.validate()?;
        let manifest = manifest.clone();
        self.database.call(move|connection|{
            let tx=business_transaction(connection,rusqlite::TransactionBehavior::Immediate).map_err(err)?;
            if let Some(existing)=stored_manifest(&tx,&manifest.id)? {
                if existing!=manifest{return Err(err("immutable generation manifest changed"));}
                let state:String=tx.query_row("SELECT state FROM rag_graph_generations WHERE generation_id=?1",[&manifest.id],|r|r.get(0)).map_err(err)?;
                if state=="Cleaning" {return Err(err("graph generation is being reclaimed; retry after cleanup"));}
                return Ok(());
            }
            let now=chrono::Utc::now().timestamp_millis();
            tx.execute("INSERT INTO rag_graph_generations(generation_id,partition_key,corpus_id,scope_type,scope_code,logical_source_code,source_code,source_version,task_id,configuration_hash,model_code,model_fingerprint,dimension,state,expected_projections,created_at,updated_at,provider_model) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,'Staging',?14,?15,?15,?16)",
                params![manifest.id,manifest.partition.key(),manifest.partition.corpus_id,manifest.partition.scope_type,manifest.partition.scope_code,
                    manifest.logical_source_code,manifest.source_code,manifest.source_version,manifest.task_id,manifest.configuration_hash,
                    manifest.model_code,manifest.model_fingerprint,manifest.dimension,manifest.expected_projections,now,manifest.provider_model]).map_err(err)?;
            tx.execute("INSERT INTO rag_graph_cleanup_jobs(generation_id,not_before,updated_at) VALUES (?1,?2,?3)",
                params![manifest.id,now+300_000,now]).map_err(err)?;
            tx.commit().map_err(err)
        },||err("database worker"))
    }
    fn stage_projections(
        &self,
        manifest: &GraphGenerationManifest,
        projections: &[PreparedGraphProjection],
    ) -> Result<(), String> {
        manifest.validate()?;
        if projections.len() != manifest.expected_projections {
            return Err(err("node projection count differs"));
        }
        let mut ids = BTreeSet::new();
        for p in projections {
            p.validate(manifest.dimension)?;
            if !ids.insert(&p.entity_id) {
                return Err(err("duplicate node projection"));
            }
        }
        let manifest = manifest.clone();
        let projections = projections.to_vec();
        self.database.call(move|connection|{
            if stored_manifest(connection,&manifest.id)?.as_ref()!=Some(&manifest){return Err(err("staged graph manifest changed or is absent"));}
            let table=vector_table(manifest.dimension)?;
            connection.execute_batch(&format!("CREATE VIRTUAL TABLE IF NOT EXISTS {table} USING vec0(embedding float[{}] distance_metric=cosine)",manifest.dimension)).map_err(err)?;
            let tx=business_transaction(connection,rusqlite::TransactionBehavior::Immediate).map_err(err)?;
            let state:String=tx.query_row("SELECT state FROM rag_graph_generations WHERE generation_id=?1",[&manifest.id],|row|row.get(0)).map_err(err)?;
            if !matches!(state.as_str(),"Staging"|"Ready"){return Err(err("cannot change published graph projections"));}
            let rowids={let mut stmt=tx.prepare("SELECT id FROM rag_graph_node_projections WHERE generation_id=?1").map_err(err)?;
                stmt.query_map([&manifest.id],|r|r.get::<_,i64>(0)).map_err(err)?.collect::<Result<Vec<_>,_>>().map_err(err)?};
            for id in rowids {
                tx.execute("DELETE FROM rag_graph_nodes_fts WHERE rowid=?1",[id]).map_err(err)?;
                tx.execute(&format!("DELETE FROM {table} WHERE rowid=?1"),[id]).map_err(err)?;
            }
            tx.execute("DELETE FROM rag_graph_node_projections WHERE generation_id=?1",[&manifest.id]).map_err(err)?;
            for projection in projections {
                let embedding=projection.embedding.iter().flat_map(|v|v.to_le_bytes()).collect::<Vec<_>>();
                tx.execute("INSERT INTO rag_graph_node_projections(entity_id,generation_id,chunk_codes_json,text,embedding) VALUES (?1,?2,?3,?4,?5)",
                    params![projection.entity_id,manifest.id,serde_json::to_string(&projection.chunk_codes).map_err(err)?,projection.text,embedding]).map_err(err)?;
                let id=tx.last_insert_rowid();
                tx.execute("INSERT INTO rag_graph_nodes_fts(rowid,text) VALUES (?1,?2)",params![id,projection.text]).map_err(err)?;
                tx.execute(&format!("INSERT INTO {table}(rowid,embedding) VALUES (?1,?2)"),params![id,embedding]).map_err(err)?;
            }
            tx.execute("UPDATE rag_graph_generations SET state='Ready',updated_at=?2 WHERE generation_id=?1",params![manifest.id,chrono::Utc::now().timestamp_millis()]).map_err(err)?;
            tx.commit().map_err(err)
        },||err("database worker"))
    }
    fn active_generations(
        &self,
        partitions: &[GraphPartition],
    ) -> Result<Vec<GraphGenerationManifest>, String> {
        let partitions = partitions.to_vec();
        self.database.call(move|connection|{
            let mut result=Vec::new();
            let columns=MANIFEST_COLUMNS.split(',').map(|name|format!("g.{name}")).collect::<Vec<_>>().join(",");
            let keys = partitions.iter().map(GraphPartition::key).collect::<BTreeSet<_>>();
            let mut stmt = connection.prepare(&format!(
                "SELECT {columns} FROM rag_graph_generations g JOIN rag_graph_active_sources a ON a.generation_id=g.generation_id \
                 WHERE a.partition_key IN (SELECT value FROM json_each(?1)) AND g.state='Active' ORDER BY g.generation_id"
            )).map_err(err)?;
            result.extend(stmt.query_map([serde_json::to_string(&keys).map_err(err)?], manifest_row).map_err(err)?
                .collect::<Result<Vec<_>, _>>().map_err(err)?);
            Ok(result)
        },||err("database worker"))
    }
}

/// Verify source binding inside the caller's publication transaction.
pub(crate) fn validate_graph_generation_binding(
    tx: &BusinessTransaction<'_>,
    id: &str,
    partition: &GraphPartition,
    logical: &str,
    source: &str,
    task: &str,
    configuration: &str,
) -> Result<(), String> {
    validate_graph_generation_bindings(
        tx,
        &[(
            id.to_owned(),
            partition.clone(),
            logical.to_owned(),
            source.to_owned(),
            task.to_owned(),
            configuration.to_owned(),
        )],
    )
}

pub(crate) type GraphPublicationBinding = (String, GraphPartition, String, String, String, String);
pub(crate) fn validate_graph_generation_bindings(
    tx: &BusinessTransaction<'_>,
    bindings: &[GraphPublicationBinding],
) -> Result<(), String> {
    if bindings.is_empty() {
        return Ok(());
    }
    let ids = serde_json::to_string(&bindings.iter().map(|item| &item.0).collect::<Vec<_>>())
        .map_err(err)?;
    let mut query=tx.prepare(&format!("SELECT {MANIFEST_COLUMNS} FROM rag_graph_generations WHERE generation_id IN (SELECT value FROM json_each(?1))")).map_err(err)?;
    let manifests = query
        .query_map([ids], manifest_row)
        .map_err(err)?
        .map(|row| {
            let manifest = row.map_err(err)?;
            Ok((manifest.id.clone(), manifest))
        })
        .collect::<Result<std::collections::BTreeMap<_, _>, String>>()?;
    for (id, partition, logical, source, task, configuration) in bindings {
        let manifest = manifests
            .get(id)
            .ok_or_else(|| err("graph generation is absent"))?;
        if manifest.partition != *partition
            || manifest.logical_source_code != *logical
            || manifest.source_code != *source
            || manifest.task_id != *task
            || manifest.configuration_hash != *configuration
        {
            return Err(err("graph generation does not belong to this source task"));
        }
    }
    Ok(())
}
pub(crate) fn retire_graph_source_version(
    tx: &BusinessTransaction<'_>,
    partition: &GraphPartition,
    source: &str,
    version: &str,
) -> Result<(), String> {
    let logicals = {
        let mut statement=tx.prepare("SELECT g.logical_source_code FROM rag_graph_active_sources a JOIN rag_graph_generations g ON g.generation_id=a.generation_id \
        WHERE a.partition_key=?1 AND g.source_code=?2 AND g.source_version=?3").map_err(err)?;
        statement
            .query_map(params![partition.key(), source, version], |r| {
                r.get::<_, String>(0)
            })
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?
    };
    for logical in logicals {
        retire_graph_source(tx, partition, &logical)?;
    }
    Ok(())
}
pub(crate) fn retire_graph_corpus(
    tx: &BusinessTransaction<'_>,
    corpus: &str,
) -> Result<(), String> {
    let sources = {
        let mut statement=tx.prepare("SELECT g.scope_type,g.scope_code,g.logical_source_code \
        FROM rag_graph_active_sources a JOIN rag_graph_generations g ON g.generation_id=a.generation_id WHERE g.corpus_id=?1").map_err(err)?;
        statement
            .query_map([corpus], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?
    };
    for (scope_type, scope_code, logical) in sources {
        retire_graph_source(
            tx,
            &GraphPartition {
                corpus_id: corpus.into(),
                scope_type,
                scope_code,
            },
            &logical,
        )?;
    }
    Ok(())
}
pub(crate) fn retire_graph_source(
    tx: &BusinessTransaction<'_>,
    partition: &GraphPartition,
    logical: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp_millis();
    let old:Option<String>=tx.query_row("SELECT generation_id FROM rag_graph_active_sources WHERE partition_key=?1 AND logical_source_code=?2",
        params![partition.key(),logical],|r|r.get(0)).optional().map_err(err)?;
    if let Some(old) = old {
        tx.execute("DELETE FROM rag_graph_active_sources WHERE partition_key=?1 AND logical_source_code=?2",params![partition.key(),logical]).map_err(err)?;
        tx.execute(
            "UPDATE rag_graph_generations SET state='Retired',updated_at=?2 WHERE generation_id=?1",
            params![old, now],
        )
        .map_err(err)?;
        tx.execute("INSERT INTO rag_graph_cleanup_jobs(generation_id,not_before,updated_at) VALUES (?1,?2,?3) ON CONFLICT(generation_id) DO UPDATE SET not_before=excluded.not_before,updated_at=excluded.updated_at",params![old,now+300_000,now]).map_err(err)?;
    }
    Ok(())
}

/// Called within the source/task publication transaction, after the caller fences
/// cancellation, configuration and lease ownership. Never use an independent commit.
pub(crate) fn activate_graph_generation(
    tx: &BusinessTransaction<'_>,
    id: &str,
) -> Result<(), String> {
    activate_graph_generations(tx, &[id.to_owned()])
}

/// Batch readiness checks share the caller's publication transaction.
pub(crate) fn activate_graph_generations(
    tx: &BusinessTransaction<'_>,
    ids: &[String],
) -> Result<(), String> {
    if ids.is_empty() {
        return Ok(());
    }
    let selected = serde_json::to_string(ids).map_err(err)?;
    let columns = MANIFEST_COLUMNS
        .split(',')
        .map(|name| format!("g.{name}"))
        .collect::<Vec<_>>()
        .join(",");
    let mut query=tx.prepare(&format!("SELECT {columns},g.state,COALESCE(p.actual,0),a.generation_id FROM rag_graph_generations g LEFT JOIN (SELECT generation_id,count(*) AS actual FROM rag_graph_node_projections WHERE generation_id IN (SELECT value FROM json_each(?1)) GROUP BY generation_id) p ON p.generation_id=g.generation_id LEFT JOIN rag_graph_active_sources a ON a.partition_key=g.partition_key AND a.logical_source_code=g.logical_source_code WHERE g.generation_id IN (SELECT value FROM json_each(?1))")).map_err(err)?;
    let mut manifests = query
        .query_map([selected], |row| {
            Ok((
                manifest_row(row)?,
                row.get::<_, String>(14)?,
                row.get::<_, usize>(15)?,
                row.get::<_, Option<String>>(16)?,
            ))
        })
        .map_err(err)?
        .map(|row| {
            let (manifest, state, count, old) = row.map_err(err)?;
            Ok((manifest.id.clone(), (manifest, state, count, old)))
        })
        .collect::<Result<std::collections::BTreeMap<_, _>, String>>()?;
    let mut active = std::collections::BTreeMap::new();
    for (manifest, _, _, old) in manifests.values() {
        active.insert(
            (
                manifest.partition.key(),
                manifest.logical_source_code.clone(),
            ),
            old.clone(),
        );
    }
    for id in ids {
        let (manifest, state, count, _) = manifests
            .get(id)
            .cloned()
            .ok_or_else(|| err("graph generation is absent"))?;
        if state == "Active" {
            continue;
        }
        if state != "Ready" {
            return Err(err("graph generation is not ready"));
        }
        if count != manifest.expected_projections {
            return Err(err("graph projection count differs at publication"));
        }
        let now = chrono::Utc::now().timestamp_millis();
        let partition = manifest.partition.key();
        let key = (partition.clone(), manifest.logical_source_code.clone());
        if let Some(old) = active
            .get(&key)
            .and_then(Option::as_ref)
            .filter(|old| *old != id)
        {
            tx.execute("UPDATE rag_graph_generations SET state='Retired',updated_at=?2 WHERE generation_id=?1",params![old,now]).map_err(err)?;
            tx.execute("INSERT INTO rag_graph_cleanup_jobs(generation_id,not_before,updated_at) VALUES (?1,?2,?3) ON CONFLICT(generation_id) DO UPDATE SET not_before=excluded.not_before,updated_at=excluded.updated_at",params![old,now+300_000,now]).map_err(err)?;
            if let Some(entry) = manifests.get_mut(old) {
                entry.1 = "Retired".into();
            }
        }
        tx.execute("INSERT INTO rag_graph_active_sources(partition_key,logical_source_code,generation_id,publication_epoch,updated_at) VALUES (?1,?2,?3,1,?4) ON CONFLICT(partition_key,logical_source_code) DO UPDATE SET generation_id=excluded.generation_id,publication_epoch=publication_epoch+1,updated_at=excluded.updated_at",params![partition,manifest.logical_source_code,id,now]).map_err(err)?;
        tx.execute(
            "DELETE FROM rag_graph_cleanup_jobs WHERE generation_id=?1",
            [id],
        )
        .map_err(err)?;
        tx.execute(
            "UPDATE rag_graph_generations SET state='Active',updated_at=?2 WHERE generation_id=?1",
            params![id, now],
        )
        .map_err(err)?;
        if let Some(entry) = manifests.get_mut(id) {
            entry.1 = "Active".into();
        }
        active.insert(key, Some(id.clone()));
    }
    Ok(())
}
