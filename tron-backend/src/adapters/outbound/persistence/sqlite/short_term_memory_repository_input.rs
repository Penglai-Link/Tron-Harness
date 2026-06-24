use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::{
    domain::short_term_memory::{
        ShortTermMemoryError, ShortTermMemoryEvent, ShortTermMemoryInput, ShortTermMemoryMessage,
        ShortTermMemoryPart,
    },
    shared::time::{option_timestamp_to_millis, timestamp_to_millis},
};

use super::{
    super::{
        short_term_memory_repository_append::{
            assign_event_storage_fields, assign_message_storage_fields,
        },
        short_term_memory_rows::{
            insert_event, insert_message, insert_part, json_to_storage, option_json_to_storage,
            row_to_input, storage_error,
        },
    },
    repository_init::next_table_id,
};

pub(super) fn create_input(
    connection: &mut Connection,
    input: &ShortTermMemoryInput,
) -> Result<(), ShortTermMemoryError> {
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let mut input = input.clone();
    assign_input_storage_id(&tx, &mut input)?;
    insert_input(&tx, &input)?;
    tx.commit().map_err(storage_error)
}

pub(super) fn input_code_by_idempotency_key(
    connection: &Connection,
    session_code: &str,
    idempotency_key: &str,
) -> Result<Option<String>, ShortTermMemoryError> {
    connection
        .query_row(
            "SELECT input_code
             FROM agent_transcript_inputs
             WHERE session_code = ?1
               AND idempotency_key = ?2
             ORDER BY id ASC
             LIMIT 1",
            params![session_code, idempotency_key],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage_error)
}

pub(super) fn list_inputs_for_run(
    connection: &Connection,
    session_code: &str,
    run_code: &str,
) -> Result<Vec<ShortTermMemoryInput>, ShortTermMemoryError> {
    let mut statement = connection
        .prepare(
            "SELECT id, input_code, session_code, workspace_code, run_code, input_kind, delivery, status, priority, idempotency_key, prompt_json, metadata_json, admitted_seq, promoted_seq, created_at, promoted_at, consumed_at, cancelled_at
             FROM agent_transcript_inputs
             WHERE session_code = ?1
               AND run_code = ?2
             ORDER BY admitted_seq ASC, id ASC",
        )
        .map_err(storage_error)?;
    statement
        .query_map(params![session_code, run_code], row_to_input)
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)
}

pub(super) fn create_input_with_message_and_events(
    connection: &mut Connection,
    input: &ShortTermMemoryInput,
    message: Option<&ShortTermMemoryMessage>,
    parts: &[ShortTermMemoryPart],
    events: &[ShortTermMemoryEvent],
) -> Result<(), ShortTermMemoryError> {
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let mut input = input.clone();
    assign_input_storage_id(&tx, &mut input)?;
    insert_input(&tx, &input)?;
    if let Some(message) = message {
        let mut message = message.clone();
        let mut parts = parts.to_vec();
        assign_message_storage_fields(&tx, &mut message, &mut parts)?;
        insert_message(&tx, &message)?;
        for part in &parts {
            insert_part(&tx, part)?;
        }
    }
    for event in events {
        let mut event = event.clone();
        assign_event_storage_fields(&tx, &mut event)?;
        insert_event(&tx, &event)?;
    }
    tx.commit().map_err(storage_error)
}

fn assign_input_storage_id(
    connection: &Connection,
    input: &mut ShortTermMemoryInput,
) -> Result<(), ShortTermMemoryError> {
    if input.id == 0 {
        input.id = next_table_id(connection, "agent_transcript_inputs")?;
    }
    Ok(())
}

fn insert_input(
    connection: &Connection,
    input: &ShortTermMemoryInput,
) -> Result<(), ShortTermMemoryError> {
    connection
        .execute(
            "INSERT INTO agent_transcript_inputs (id, input_code, session_code, workspace_code, run_code, input_kind, delivery, status, priority, idempotency_key, prompt_json, metadata_json, admitted_seq, promoted_seq, created_at, promoted_at, consumed_at, cancelled_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                input.id,
                input.input_code,
                input.session_code,
                input.workspace_code,
                input.run_code,
                input.input_kind,
                input.delivery,
                input.status,
                input.priority,
                input.idempotency_key,
                json_to_storage(&input.prompt_json)?,
                option_json_to_storage(input.metadata_json.as_ref())?,
                input.admitted_seq,
                input.promoted_seq,
                timestamp_to_millis(&input.created_at),
                option_timestamp_to_millis(input.promoted_at.as_deref()),
                option_timestamp_to_millis(input.consumed_at.as_deref()),
                option_timestamp_to_millis(input.cancelled_at.as_deref()),
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}
