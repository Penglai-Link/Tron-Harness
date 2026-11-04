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
