//! Atomic append/adoption of immutable, compact node-vector batches.
use super::*;

fn manifest(raw: &str, task: &str) -> Result<GraphNodeVectorManifest, String> {
    let value: GraphNodeVectorManifest = serde_json::from_str(raw)
        .map_err(|_| err("node-vector checkpoint requires verified format conversion"))?;
    value.validate()?;
    if value.storage_task_id != task {
        return Err(err("node-vector checkpoint owner differs"));
    }
    Ok(value)
}
fn validate_key(key: &str) -> Result<(), String> {
    if !key.starts_with("NodeVectors_") || key.contains(':') || key.len() > 256 {
        return Err(err("invalid node-vector checkpoint key"));
    }
    Ok(())
}
fn prefix(key: &str) -> String {
    format!("{key}:Batch:")
}

pub(super) fn load(
    connection: &mut Connection,
    task: &str,
    scope: &str,
    key: &str,
) -> Result<Option<GraphNodeVectorCheckpoint>, String> {
    validate_key(key)?;
    let tx =
        business_transaction(connection, rusqlite::TransactionBehavior::Immediate).map_err(err)?;
    let row: Option<(String, String)> = tx.query_row(
        "SELECT c.task_id,c.payload_json FROM rag_graph_task_checkpoints c \
         WHERE c.cache_scope=?2 AND c.checkpoint_key=?3 AND \
         (c.task_id=?1 OR EXISTS(SELECT 1 FROM rag_graph_active_sources a JOIN rag_graph_generations g \
         ON g.generation_id=a.generation_id WHERE g.task_id=c.task_id AND g.state='Active' AND g.vector_cache_invalidated=0)) \
         ORDER BY (c.task_id=?1) DESC,c.updated_at DESC LIMIT 1",
        params![task,scope,key], |r| Ok((r.get(0)?,r.get(1)?)),
    ).optional().map_err(err)?;
    let Some((owner, raw)) = row else {
        return Ok(None);
    };
    let mut saved = manifest(&raw, &owner)?;
    let lower = prefix(key);
    let upper = format!("{lower}\u{10ffff}");
    let mut batches = Vec::new();
    {
        let mut statement = tx
            .prepare(
                "SELECT checkpoint_key,payload_json FROM rag_graph_task_checkpoints \
             WHERE task_id=?1 AND cache_scope=?2 AND checkpoint_key>=?3 AND checkpoint_key<?4 \
             ORDER BY checkpoint_key",
            )
            .map_err(err)?;
        let mut rows = statement
            .query(params![owner, scope, lower, upper])
            .map_err(err)?;
        let mut offset = 0usize;
        while let Some(row) = rows.next().map_err(err)? {
            let batch_key: String = row.get(0).map_err(err)?;
            if batches.len() >= saved.batch_count
                || batch_key != graph_node_vector_batch_key(key, batches.len())
            {
                return Err(err("node-vector batch ordinal differs"));
            }
            let raw: String = row.get(1).map_err(err)?;
            let batch: GraphNodeVectorBatch = serde_json::from_str(&raw).map_err(err)?;
            if batch.count == 0 || batch.offset != offset {
                return Err(err("node-vector batches are not contiguous"));
            }
            offset = offset
                .checked_add(batch.count)
                .ok_or("node-vector checkpoint too large")?;
            if offset > saved.completed_units {
                return Err(err("node-vector batch exceeds manifest"));
            }
            batches.push(batch);
        }
        if batches.len() != saved.batch_count || offset != saved.completed_units {
            return Err(err("node-vector checkpoint is incomplete"));
        }
    }
    if owner != task {
        // Copy exact encoded bytes, without deserializing historical floats.
        // The new task owns its prefix before the old generation can be retired.
        let now = chrono::Utc::now().timestamp_millis();
        tx.execute(
            "INSERT INTO rag_graph_task_checkpoints(task_id,cache_scope,checkpoint_key,payload_json,updated_at) \
             SELECT ?1,cache_scope,checkpoint_key,payload_json,?6 FROM rag_graph_task_checkpoints \
             WHERE task_id=?2 AND cache_scope=?3 AND checkpoint_key>=?4 AND checkpoint_key<?5",
            params![task,owner,scope,lower,upper,now],
        ).map_err(err)?;
        saved.storage_task_id = task.into();
        tx.execute(
            "INSERT INTO rag_graph_task_checkpoints(task_id,cache_scope,checkpoint_key,payload_json,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![task,scope,key,serde_json::to_string(&saved).map_err(err)?,now],
        ).map_err(err)?;
    }
    tx.commit().map_err(err)?;
    Ok(Some(GraphNodeVectorCheckpoint {
        manifest: saved,
        batches,
    }))
}

pub(super) fn save(
    connection: &mut Connection,
    task: &str,
    scope: &str,
    key: &str,
    expected: usize,
    next: &GraphNodeVectorManifest,
    batch: Option<&GraphNodeVectorBatch>,
) -> Result<(), String> {
    validate_key(key)?;
    next.validate()?;
    if next.storage_task_id != task {
        return Err(err("node-vector checkpoint owner differs"));
    }
    if let Some(batch) = batch {
        // Bounded to one provider batch; never decode or rewrite the historical prefix.
        batch.decode(next.dimension)?;
        if batch.offset != expected
            || batch.offset.checked_add(batch.count) != Some(next.completed_units)
        {
            return Err(err("node-vector batch offset differs"));
        }
    }
    let tx =
        business_transaction(connection, rusqlite::TransactionBehavior::Immediate).map_err(err)?;
    let raw: Option<String> = tx.query_row(
        "SELECT payload_json FROM rag_graph_task_checkpoints WHERE task_id=?1 AND cache_scope=?2 AND checkpoint_key=?3",
        params![task,scope,key], |r| r.get(0),
    ).optional().map_err(err)?;
    let previous = raw.as_deref().map(|s| manifest(s, task)).transpose()?;
    if let Some(old) = &previous {
        if old == next {
            if let Some(batch) = batch {
                let stored: Option<String> = tx.query_row(
                    "SELECT payload_json FROM rag_graph_task_checkpoints WHERE task_id=?1 AND cache_scope=?2 AND checkpoint_key=?3",
                    params![task,scope,graph_node_vector_batch_key(key,next.batch_count-1)], |r| r.get(0),
                ).optional().map_err(err)?;
                if stored
                    .as_deref()
                    .map(serde_json::from_str::<GraphNodeVectorBatch>)
                    .transpose()
                    .map_err(err)?
                    .as_ref()
                    != Some(batch)
                {
                    return Err(err("node-vector checkpoint replay differs"));
                }
            } else if expected != old.completed_units {
                return Err(err("node-vector manifest offset differs"));
            }
            return Ok(());
        }
        if old.model_code != next.model_code
            || old.fingerprint != next.fingerprint
            || old.dimension != next.dimension
            || (old.completed_units > 0 && old.provider_model != next.provider_model)
        {
            return Err(err("node-vector model space differs"));
        }
    }
    let old_completed = previous.as_ref().map_or(0, |m| m.completed_units);
    let old_batches = previous.as_ref().map_or(0, |m| m.batch_count);
    if old_completed != expected
        || next.batch_count != old_batches + usize::from(batch.is_some())
        || (batch.is_none() && next.completed_units != old_completed)
    {
        return Err(err("node-vector checkpoint prefix changed"));
    }
    let now = chrono::Utc::now().timestamp_millis();
    if let Some(batch) = batch {
        tx.execute(
            "INSERT INTO rag_graph_task_checkpoints(task_id,cache_scope,checkpoint_key,payload_json,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![task,scope,graph_node_vector_batch_key(key,old_batches),serde_json::to_string(batch).map_err(err)?,now],
        ).map_err(err)?;
    }
    tx.execute(
        "INSERT INTO rag_graph_task_checkpoints(task_id,cache_scope,checkpoint_key,payload_json,updated_at) \
         VALUES (?1,?2,?3,?4,?5) ON CONFLICT(task_id,cache_scope,checkpoint_key) \
         DO UPDATE SET payload_json=excluded.payload_json,updated_at=excluded.updated_at",
        params![task,scope,key,serde_json::to_string(next).map_err(err)?,now],
    ).map_err(err)?;
    tx.commit().map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    const SCOPE: &str = "source-and-model-space";
    const KEY: &str = "NodeVectors_input-hash";
    fn initial(task: &str) -> GraphNodeVectorManifest {
        GraphNodeVectorManifest {
            format_version: 1,
            storage_task_id: task.into(),
            model_code: "model".into(),
            fingerprint: "fingerprint".into(),
            provider_model: Some("provider".into()),
            dimension: 2,
            completed_units: 0,
            batch_count: 0,
        }
    }
    fn append(
        repo: &SqliteRagGraphIndexRepository,
        state: &mut GraphNodeVectorManifest,
        values: &[Vec<f32>],
    ) {
        let old = state.completed_units;
        let batch = GraphNodeVectorBatch::encode(old, values, 2).unwrap();
        state.completed_units += values.len();
        state.batch_count += 1;
        repo.save_node_vector_checkpoint(
            &state.storage_task_id,
            SCOPE,
            KEY,
            old,
            state,
            Some(&batch),
        )
        .unwrap();
    }
    fn execute(repo: &SqliteRagGraphIndexRepository, sql: &str) {
        let sql = sql.to_owned();
        repo.database
            .call(
                move |c| c.execute_batch(&sql).map_err(err),
                || err("worker"),
            )
            .unwrap();
    }
    fn vectors(repo: &SqliteRagGraphIndexRepository, task: &str, count: usize) -> Vec<Vec<f32>> {
        repo.load_node_vector_checkpoint(task, SCOPE, KEY)
            .unwrap()
            .unwrap()
            .restore("model", "fingerprint", 2, count)
            .unwrap()
            .0
            .vectors
    }
    #[test]
    fn node_vector_batches_survive_reopen_and_never_rewrite_paid_prefixes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vectors.sqlite3");
        let repo = SqliteRagGraphIndexRepository::open(path.to_str().unwrap()).unwrap();
        execute(
            &repo,
            "CREATE TRIGGER immutable_vector_batch BEFORE UPDATE ON rag_graph_task_checkpoints \
            WHEN instr(OLD.checkpoint_key,':Batch:')>0 BEGIN SELECT RAISE(ABORT,'paid batch rewritten'); END",
        );
        let mut state = initial("task");
        let expected = (0..40)
            .map(|i| vec![i as f32 + 1., -0.])
            .collect::<Vec<_>>();
        for batch in expected.chunks(2) {
            append(&repo, &mut state, batch);
        }
        repo.save_node_vector_checkpoint("task", SCOPE, KEY, 40, &state, None)
            .unwrap();
        assert_eq!(vectors(&repo, "task", 40), expected);
        drop(repo);
        let reopened = SqliteRagGraphIndexRepository::open(path.to_str().unwrap()).unwrap();
        assert_eq!(vectors(&reopened, "task", 40), expected);
        let manifest = reopened
            .load_checkpoint("task", SCOPE, KEY, false)
            .unwrap()
            .unwrap();
        assert!(manifest.get("vectors").is_none());
        assert!(serde_json::to_string(&manifest).unwrap().len() < 512);
    }
    #[test]
    fn node_vector_batch_and_manifest_roll_back_together_and_stale_offsets_fail() {
        let repo = SqliteRagGraphIndexRepository::open(":memory:").unwrap();
        let mut state = initial("task");
        append(&repo, &mut state, &[vec![1., 0.]]);
        execute(
            &repo,
            "CREATE TRIGGER fail_vector_manifest BEFORE UPDATE ON rag_graph_task_checkpoints \
            WHEN NEW.checkpoint_key='NodeVectors_input-hash' BEGIN SELECT RAISE(ABORT,'disk failure'); END",
        );
        let batch = GraphNodeVectorBatch::encode(1, &[vec![0., 1.]], 2).unwrap();
        let mut next = state.clone();
        next.completed_units = 2;
        next.batch_count = 2;
        assert!(
            repo.save_node_vector_checkpoint("task", SCOPE, KEY, 1, &next, Some(&batch))
                .is_err()
        );
        assert!(
            repo.load_checkpoint("task", SCOPE, &graph_node_vector_batch_key(KEY, 1), false)
                .unwrap()
                .is_none()
        );
        assert_eq!(vectors(&repo, "task", 1), vec![vec![1., 0.]]);
        execute(&repo, "DROP TRIGGER fail_vector_manifest");
        repo.save_node_vector_checkpoint("task", SCOPE, KEY, 1, &next, Some(&batch))
            .unwrap();
        repo.save_node_vector_checkpoint("task", SCOPE, KEY, 1, &next, Some(&batch))
            .unwrap();
        assert_eq!(vectors(&repo, "task", 2), vec![vec![1., 0.], vec![0., 1.]]);
        let bad = GraphNodeVectorBatch::encode(0, &[vec![9., 9.]], 2).unwrap();
        assert!(
            repo.save_node_vector_checkpoint("task", SCOPE, KEY, 0, &state, Some(&bad))
                .is_err()
        );
        assert_eq!(vectors(&repo, "task", 2), vec![vec![1., 0.], vec![0., 1.]]);
    }
    #[test]
    fn node_vector_published_adoption_owns_batches_and_respects_invalidation_and_scope() {
        let repo = SqliteRagGraphIndexRepository::open(":memory:").unwrap();
        let mut state = initial("published");
        append(&repo, &mut state, &[vec![1., 0.], vec![0., 1.]]);
        let partition = GraphPartition {
            corpus_id: "corpus".into(),
            scope_type: "USER".into(),
            scope_code: "owner".into(),
        };
        repo.begin_generation(&GraphGenerationManifest {
            id: "generation".into(),
            partition: partition.clone(),
            logical_source_code: "source".into(),
            source_code: "versioned-source".into(),
            source_version: "v1".into(),
            task_id: "published".into(),
            configuration_hash: "config".into(),
            model_code: "model".into(),
            model_fingerprint: "fingerprint".into(),
            provider_model: Some("provider".into()),
            dimension: 2,
            expected_projections: 2,
        })
        .unwrap();
        let key = partition.key();
        repo.database.call(move|c| {
            c.execute("UPDATE rag_graph_generations SET state='Active' WHERE generation_id='generation'",[]).map_err(err)?;
            c.execute("INSERT INTO rag_graph_active_sources(partition_key,logical_source_code,generation_id,publication_epoch,updated_at) VALUES (?1,'source','generation',1,0)",[key]).map_err(err)?;Ok(())
        },||err("worker")).unwrap();
        assert!(
            repo.load_node_vector_checkpoint("next", "another-scope", KEY)
                .unwrap()
                .is_none()
        );
        let adopted = repo
            .load_node_vector_checkpoint("next", SCOPE, KEY)
            .unwrap()
            .unwrap();
        assert_eq!(adopted.manifest.storage_task_id, "next");
        assert_eq!(
            adopted
                .restore("model", "fingerprint", 2, 2)
                .unwrap()
                .0
                .vectors,
            vec![vec![1., 0.], vec![0., 1.]]
        );
        execute(
            &repo,
            "UPDATE rag_graph_generations SET vector_cache_invalidated=1",
        );
        assert!(
            repo.load_node_vector_checkpoint("blocked", SCOPE, KEY)
                .unwrap()
                .is_none()
        );
        execute(
            &repo,
            "DELETE FROM rag_graph_task_checkpoints WHERE task_id='published'",
        );
        assert_eq!(vectors(&repo, "next", 2), vec![vec![1., 0.], vec![0., 1.]]);
    }
}
