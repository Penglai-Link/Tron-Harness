use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    domain::short_term_memory::{ShortTermMemoryError, ShortTermMemoryFamilySnapshot},
    shared::time::timestamp_to_millis,
};

use super::super::short_term_memory_rows::{
    insert_family_snapshot, option_timestamp_to_millis, row_to_family_snapshot, storage_error,
};

pub(super) fn list_family_session_codes(
    connection: &Connection,
    family_code: &str,
    exclude_session_code: &str,
    limit: usize,
) -> Result<Vec<String>, ShortTermMemoryError> {
    let mut statement = connection
        .prepare(
            "SELECT session_code
             FROM agent_session_family_members
             WHERE family_code = ?1
               AND session_code != ?2
               AND model_context_enabled = 1
             ORDER BY last_activity_at DESC, id DESC
             LIMIT ?3",
        )
        .map_err(storage_error)?;
    statement
        .query_map(
            params![family_code, exclude_session_code, limit as i64],
            |row| row.get(0),
        )
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)
}

pub(super) fn list_completed_family_snapshots(
    connection: &Connection,
    family_code: &str,
    exclude_session_code: &str,
    limit: usize,
) -> Result<Vec<ShortTermMemoryFamilySnapshot>, ShortTermMemoryError> {
    let mut statement = connection
        .prepare(
            "SELECT id, snapshot_code, family_code, session_code, run_code, snapshot_kind, status, source_start_seq, source_end_seq, source_checkpoint_message_code, summary_markdown, token_estimate, error_code, error_message, metadata_json, created_at, completed_at, expires_at
             FROM agent_session_family_snapshots
             WHERE family_code = ?1
               AND session_code != ?2
               AND status = 'COMPLETED'
               AND (expires_at IS NULL OR expires_at > CAST(strftime('%s','now') AS INTEGER) * 1000)
             ORDER BY created_at DESC, id DESC
             LIMIT ?3",
        )
        .map_err(storage_error)?;
    statement
        .query_map(
            params![family_code, exclude_session_code, limit as i64],
            row_to_family_snapshot,
        )
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)
}

pub(super) fn session_summary_for_run(
    connection: &Connection,
    session_code: &str,
    run_code: &str,
) -> Result<Option<ShortTermMemoryFamilySnapshot>, ShortTermMemoryError> {
    connection
        .query_row(
            "SELECT id, snapshot_code, family_code, session_code, run_code, snapshot_kind, status, source_start_seq, source_end_seq, source_checkpoint_message_code, summary_markdown, token_estimate, error_code, error_message, metadata_json, created_at, completed_at, expires_at
             FROM agent_session_family_snapshots
             WHERE session_code = ?1
               AND run_code = ?2
               AND snapshot_kind = 'SESSION_SUMMARY'
             ORDER BY id DESC
             LIMIT 1",
            params![session_code, run_code],
            row_to_family_snapshot,
        )
        .optional()
        .map_err(storage_error)
}

pub(super) fn latest_completed_session_summary(
    connection: &Connection,
    session_code: &str,
) -> Result<Option<ShortTermMemoryFamilySnapshot>, ShortTermMemoryError> {
    connection
        .query_row(
            "SELECT id, snapshot_code, family_code, session_code, run_code, snapshot_kind, status, source_start_seq, source_end_seq, source_checkpoint_message_code, summary_markdown, token_estimate, error_code, error_message, metadata_json, created_at, completed_at, expires_at
             FROM agent_session_family_snapshots
             WHERE session_code = ?1
               AND snapshot_kind = 'SESSION_SUMMARY'
               AND status = 'COMPLETED'
               AND source_end_seq IS NOT NULL
             ORDER BY source_end_seq DESC, id DESC
             LIMIT 1",
            [session_code],
            row_to_family_snapshot,
        )
        .optional()
        .map_err(storage_error)
}

pub(super) fn latest_completed_family_handoff(
    connection: &Connection,
    family_code: &str,
    session_code: &str,
) -> Result<Option<ShortTermMemoryFamilySnapshot>, ShortTermMemoryError> {
    connection
        .query_row(
            "SELECT id, snapshot_code, family_code, session_code, run_code, snapshot_kind, status, source_start_seq, source_end_seq, source_checkpoint_message_code, summary_markdown, token_estimate, error_code, error_message, metadata_json, created_at, completed_at, expires_at
             FROM agent_session_family_snapshots
             WHERE family_code = ?1
               AND session_code = ?2
               AND snapshot_kind = 'RUN_HANDOFF'
               AND status = 'COMPLETED'
             ORDER BY source_end_seq DESC, created_at DESC, id DESC
             LIMIT 1",
            params![family_code, session_code],
            row_to_family_snapshot,
        )
        .optional()
        .map_err(storage_error)
}

pub(super) fn create_family_snapshots(
    connection: &mut Connection,
    snapshots: &[ShortTermMemoryFamilySnapshot],
) -> Result<(), ShortTermMemoryError> {
    let tx = connection.transaction().map_err(storage_error)?;
    for snapshot in snapshots {
        insert_family_snapshot(&tx, snapshot)?;
        if snapshot.status == "COMPLETED"
            && snapshot.snapshot_kind == "RUN_HANDOFF"
            && let Some(family_code) = snapshot.family_code.as_deref()
        {
            tx.execute(
                "UPDATE agent_session_families
                 SET latest_snapshot_code = ?1,
                     last_activity_at = ?2,
                     updated_at = ?2
                 WHERE family_code = ?3",
                params![
                    snapshot.snapshot_code,
                    option_timestamp_to_millis(Some(snapshot.created_at.as_str())),
                    family_code,
                ],
            )
            .map_err(storage_error)?;
        }
    }
    tx.commit().map_err(storage_error)
}

pub(super) fn recover_stale_session_summaries(
    connection: &Connection,
    completed_at: &str,
) -> Result<usize, ShortTermMemoryError> {
    connection
        .execute(
            "UPDATE agent_session_family_snapshots
             SET status = 'FAILED',
                 error_code = 'SESSION_SUMMARY_INTERRUPTED',
                 error_message = 'session summary was interrupted by process restart',
                 completed_at = ?1
             WHERE snapshot_kind = 'SESSION_SUMMARY'
               AND status IN ('PENDING', 'IN_PROGRESS')",
            [timestamp_to_millis(completed_at)],
        )
        .map_err(storage_error)
}
