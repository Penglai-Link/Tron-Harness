use rusqlite::{Connection, OptionalExtension, TransactionBehavior};

use crate::domain::short_term_memory::{
    ShortTermMemoryCompaction, ShortTermMemoryContextEpoch, ShortTermMemoryError,
    ShortTermMemoryEvent,
};

use super::super::{
    short_term_memory_repository_append::{
        assign_context_epoch_storage_fields, assign_event_storage_fields,
    },
    short_term_memory_rows::{
        insert_context_epoch_record, insert_event, row_to_compaction, storage_error,
    },
};

pub(super) fn create_context_epoch(
    connection: &mut Connection,
    epoch: &ShortTermMemoryContextEpoch,
) -> Result<(), ShortTermMemoryError> {
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let mut epoch = epoch.clone();
    assign_context_epoch_storage_fields(&tx, &mut epoch)?;
    insert_context_epoch_record(&tx, &epoch)?;
    tx.commit().map_err(storage_error)
}

pub(super) fn create_context_epoch_with_event(
    connection: &mut Connection,
    epoch: &ShortTermMemoryContextEpoch,
    event: &ShortTermMemoryEvent,
) -> Result<ShortTermMemoryContextEpoch, ShortTermMemoryError> {
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let mut epoch = epoch.clone();
    assign_context_epoch_storage_fields(&tx, &mut epoch)?;
    insert_context_epoch_record(&tx, &epoch)?;
    let mut event = event.clone();
    assign_event_storage_fields(&tx, &mut event)?;
    insert_event(&tx, &event)?;
    tx.commit().map_err(storage_error)?;
    Ok(epoch)
}

pub(super) fn latest_context_epoch_revision(
    connection: &Connection,
    session_code: &str,
) -> Result<i64, ShortTermMemoryError> {
    connection
        .query_row(
            "SELECT COALESCE(max(revision), 0)
             FROM agent_transcript_context_epochs
             WHERE session_code = ?1",
            [session_code],
            |row| row.get(0),
        )
        .map_err(storage_error)
}

pub(super) fn latest_compaction(
    connection: &Connection,
    session_code: &str,
) -> Result<Option<ShortTermMemoryCompaction>, ShortTermMemoryError> {
    connection
        .query_row(
            "SELECT m.session_code, m.message_code, m.status, p.content_json, m.total_tokens, m.created_at
             FROM agent_transcript_messages m
             JOIN agent_transcript_parts p ON p.message_code = m.message_code
             WHERE m.session_code = ?1
               AND m.message_type = 'compaction'
               AND m.status = 'completed'
               AND p.part_type = 'compaction_summary'
               AND p.status = 'completed'
             ORDER BY m.seq DESC
             LIMIT 1",
            [session_code],
            row_to_compaction,
        )
        .optional()
        .map_err(storage_error)?
        .transpose()
}
