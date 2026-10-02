use rusqlite::{Connection, Transaction, TransactionBehavior};
use serde_json::json;

use crate::{
    domain::short_term_memory::{ShortTermMemoryError, ShortTermMemoryEvent},
    shared::time::{current_timestamp, timestamp_to_millis},
};

use super::super::{
    short_term_memory_repository_append::assign_event_storage_fields,
    short_term_memory_rows::{insert_event, storage_error},
};

pub(super) fn active_run_exists(
    connection: &Connection,
    session_code: &str,
) -> Result<bool, ShortTermMemoryError> {
    let count = connection
        .query_row(
            "SELECT count(1)
             FROM agent_session_run_leases
             WHERE session_code = ?1",
            [session_code],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    Ok(count > 0)
}

pub(super) fn active_run_exists_in_transaction(
    tx: &Transaction<'_>,
    session_code: &str,
) -> Result<bool, ShortTermMemoryError> {
    let count = tx
        .query_row(
            "SELECT count(1)
             FROM agent_session_run_leases
             WHERE session_code = ?1",
            [session_code],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    Ok(count > 0)
}

pub(super) fn create_event_and_update_run_lease(
    connection: &mut Connection,
    event: &ShortTermMemoryEvent,
) -> Result<(), ShortTermMemoryError> {
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let mut event = event.clone();
    assign_event_storage_fields(&tx, &mut event)?;
    match event.event_type.as_str() {
        "agent.run.started" => insert_run_lease(&tx, &event)?,
        "agent.run.completed"
        | "agent.run.awaiting_input"
        | "agent.run.failed"
        | "agent.run.recovered" => delete_run_lease(&tx, &event)?,
        _ => {}
    }
    insert_event(&tx, &event)?;
    tx.commit().map_err(storage_error)
}

fn insert_run_lease(
    tx: &Transaction<'_>,
    event: &ShortTermMemoryEvent,
) -> Result<(), ShortTermMemoryError> {
    let Some(run_code) = event.run_code.as_deref() else {
        return Ok(());
    };
    let next_id = tx
        .query_row(
            "SELECT COALESCE(MAX(id), 735910000000000000) + 1
             FROM agent_session_run_leases",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    tx.execute(
        "INSERT INTO agent_session_run_leases
         (id, session_code, workspace_code, run_code, input_code, started_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        rusqlite::params![
            next_id,
            event.session_code,
            event.workspace_code,
            run_code,
            event.input_code.as_deref(),
            timestamp_to_millis(&event.created_at),
        ],
    )
    .map_err(run_lease_write_error)?;
    Ok(())
}

fn delete_run_lease(
    tx: &Transaction<'_>,
    event: &ShortTermMemoryEvent,
) -> Result<(), ShortTermMemoryError> {
    let Some(run_code) = event.run_code.as_deref() else {
        return Ok(());
    };
    tx.execute(
        "DELETE FROM agent_session_run_leases
         WHERE session_code = ?1
           AND run_code = ?2",
        rusqlite::params![event.session_code.as_str(), run_code],
    )
    .map_err(storage_error)?;
    Ok(())
}

pub(super) fn recover_stale_runs(
    connection: &mut Connection,
    limit: usize,
) -> Result<usize, ShortTermMemoryError> {
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let runs = {
        let mut statement = tx
            .prepare(
                "SELECT started.session_code, started.workspace_code, started.run_code, started.input_code
                 FROM agent_transcript_events started
                 WHERE started.event_type = 'agent.run.started'
                   AND started.run_code IS NOT NULL
                   AND NOT EXISTS (
                       SELECT 1
                       FROM agent_transcript_events ended
                       WHERE ended.session_code = started.session_code
                         AND ended.run_code = started.run_code
                         AND ended.event_type IN ('agent.run.completed', 'agent.run.awaiting_input', 'agent.run.failed', 'agent.run.recovered')
                   )
                 ORDER BY started.created_at ASC, started.id ASC
                 LIMIT ?1",
            )
            .map_err(storage_error)?;
        statement
            .query_map([limit.max(1) as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?
    };
    let mut recovered = 0usize;
    for (session_code, workspace_code, run_code, input_code) in runs {
        let now = current_timestamp();
        let mut event = ShortTermMemoryEvent {
            id: 0,
            event_code: String::new(),
            session_code,
            workspace_code,
            seq: 0,
            run_code: Some(run_code.clone()),
            input_code,
            message_code: None,
            part_code: None,
            event_type: "agent.run.recovered".into(),
            event_source: "RECOVERY".into(),
            payload_json: json!({
                "runCode": run_code,
                "reason": "startup_recovered_stale_run",
            }),
            metadata_json: None,
            occurred_at: now.clone(),
            created_at: now,
        };
        assign_event_storage_fields(&tx, &mut event)?;
        delete_run_lease(&tx, &event)?;
        insert_event(&tx, &event)?;
        recovered += 1;
    }
    tx.commit().map_err(storage_error)?;
    Ok(recovered)
}

pub(super) fn cleanup_event_log_before(
    connection: &mut Connection,
    cutoff_created_at_ms: i64,
    batch_size: usize,
    max_rows: usize,
) -> Result<usize, ShortTermMemoryError> {
    let mut deleted_total = 0usize;
    let batch_size = batch_size.max(1);
    let max_rows = max_rows.max(1);
    while deleted_total < max_rows {
        let limit = batch_size.min(max_rows - deleted_total);
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let deleted = tx
            .execute(
                "DELETE FROM agent_transcript_events
                 WHERE id IN (
                     SELECT events.id
                     FROM agent_transcript_events events
                     LEFT JOIN agent_session_run_leases leases
                       ON leases.session_code = events.session_code
                      AND leases.run_code = events.run_code
                     WHERE events.created_at < ?1
                       AND leases.run_code IS NULL
                     ORDER BY events.created_at ASC, events.id ASC
                     LIMIT ?2
                 )",
                rusqlite::params![cutoff_created_at_ms, limit as i64],
            )
            .map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        if deleted == 0 {
            break;
        }
        deleted_total += deleted;
    }
    Ok(deleted_total)
}

fn run_lease_write_error(error: rusqlite::Error) -> ShortTermMemoryError {
    let message = error.to_string();
    if message.contains("agent_session_run_leases")
        || message.contains("idx_agent_session_run_leases_session")
        || message.contains("idx_agent_session_run_leases_run")
    {
        eprintln!("short-term memory sqlite run conflict: {message}");
        return ShortTermMemoryError::RunConflict;
    }
    storage_error(error)
}
