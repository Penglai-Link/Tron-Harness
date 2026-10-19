//! Leased cleanup is fenced in SQL before any graph mutation, and can be replayed after a crash.
use super::*;
const LIVE_TASK: &str = "(EXISTS(SELECT 1 FROM rag_record_tasks r WHERE r.id=g.task_id AND r.state IN ('QUEUED','RUNNING')) OR EXISTS(SELECT 1 FROM knowledge_base_ingestion_tasks k WHERE k.id=g.task_id AND k.state IN ('QUEUED','RUNNING')))";

pub(super) fn claim(
    connection: &Connection,
    now: i64,
    lease_ms: i64,
) -> Result<Option<GraphCleanupClaim>, String> {
    if now < 0 || !(1000..=600_000).contains(&lease_ms) {
        return Err(err("invalid cleanup lease"));
    }
    let tx =
        business_transaction(connection, rusqlite::TransactionBehavior::Immediate).map_err(err)?;
    let id:Option<String>=tx.query_row(&format!(
        "SELECT j.generation_id FROM rag_graph_cleanup_jobs j JOIN rag_graph_generations g ON g.generation_id=j.generation_id          WHERE j.not_before<=?1 AND j.lease_until<=?1 AND g.state<>'Active'          AND NOT EXISTS(SELECT 1 FROM rag_graph_active_sources a WHERE a.generation_id=g.generation_id)          AND (g.state='Cleaning' OR NOT {LIVE_TASK}) ORDER BY j.not_before,j.generation_id LIMIT 1"),[now],|r|r.get(0)).optional().map_err(err)?;
    let Some(id) = id else {
        return Ok(None);
    };
    let manifest = stored_manifest(&tx, &id)?.ok_or_else(|| err("cleanup manifest is absent"))?;
    let token = rand::random::<[u8; 32]>()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    tx.execute("UPDATE rag_graph_cleanup_jobs SET lease_token=?2,lease_until=?3,attempts=attempts+1,updated_at=?4 WHERE generation_id=?1",
        params![id,token,now.saturating_add(lease_ms),now]).map_err(err)?;
    tx.execute(
        "UPDATE rag_graph_generations SET state='Cleaning',updated_at=?2 WHERE generation_id=?1",
        params![id, now],
    )
    .map_err(err)?;
    let attempts = tx
        .query_row(
            "SELECT attempts FROM rag_graph_cleanup_jobs WHERE generation_id=?1",
            [&id],
            |r| r.get(0),
        )
        .map_err(err)?;
    tx.commit().map_err(err)?;
    Ok(Some(GraphCleanupClaim {
        manifest,
        lease_token: token,
        attempts,
    }))
}

pub(super) fn finish(
    connection: &Connection,
    claim: &GraphCleanupClaim,
    now: i64,
    failure: Option<&str>,
) -> Result<(), String> {
    let tx =
        business_transaction(connection, rusqlite::TransactionBehavior::Immediate).map_err(err)?;
    let id = &claim.manifest.id;
    let owned:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM rag_graph_cleanup_jobs j JOIN rag_graph_generations g ON g.generation_id=j.generation_id         WHERE j.generation_id=?1 AND j.lease_token=?2 AND j.lease_until>?3 AND g.state='Cleaning'         AND NOT EXISTS(SELECT 1 FROM rag_graph_active_sources a WHERE a.generation_id=j.generation_id))",
        params![id,claim.lease_token,now],|r|r.get(0)).map_err(err)?;
    if !owned || stored_manifest(&tx, id)?.as_ref() != Some(&claim.manifest) {
        return Err(err("cleanup lease is stale"));
    }
    if let Some(failure) = failure {
        let delay = 1000_i64
            .saturating_mul(1_i64 << claim.attempts.min(8))
            .min(300_000);
        let message = failure.chars().take(512).collect::<String>();
        tx.execute("UPDATE rag_graph_cleanup_jobs SET lease_token=NULL,lease_until=0,not_before=?2,last_error=?3,updated_at=?4 WHERE generation_id=?1",
            params![id,now.saturating_add(delay),message,now]).map_err(err)?;
    } else {
        let table = vector_table(claim.manifest.dimension)?;
        let has_vectors: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name=?1)",
                [&table],
                |r| r.get(0),
            )
            .map_err(err)?;
        if has_vectors {
            tx.execute(&format!("DELETE FROM {table} WHERE rowid IN (SELECT id FROM rag_graph_node_projections WHERE generation_id=?1)"),[id]).map_err(err)?;
        }
        tx.execute("DELETE FROM rag_graph_nodes_fts WHERE rowid IN (SELECT id FROM rag_graph_node_projections WHERE generation_id=?1)",[id]).map_err(err)?;
        tx.execute(
            "DELETE FROM rag_graph_node_projections WHERE generation_id=?1",
            [id],
        )
        .map_err(err)?;
        // Cursors that referenced a retired generation must request a new snapshot.
        tx.execute("DELETE FROM rag_graph_query_snapshots WHERE snapshot_id IN (SELECT snapshot_id FROM rag_graph_snapshot_generations WHERE generation_id=?1)",[id]).map_err(err)?;
        tx.execute(
            "DELETE FROM rag_graph_cleanup_jobs WHERE generation_id=?1",
            [id],
        )
        .map_err(err)?;
        tx.execute(
            "DELETE FROM rag_graph_generations WHERE generation_id=?1",
            [id],
        )
        .map_err(err)?;
    }
    tx.commit().map_err(err)
}

pub(super) fn sweep(
    connection: &Connection,
    now: i64,
    limit: usize,
) -> Result<GraphHousekeepingResult, String> {
    if now < 0 || !(1..=256).contains(&limit) {
        return Err(err("invalid graph housekeeping budget"));
    }
    let tx =
        business_transaction(connection, rusqlite::TransactionBehavior::Immediate).map_err(err)?;
    let expired_snapshots=tx.execute("DELETE FROM rag_graph_query_snapshots WHERE snapshot_id IN (SELECT snapshot_id FROM rag_graph_query_snapshots WHERE expires_at<=?1 ORDER BY expires_at LIMIT ?2)",params![now,limit]).map_err(err)?;
    // Failed/cancelled attempts keep paid computation for retries. Only old,
    // completed or ownerless checkpoints with no live/active generation are GC'd.
    let removed_checkpoints=tx.execute("DELETE FROM rag_graph_task_checkpoints WHERE rowid IN (SELECT c.rowid FROM rag_graph_task_checkpoints c         WHERE c.updated_at<?1 AND NOT EXISTS(SELECT 1 FROM rag_graph_generations g WHERE g.task_id=c.task_id AND g.state IN ('Staging','Ready','Active','Cleaning'))         AND NOT EXISTS(SELECT 1 FROM rag_record_tasks r WHERE r.id=c.task_id AND r.state<>'COMPLETED')         AND NOT EXISTS(SELECT 1 FROM knowledge_base_ingestion_tasks k WHERE k.id=c.task_id AND k.state NOT IN ('COMPLETED','READY'))         ORDER BY c.updated_at,c.rowid LIMIT ?2)",params![now.saturating_sub(86_400_000),limit]).map_err(err)?;
    tx.commit().map_err(err)?;
    Ok(GraphHousekeepingResult {
        expired_snapshots,
        removed_checkpoints,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn manifest(id: &str) -> GraphGenerationManifest {
        GraphGenerationManifest {
            id: id.into(),
            partition: GraphPartition {
                corpus_id: "skill".into(),
                scope_type: "USER".into(),
                scope_code: "owner".into(),
            },
            logical_source_code: id.into(),
            source_code: id.into(),
            source_version: "v1".into(),
            task_id: id.into(),
            configuration_hash: "config".into(),
            model_code: "model".into(),
            model_fingerprint: "space".into(),
            provider_model: None,
            dimension: 2,
            expected_projections: 1,
        }
    }
    fn stage(repo: &SqliteRagGraphIndexRepository, id: &str) -> GraphGenerationManifest {
        let m = manifest(id);
        repo.begin_generation(&m).unwrap();
        repo.stage_projections(
            &m,
            &[PreparedGraphProjection {
                entity_id: "entity".into(),
                chunk_codes: vec!["chunk".into()],
                text: "source evidence".into(),
                embedding: vec![1., 0.],
            }],
        )
        .unwrap();
        m
    }
    fn sql(repo: &SqliteRagGraphIndexRepository, query: &str) {
        let query = query.to_owned();
        repo.database
            .call(
                move |c| c.execute_batch(&query).map_err(err),
                || err("worker"),
            )
            .unwrap();
    }
    fn due() -> i64 {
        chrono::Utc::now().timestamp_millis() + 300_001
    }
    #[test]
    fn active_and_running_sources_are_protected_and_cleaning_fences_republication() {
        let repo = SqliteRagGraphIndexRepository::open(":memory:").unwrap();
        let active = stage(&repo, "active");
        let running = stage(&repo, "running");
        sql(
            &repo,
            "UPDATE rag_graph_generations SET state='Active' WHERE generation_id='active'; INSERT INTO rag_record_tasks(id,user_code,corpus_id,state,payload_json,updated_at) VALUES ('running','owner','skill','RUNNING','{}',0)",
        );
        assert!(repo.claim_cleanup(due(), 1000).unwrap().is_none());
        sql(
            &repo,
            "UPDATE rag_record_tasks SET state='FAILED' WHERE id='running'",
        );
        let claim = repo.claim_cleanup(due(), 1000).unwrap().unwrap();
        assert_eq!(claim.manifest, running);
        assert!(repo.begin_generation(&running).is_err());
        assert!(
            repo.stage_projections(
                &running,
                &[PreparedGraphProjection {
                    entity_id: "entity".into(),
                    chunk_codes: vec!["chunk".into()],
                    text: "source evidence".into(),
                    embedding: vec![1., 0.]
                }]
            )
            .is_err()
        );
        let id = claim.manifest.id.clone();
        assert!(
            repo.database
                .call(
                    move |c| {
                        let tx = business_transaction(c, rusqlite::TransactionBehavior::Immediate)
                            .map_err(err)?;
                        super::super::activate_graph_generation(&tx, &id)
                    },
                    || err("worker")
                )
                .is_err()
        );
        assert!(
            repo.active_generations(&[active.partition])
                .unwrap()
                .is_empty(),
            "a corrupt Active row without a publication pointer is never traversed"
        );
    }
    #[test]
    fn restart_reclaims_expired_lease_and_removes_native_projection_rows_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("graph.sqlite").display().to_string();
        let repo = SqliteRagGraphIndexRepository::open(&path).unwrap();
        let m = stage(&repo, "old");
        repo.save_checkpoint(
            "old",
            "paid",
            "prefix",
            &serde_json::json!({"complete":true}),
        )
        .unwrap();
        let now = due();
        let first = repo.claim_cleanup(now, 1000).unwrap().unwrap();
        let reopened = SqliteRagGraphIndexRepository::open(&path).unwrap();
        assert!(reopened.claim_cleanup(now, 1000).unwrap().is_none());
        let second = reopened.claim_cleanup(now + 1001, 1000).unwrap().unwrap();
        assert_ne!(first.lease_token, second.lease_token);
        assert!(repo.finish_cleanup(&first, now + 1002, None).is_err());
        reopened.finish_cleanup(&second, now + 1002, None).unwrap();
        assert!(reopened.export_generation(&m.id).unwrap().is_none());
        reopened
            .database
            .call(
                |c| {
                    for table in [
                        "rag_graph_nodes_fts",
                        "rag_graph_nodes_vec_d2",
                        "rag_graph_node_projections",
                        "rag_graph_generations",
                        "rag_graph_cleanup_jobs",
                    ] {
                        let count: usize = c
                            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                            .map_err(err)?;
                        assert_eq!(count, 0, "{table}");
                    }
                    Ok::<_, String>(())
                },
                || err("worker"),
            )
            .unwrap();
        assert!(
            reopened
                .load_checkpoint("old", "paid", "prefix", false)
                .unwrap()
                .is_some(),
            "failed/resumable computation survives graph collection"
        );
        reopened.begin_generation(&m).unwrap();
    }
    #[test]
    fn failed_cleanup_has_persistent_backoff_and_stale_claim_cannot_reset_it() {
        let repo = SqliteRagGraphIndexRepository::open(":memory:").unwrap();
        stage(&repo, "old");
        let now = due();
        let first = repo.claim_cleanup(now, 1000).unwrap().unwrap();
        repo.finish_cleanup(&first, now, Some("storage unavailable"))
            .unwrap();
        assert!(repo.claim_cleanup(now + 1999, 1000).unwrap().is_none());
        let second = repo.claim_cleanup(now + 2000, 1000).unwrap().unwrap();
        assert_eq!(second.attempts, 2);
        assert!(
            repo.finish_cleanup(&first, now + 2001, Some("late error"))
                .is_err()
        );
        repo.finish_cleanup(&second, now + 2001, None).unwrap();
    }
    #[test]
    fn housekeeping_is_bounded_and_preserves_live_published_and_retryable_checkpoints() {
        let repo = SqliteRagGraphIndexRepository::open(":memory:").unwrap();
        for task in [
            "orphan-a",
            "orphan-b",
            "completed",
            "failed",
            "cancelled",
            "running",
            "published",
        ] {
            repo.save_checkpoint(task, "scope", "key", &serde_json::json!({"paid":true}))
                .unwrap();
        }
        sql(
            &repo,
            "UPDATE rag_graph_task_checkpoints SET updated_at=0; INSERT INTO rag_record_tasks(id,user_code,corpus_id,state,payload_json,updated_at) VALUES ('completed','u','skill','COMPLETED','{}',0),('failed','u','skill','FAILED','{}',0),('cancelled','u','skill','CANCELLED','{}',0),('running','u','skill','RUNNING','{}',0)",
        );
        stage(&repo, "published");
        sql(
            &repo,
            "UPDATE rag_graph_generations SET state='Active' WHERE generation_id='published'",
        );
        let report = repo.sweep_computation(86_400_001, 2).unwrap();
        assert_eq!(report.removed_checkpoints, 2);
        assert_eq!(
            repo.sweep_computation(86_400_001, 2)
                .unwrap()
                .removed_checkpoints,
            1
        );
        for task in ["failed", "cancelled", "running", "published"] {
            assert!(
                repo.load_checkpoint(task, "scope", "key", false)
                    .unwrap()
                    .is_some(),
                "{task}"
            );
        }
    }
}
