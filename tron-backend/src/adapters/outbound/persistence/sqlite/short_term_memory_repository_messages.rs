use rusqlite::{Connection, params};

use crate::domain::short_term_memory::{
    ShortTermMemoryError, ShortTermMemoryMessage, ShortTermMemoryPart,
};

use super::short_term_memory_rows::{list_parts, row_to_message, storage_error};

pub(super) fn list_messages_after_seq(
    connection: &Connection,
    session_code: &str,
    min_seq: i64,
) -> Result<Vec<(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)>, ShortTermMemoryError> {
    let mut statement = connection
        .prepare(
            "SELECT id, message_code, session_code, workspace_code, run_code, seq, role, message_type, status, agent_code, model_code, model_provider, model_name, model_protocol, finish_reason, error_json, input_tokens, output_tokens, reasoning_tokens, total_tokens, metadata_json, created_at, completed_at
             FROM agent_transcript_messages
             WHERE session_code = ?1
               AND seq >= ?2
               AND status = 'completed'
               AND message_type != 'compaction'
             ORDER BY seq ASC",
        )
        .map_err(storage_error)?;
    let messages = statement
        .query_map(params![session_code, min_seq], row_to_message)
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;

    hydrate_parts(connection, messages)
}

pub(super) fn list_recent_messages(
    connection: &Connection,
    session_code: &str,
    min_seq: i64,
    limit: usize,
) -> Result<Vec<(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)>, ShortTermMemoryError> {
    let mut statement = connection
        .prepare(
            "SELECT id, message_code, session_code, workspace_code, run_code, seq, role, message_type, status, agent_code, model_code, model_provider, model_name, model_protocol, finish_reason, error_json, input_tokens, output_tokens, reasoning_tokens, total_tokens, metadata_json, created_at, completed_at
             FROM agent_transcript_messages
             WHERE session_code = ?1
               AND seq >= ?2
               AND status = 'completed'
               AND message_type != 'compaction'
             ORDER BY seq DESC
             LIMIT ?3",
        )
        .map_err(storage_error)?;
    let mut messages = statement
        .query_map(params![session_code, min_seq, limit as i64], row_to_message)
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    messages.reverse();

    hydrate_parts(connection, messages)
}

pub(super) fn list_messages_for_run(
    connection: &Connection,
    session_code: &str,
    run_code: &str,
) -> Result<Vec<(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)>, ShortTermMemoryError> {
    let mut statement = connection
        .prepare(
            "SELECT id, message_code, session_code, workspace_code, run_code, seq, role, message_type, status, agent_code, model_code, model_provider, model_name, model_protocol, finish_reason, error_json, input_tokens, output_tokens, reasoning_tokens, total_tokens, metadata_json, created_at, completed_at
             FROM agent_transcript_messages
             WHERE session_code = ?1
               AND run_code = ?2
               AND status = 'completed'
             ORDER BY seq ASC, id ASC",
        )
        .map_err(storage_error)?;
    let messages = statement
        .query_map(params![session_code, run_code], row_to_message)
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;

    hydrate_parts(connection, messages)
}

fn hydrate_parts(
    connection: &Connection,
    messages: Vec<ShortTermMemoryMessage>,
) -> Result<Vec<(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)>, ShortTermMemoryError> {
    let mut result = Vec::with_capacity(messages.len());
    for message in messages {
        let parts = list_parts(connection, &message.message_code)?;
        result.push((message, parts));
    }
    Ok(result)
}
