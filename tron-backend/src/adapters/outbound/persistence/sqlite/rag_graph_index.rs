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
    let manifest = stored_manifest(tx, id)?.ok_or_else(|| err("graph generation is absent"))?;
    if manifest.partition != *partition
        || manifest.logical_source_code != logical
        || manifest.source_code != source
        || manifest.task_id != task
        || manifest.configuration_hash != configuration
    {
        return Err(err("graph generation does not belong to this source task"));
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
    let manifest = stored_manifest(tx, id)?.ok_or_else(|| err("graph generation is absent"))?;
    let state: String = tx
        .query_row(
            "SELECT state FROM rag_graph_generations WHERE generation_id=?1",
            [id],
            |r| r.get(0),
        )
        .map_err(err)?;
    if state == "Active" {
        return Ok(());
    }
    if state != "Ready" {
        return Err(err("graph generation is not ready"));
    }
    let count: usize = tx
        .query_row(
            "SELECT count(*) FROM rag_graph_node_projections WHERE generation_id=?1",
            [id],
            |r| r.get(0),
        )
        .map_err(err)?;
    if count != manifest.expected_projections {
        return Err(err("graph projection count differs at publication"));
    }
    let now = chrono::Utc::now().timestamp_millis();
    let partition = manifest.partition.key();
    let old:Option<String>=tx.query_row("SELECT generation_id FROM rag_graph_active_sources WHERE partition_key=?1 AND logical_source_code=?2",params![partition,manifest.logical_source_code],|r|r.get(0)).optional().map_err(err)?;
    if let Some(old) = old.filter(|old| old != id) {
        tx.execute(
            "UPDATE rag_graph_generations SET state='Retired',updated_at=?2 WHERE generation_id=?1",
            params![old, now],
        )
        .map_err(err)?;
        tx.execute("INSERT INTO rag_graph_cleanup_jobs(generation_id,not_before,updated_at) VALUES (?1,?2,?3) ON CONFLICT(generation_id) DO UPDATE SET not_before=excluded.not_before,updated_at=excluded.updated_at",params![old,now+300_000,now]).map_err(err)?;
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn manifest(id: &str) -> GraphGenerationManifest {
        GraphGenerationManifest {
            id: id.into(),
            partition: GraphPartition {
                corpus_id: "manual".into(),
                scope_type: "USER".into(),
                scope_code: "owner".into(),
            },
            logical_source_code: "document".into(),
            source_code: format!("document-{id}"),
            source_version: "v1".into(),
            task_id: id.into(),
            configuration_hash: "config".into(),
            model_code: "model".into(),
            model_fingerprint: "space-2".into(),
            provider_model: None,
            dimension: 2,
            expected_projections: 1,
        }
    }
    fn projection() -> PreparedGraphProjection {
        PreparedGraphProjection {
            entity_id: "entity".into(),
            chunk_codes: vec!["chunk".into()],
            text: "中文图谱检索证据 Alpha service".into(),
            embedding: vec![1., 0.],
        }
    }
    fn publish(
        repository: &SqliteRagGraphIndexRepository,
        id: &str,
        commit: bool,
    ) -> Result<(), String> {
        let id = id.to_owned();
        repository.database.call(
            move |connection| {
                let tx = business_transaction(connection, rusqlite::TransactionBehavior::Immediate)
                    .map_err(err)?;
                activate_graph_generation(&tx, &id)?;
                if commit {
                    tx.commit().map_err(err)?;
                }
                Ok(())
            },
            || "worker".into(),
        )
    }
    #[test]
    fn staged_indexes_are_invisible_and_publication_rolls_back_with_source_transaction() {
        let repository = SqliteRagGraphIndexRepository::open(":memory:").unwrap();
        let one = manifest("one");
        repository.begin_generation(&one).unwrap();
        assert!(publish(&repository, &one.id, true).is_err());
        repository.stage_projections(&one, &[projection()]).unwrap();
        assert!(
            repository
                .active_generations(&[one.partition.clone()])
                .unwrap()
                .is_empty()
        );
        publish(&repository, &one.id, false).unwrap();
        assert!(
            repository
                .active_generations(&[one.partition.clone()])
                .unwrap()
                .is_empty()
        );
        publish(&repository, &one.id, true).unwrap();
        let mut changed = one.clone();
        changed.model_fingerprint = "other-model".into();
        assert!(repository.begin_generation(&changed).is_err());
        assert!(repository.stage_projections(&one, &[projection()]).is_err());
        assert_eq!(
            repository
                .active_generations(&[one.partition.clone()])
                .unwrap(),
            vec![one.clone()]
        );
        let two = manifest("two");
        repository.begin_generation(&two).unwrap();
        let mut invalid = projection();
        invalid.embedding = vec![f32::NAN, 0.];
        assert!(repository.stage_projections(&two, &[invalid]).is_err());
        assert_eq!(
            repository
                .active_generations(&[one.partition.clone()])
                .unwrap(),
            vec![one.clone()]
        );
        repository.stage_projections(&two, &[projection()]).unwrap();
        publish(&repository, &two.id, true).unwrap();
        assert_eq!(
            repository
                .active_generations(&[two.partition.clone()])
                .unwrap(),
            vec![two]
        );
        repository.database.call(|connection|{
            let cleanup:usize=connection.query_row("SELECT count(*) FROM rag_graph_cleanup_jobs WHERE generation_id='one'",[],|r|r.get(0)).unwrap();assert_eq!(cleanup,1);
            let state:String=connection.query_row("SELECT state FROM rag_graph_generations WHERE generation_id='one'",[],|r|r.get(0)).unwrap();assert_eq!(state,"Retired");
            let fulltext:usize=connection.query_row("SELECT count(*) FROM rag_graph_nodes_fts WHERE text MATCH jieba_query('中文图谱')",[],|r|r.get(0)).unwrap();assert_eq!(fulltext,2);
            let vectors:usize=connection.query_row("SELECT count(*) FROM rag_graph_nodes_vec_d2",[],|r|r.get(0)).unwrap();assert_eq!(vectors,2);
            Ok::<_,String>(())
        },||"worker".into()).unwrap();
    }

    #[test]
    fn node_recall_filters_before_ranking_and_keeps_model_spaces_isolated() {
        let repository = SqliteRagGraphIndexRepository::open(":memory:").unwrap();
        let allowed = manifest("allowed");
        repository.begin_generation(&allowed).unwrap();
        repository
            .stage_projections(&allowed, &[projection()])
            .unwrap();
        let search = |matching| {
            repository
                .search_nodes(GraphNodeSearchQuery {
                    generation_ids: vec!["allowed".into(), "staged".into(), "other-space".into()],
                    matching,
                    limit: 10,
                })
                .unwrap()
        };
        assert!(
            search(GraphNodeMatch::Fulltext("中文图谱".into()))
                .hits
                .is_empty()
        );
        assert!(search(GraphNodeMatch::Browse { excluded_ids: BTreeSet::new() }).hits.is_empty());
        publish(&repository, "allowed", true).unwrap();
        let mut other = manifest("other-space");
        other.logical_source_code = "other-document".into();
        other.model_fingerprint = "different-space".into();
        repository.begin_generation(&other).unwrap();
        let mut p = projection();
        p.entity_id = "other-entity".into();
        p.embedding = vec![0., 1.];
        repository.stage_projections(&other, &[p]).unwrap();
        publish(&repository, &other.id, true).unwrap();
        let staged = manifest("staged");
        repository.begin_generation(&staged).unwrap();
        repository
            .stage_projections(&staged, &[projection()])
            .unwrap();
        let fulltext = search(GraphNodeMatch::Fulltext("中文图谱".into()));
        assert_eq!(fulltext.hits.len(), 2);
        let browsed = search(GraphNodeMatch::Browse { excluded_ids: BTreeSet::new() });
        assert_eq!(browsed.hits.len(), 2);
        assert!(browsed.hits.iter().all(|hit| hit.generation_id != "staged"));
        let browsed = repository
            .search_nodes(GraphNodeSearchQuery {
                generation_ids: vec!["allowed".into(), "other-space".into()],
                matching: GraphNodeMatch::Browse { excluded_ids: BTreeSet::new() },
                limit: 1,
            })
            .unwrap();
        assert_eq!(browsed.hits.len(), 1);
        assert!(browsed.has_more);
        let vector = search(GraphNodeMatch::Vector {
            embedding: vec![1., 0.],
            model_code: "model".into(),
            model_fingerprint: "space-2".into(),
            provider_model: None,
        });
        assert_eq!(vector.hits.len(), 1);
        assert_eq!(vector.hits[0].generation_id, "allowed");
        assert!((vector.hits[0].score - 1.).abs() < 1e-6);
        let restricted = repository
            .search_nodes(GraphNodeSearchQuery {
                generation_ids: vec!["allowed".into()],
                matching: GraphNodeMatch::Fulltext("Alpha".into()),
                limit: 1,
            })
            .unwrap();
        assert_eq!(restricted.hits.len(), 1);
        assert_eq!(restricted.hits[0].chunk_codes, vec!["chunk"]);
        assert!(!restricted.has_more);
        let wrong_provider = search(GraphNodeMatch::Vector {
            embedding: vec![1., 0.],
            model_code: "model".into(),
            model_fingerprint: "space-2".into(),
            provider_model: Some("different".into()),
        });
        assert!(wrong_provider.hits.is_empty());
        let replacement = manifest("replacement");
        repository.begin_generation(&replacement).unwrap();
        repository
            .stage_projections(&replacement, &[projection()])
            .unwrap();
        publish(&repository, &replacement.id, true).unwrap();
        assert!(
            repository
                .search_nodes(GraphNodeSearchQuery {
                    generation_ids: vec!["allowed".into()],
                    matching: GraphNodeMatch::Fulltext("Alpha".into()),
                    limit: 10
                })
                .unwrap()
                .hits
                .is_empty()
        );
    }
    #[test]
    fn snapshots_bind_identity_expire_and_cap_across_agent_identities() {
        let repository = SqliteRagGraphIndexRepository::open(":memory:").unwrap();
        let one = manifest("snapshot-source");
        repository.begin_generation(&one).unwrap();
        repository.stage_projections(&one, &[projection()]).unwrap();
        publish(&repository, &one.id, true).unwrap();
        let now = 1_000_000;
        let draft = GraphSnapshotDraft {
            owner_hash: "owner".into(),
            identity_hash: "agent-a".into(),
            request: serde_json::json!({"query":"How does embedding work?"}),
            traversal: serde_json::json!({"incoming":[],"outgoing":[]}),
            generation_ids: BTreeSet::from([one.id.clone()]),
            expires_at: now + GRAPH_SNAPSHOT_TTL_MS,
        };
        let original = repository.save_snapshot(draft.clone(), now).unwrap();
        assert_eq!(original.cursor.len(), 64);
        assert_eq!(
            repository
                .load_snapshot("agent-a", &original.cursor, now + 1)
                .unwrap()
                .request,
            draft.request
        );
        assert_eq!(
            repository
                .load_snapshot("agent-b", &original.cursor, now + 1)
                .unwrap_err(),
            GraphSnapshotError::Unavailable
        );
        assert_eq!(
            repository
                .load_snapshot("agent-a", &original.cursor, draft.expires_at)
                .unwrap_err(),
            GraphSnapshotError::Unavailable
        );
        let mut invalid = draft.clone();
        invalid.request = serde_json::json!({"nested":{"queryEmbedding":[1.,0.]}});
        assert_eq!(
            repository.save_snapshot(invalid, now).unwrap_err(),
            GraphSnapshotError::Invalid
        );
        for n in 1..=GRAPH_SNAPSHOTS_PER_USER {
            let mut next = draft.clone();
            next.identity_hash = format!("agent-{n}");
            repository.save_snapshot(next, now + n as i64).unwrap();
        }
        assert_eq!(
            repository
                .load_snapshot("agent-a", &original.cursor, now + 100)
                .unwrap_err(),
            GraphSnapshotError::Unavailable
        );
        repository.database.call(|connection|{
            let count:usize=connection.query_row("SELECT count(*) FROM rag_graph_query_snapshots WHERE owner_hash='owner'",[],|r|r.get(0)).unwrap();
            assert_eq!(count,GRAPH_SNAPSHOTS_PER_USER);
            let refs:usize=connection.query_row("SELECT count(*) FROM rag_graph_snapshot_generations",[],|r|r.get(0)).unwrap();assert_eq!(refs,count);Ok::<_,String>(())
        },||"worker".into()).unwrap();
        let current = repository.save_snapshot(draft.clone(), now + 100).unwrap();
        let replacement = manifest("snapshot-replacement");
        repository.begin_generation(&replacement).unwrap();
        repository
            .stage_projections(&replacement, &[projection()])
            .unwrap();
        publish(&repository, &replacement.id, true).unwrap();
        assert_eq!(
            repository
                .load_snapshot("agent-a", &current.cursor, now + 101)
                .unwrap_err(),
            GraphSnapshotError::PublicationChanged
        );
        assert_eq!(
            repository.save_snapshot(draft, now + 101).unwrap_err(),
            GraphSnapshotError::PublicationChanged
        );
    }
}
