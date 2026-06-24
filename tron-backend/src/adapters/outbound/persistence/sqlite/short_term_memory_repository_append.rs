use rusqlite::Connection;

use crate::domain::short_term_memory::{
    ShortTermMemoryContextEpoch, ShortTermMemoryError, ShortTermMemoryEvent,
    ShortTermMemoryMessage, ShortTermMemoryPart,
};

use super::short_term_memory_rows::storage_error;

pub(super) fn assign_message_storage_fields(
    connection: &Connection,
    message: &mut ShortTermMemoryMessage,
    parts: &mut [ShortTermMemoryPart],
) -> Result<(), ShortTermMemoryError> {
    message.id = next_table_id_on(connection, "agent_transcript_messages")?;
    message.seq = next_transcript_seq_on(connection, &message.session_code)?;
    if message.message_code.is_empty() {
        message.message_code = format!("ATM-{}", message.id);
    }

    let mut next_part_id = next_table_id_on(connection, "agent_transcript_parts")?;
    for part in parts {
        part.id = next_part_id;
        next_part_id += 1;
        if part.part_code.is_empty() {
            part.part_code = format!("ATP-{}", part.id);
        }
        part.message_code = message.message_code.clone();
        if part.session_code.is_empty() {
            part.session_code = message.session_code.clone();
        }
        if part.workspace_code.is_empty() {
            part.workspace_code = message.workspace_code.clone();
        }
    }
    Ok(())
}

pub(super) fn assign_event_storage_fields(
    connection: &Connection,
    event: &mut ShortTermMemoryEvent,
) -> Result<(), ShortTermMemoryError> {
    event.id = next_table_id_on(connection, "agent_transcript_events")?;
    event.seq = next_transcript_seq_on(connection, &event.session_code)?;
    if event.event_code.is_empty() {
        event.event_code = format!("ATE-{}", event.id);
    }
    Ok(())
}

pub(super) fn assign_context_epoch_storage_fields(
    connection: &Connection,
    epoch: &mut ShortTermMemoryContextEpoch,
) -> Result<(), ShortTermMemoryError> {
    epoch.id = next_table_id_on(connection, "agent_transcript_context_epochs")?;
    if epoch.epoch_code.is_empty() {
        epoch.epoch_code = format!("ATCE-{}", epoch.id);
    }
    epoch.baseline_seq = next_transcript_seq_on(connection, &epoch.session_code)?.saturating_sub(1);
    epoch.replacement_seq = epoch.baseline_seq;
    epoch.revision = next_context_epoch_revision_on(connection, &epoch.session_code)?;
    Ok(())
}

fn next_transcript_seq_on(
    connection: &Connection,
    session_code: &str,
) -> Result<i64, ShortTermMemoryError> {
    let max_seq = connection
        .query_row(
            "SELECT max(seq) FROM (
                SELECT seq FROM agent_transcript_events WHERE session_code = ?1
                UNION ALL
                SELECT seq FROM agent_transcript_messages WHERE session_code = ?1
            )",
            [session_code],
            |row| row.get::<_, Option<i64>>(0),
        )
        .map_err(storage_error)?
        .unwrap_or(0);
    Ok(max_seq + 1)
}

fn next_context_epoch_revision_on(
    connection: &Connection,
    session_code: &str,
) -> Result<i64, ShortTermMemoryError> {
    let max_revision = connection
        .query_row(
            "SELECT COALESCE(max(revision), 0)
             FROM agent_transcript_context_epochs
             WHERE session_code = ?1",
            [session_code],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    Ok(max_revision + 1)
}

fn next_table_id_on(connection: &Connection, table: &str) -> Result<i64, ShortTermMemoryError> {
    let max_id = connection
        .query_row(&format!("SELECT max(id) FROM {table}"), [], |row| {
            row.get::<_, Option<i64>>(0)
        })
        .map_err(storage_error)?
        .unwrap_or(735_900_000_000_000_000);
    Ok(max_id + 1)
}
