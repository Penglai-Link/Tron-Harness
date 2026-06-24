use std::{fs, path::Path};

use rusqlite::{Connection, Row};
use serde_json::Value;

use crate::{
    domain::short_term_memory::{
        ShortTermMemoryCompaction, ShortTermMemoryError, ShortTermMemoryFamily,
        ShortTermMemoryFamilyMember, ShortTermMemoryFamilySnapshot, ShortTermMemoryGoal,
        ShortTermMemoryGoalCriterion, ShortTermMemoryInput, ShortTermMemoryMessage,
        ShortTermMemoryPart, ShortTermMemoryTodo, TodoPriority, TodoStatus,
    },
    shared::time::{timestamp_from_millis, timestamp_to_millis},
};

#[path = "short_term_memory_rows_goal_insert.rs"]
mod goal_insert;
#[path = "short_term_memory_rows_insert.rs"]
mod insert;

pub(super) use goal_insert::{
    insert_goal, insert_goal_criterion, insert_goal_event, update_goal_row,
};
pub(super) use insert::{
    insert_context_epoch, insert_context_epoch_record, insert_event, insert_family,
    insert_family_member, insert_family_snapshot, insert_message, insert_part, insert_todo,
};

pub(super) fn list_parts(
    connection: &Connection,
    message_code: &str,
) -> Result<Vec<ShortTermMemoryPart>, ShortTermMemoryError> {
    let mut statement = connection
        .prepare(
            "SELECT id, part_code, session_code, workspace_code, message_code, seq, part_type, status, content_text, content_json, provider_metadata_json, visibility, created_at, completed_at
             FROM agent_transcript_parts
             WHERE message_code = ?1
             ORDER BY seq ASC, id ASC",
        )
        .map_err(storage_error)?;
    statement
        .query_map([message_code], row_to_part)
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)
}

pub(super) fn row_to_input(row: &Row<'_>) -> rusqlite::Result<ShortTermMemoryInput> {
    Ok(ShortTermMemoryInput {
        id: row.get(0)?,
        input_code: row.get(1)?,
        session_code: row.get(2)?,
        workspace_code: row.get(3)?,
        run_code: row.get(4)?,
        input_kind: row.get(5)?,
        delivery: row.get(6)?,
        status: row.get(7)?,
        priority: row.get(8)?,
        idempotency_key: row.get(9)?,
        prompt_json: json_from_storage(row.get::<_, String>(10)?)?,
        metadata_json: option_json_from_storage(row.get::<_, Option<String>>(11)?)?,
        admitted_seq: row.get(12)?,
        promoted_seq: row.get(13)?,
        created_at: timestamp_from_millis(row.get(14)?),
        promoted_at: option_timestamp_from_millis(row.get(15)?),
        consumed_at: option_timestamp_from_millis(row.get(16)?),
        cancelled_at: option_timestamp_from_millis(row.get(17)?),
    })
}

pub(super) fn row_to_message(row: &Row<'_>) -> rusqlite::Result<ShortTermMemoryMessage> {
    Ok(ShortTermMemoryMessage {
        id: row.get(0)?,
        message_code: row.get(1)?,
        session_code: row.get(2)?,
        workspace_code: row.get(3)?,
        run_code: row.get(4)?,
        seq: row.get(5)?,
        role: row.get(6)?,
        message_type: row.get(7)?,
        status: row.get(8)?,
        agent_code: row.get(9)?,
        model_code: row.get(10)?,
        model_provider: row.get(11)?,
        model_name: row.get(12)?,
        model_protocol: row.get(13)?,
        finish_reason: row.get(14)?,
        error_json: option_json_from_storage(row.get::<_, Option<String>>(15)?)?,
        input_tokens: row.get(16)?,
        output_tokens: row.get(17)?,
        reasoning_tokens: row.get(18)?,
        total_tokens: row.get(19)?,
        metadata_json: option_json_from_storage(row.get::<_, Option<String>>(20)?)?,
        created_at: timestamp_from_millis(row.get(21)?),
        completed_at: option_timestamp_from_millis(row.get(22)?),
    })
}

fn row_to_part(row: &Row<'_>) -> rusqlite::Result<ShortTermMemoryPart> {
    Ok(ShortTermMemoryPart {
        id: row.get(0)?,
        part_code: row.get(1)?,
        session_code: row.get(2)?,
        workspace_code: row.get(3)?,
        message_code: row.get(4)?,
        seq: row.get(5)?,
        part_type: row.get(6)?,
        status: row.get(7)?,
        content_text: row.get(8)?,
        content_json: option_json_from_storage(row.get::<_, Option<String>>(9)?)?,
        provider_metadata_json: option_json_from_storage(row.get::<_, Option<String>>(10)?)?,
        visibility: row.get(11)?,
        created_at: timestamp_from_millis(row.get(12)?),
        completed_at: option_timestamp_from_millis(row.get(13)?),
    })
}

pub(super) fn row_to_todo(row: &Row<'_>) -> rusqlite::Result<ShortTermMemoryTodo> {
    let status: String = row.get(6)?;
    let priority: String = row.get(7)?;
    Ok(ShortTermMemoryTodo {
        id: row.get(0)?,
        todo_code: row.get(1)?,
        session_code: row.get(2)?,
        workspace_code: row.get(3)?,
        position: row.get(4)?,
        content: row.get(5)?,
        status: TodoStatus::from_code(&status).map_err(mapping_error)?,
        priority: TodoPriority::from_code(&priority).map_err(mapping_error)?,
        created_by_run_code: row.get(8)?,
        updated_by_run_code: row.get(9)?,
        created_at: timestamp_from_millis(row.get(10)?),
        updated_at: timestamp_from_millis(row.get(11)?),
    })
}

pub(super) fn row_to_compaction(
    row: &Row<'_>,
) -> rusqlite::Result<Result<ShortTermMemoryCompaction, ShortTermMemoryError>> {
    let content = json_from_storage(row.get::<_, String>(3)?)?;
    let source_start_seq = content
        .get("sourceStartSeq")
        .and_then(|value| value.as_i64())
        .unwrap_or_default();
    let source_end_seq = content
        .get("sourceEndSeq")
        .and_then(|value| value.as_i64())
        .unwrap_or_default();
    let tail_start_seq = content
        .get("tailStartSeq")
        .and_then(|value| value.as_i64())
        .unwrap_or_else(|| source_end_seq.saturating_sub(3).max(0));
    let selector_version = content
        .get("selectorVersion")
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(1);
    let coverage = content
        .get("coverage")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    Ok(Ok(ShortTermMemoryCompaction {
        session_code: row.get(0)?,
        checkpoint_message_code: row.get(1)?,
        status: row.get(2)?,
        source_start_seq,
        source_end_seq,
        tail_start_seq,
        selector_version,
        coverage,
        summary_token_estimate: row.get::<_, Option<u32>>(4)?.unwrap_or_default(),
        created_at: timestamp_from_millis(row.get(5)?),
    }))
}

pub(super) fn row_to_family(row: &Row<'_>) -> rusqlite::Result<ShortTermMemoryFamily> {
    Ok(ShortTermMemoryFamily {
        id: row.get(0)?,
        family_code: row.get(1)?,
        family_kind: row.get(2)?,
        family_key: row.get(3)?,
        workspace_code: row.get(4)?,
        agent_code: row.get(5)?,
        user_code: row.get(6)?,
        app_code: row.get(7)?,
        requirement_code: row.get(8)?,
        task_code: row.get(9)?,
        parent_family_code: row.get(10)?,
        root_session_code: row.get(11)?,
        active_session_code: row.get(12)?,
        active_run_code: row.get(13)?,
        latest_snapshot_code: row.get(14)?,
        title: row.get(15)?,
        status: row.get(16)?,
        load_policy: row.get(17)?,
        load_policy_json: option_json_from_storage(row.get::<_, Option<String>>(18)?)?,
        metadata_json: option_json_from_storage(row.get::<_, Option<String>>(19)?)?,
        created_at: timestamp_from_millis(row.get(20)?),
        updated_at: timestamp_from_millis(row.get(21)?),
        last_activity_at: timestamp_from_millis(row.get(22)?),
    })
}

pub(super) fn row_to_family_member(row: &Row<'_>) -> rusqlite::Result<ShortTermMemoryFamilyMember> {
    let enabled: i64 = row.get(7)?;
    Ok(ShortTermMemoryFamilyMember {
        id: row.get(0)?,
        family_code: row.get(1)?,
        session_code: row.get(2)?,
        membership_role: row.get(3)?,
        parent_session_code: row.get(4)?,
        member_kind: row.get(5)?,
        origin_kind: row.get(6)?,
        model_context_enabled: enabled != 0,
        load_weight_override: row.get(8)?,
        load_limits_json: option_json_from_storage(row.get::<_, Option<String>>(9)?)?,
        first_run_code: row.get(10)?,
        last_run_code: row.get(11)?,
        joined_at: timestamp_from_millis(row.get(12)?),
        last_activity_at: timestamp_from_millis(row.get(13)?),
    })
}

pub(super) fn row_to_family_snapshot(
    row: &Row<'_>,
) -> rusqlite::Result<ShortTermMemoryFamilySnapshot> {
    Ok(ShortTermMemoryFamilySnapshot {
        id: row.get(0)?,
        snapshot_code: row.get(1)?,
        family_code: row.get(2)?,
        session_code: row.get(3)?,
        run_code: row.get(4)?,
        snapshot_kind: row.get(5)?,
        status: row.get(6)?,
        source_start_seq: row.get(7)?,
        source_end_seq: row.get(8)?,
        source_checkpoint_message_code: row.get(9)?,
        summary_markdown: row.get(10)?,
        token_estimate: row.get(11)?,
        error_code: row.get(12)?,
        error_message: row.get(13)?,
        metadata_json: option_json_from_storage(row.get::<_, Option<String>>(14)?)?,
        created_at: timestamp_from_millis(row.get(15)?),
        completed_at: option_timestamp_from_millis(row.get(16)?),
        expires_at: option_timestamp_from_millis(row.get(17)?),
    })
}

pub(super) fn row_to_goal(row: &Row<'_>) -> rusqlite::Result<ShortTermMemoryGoal> {
    Ok(ShortTermMemoryGoal {
        id: row.get(0)?,
        goal_code: row.get(1)?,
        session_code: row.get(2)?,
        workspace_code: row.get(3)?,
        user_code: row.get(4)?,
        task_code: row.get(5)?,
        active_family_code: row.get(6)?,
        objective_markdown: row.get(7)?,
        status: row.get(8)?,
        revision: row.get(9)?,
        runs_used: row.get(10)?,
        max_runs: row.get(11)?,
        last_run_code: row.get(12)?,
        last_input_code: row.get(13)?,
        last_verdict: row.get(14)?,
        last_reason: row.get(15)?,
        blocked_reason: row.get(16)?,
        required_user_input: row.get(17)?,
        consecutive_judge_failures: row.get(18)?,
        created_by_run_code: row.get(19)?,
        created_at: timestamp_from_millis(row.get(20)?),
        updated_at: timestamp_from_millis(row.get(21)?),
        paused_at: option_timestamp_from_millis(row.get(22)?),
        blocked_at: option_timestamp_from_millis(row.get(23)?),
        completed_at: option_timestamp_from_millis(row.get(24)?),
        cancelled_at: option_timestamp_from_millis(row.get(25)?),
        cleared_at: option_timestamp_from_millis(row.get(26)?),
        metadata_json: option_json_from_storage(row.get::<_, Option<String>>(27)?)?,
    })
}

pub(super) fn row_to_goal_criterion(
    row: &Row<'_>,
) -> rusqlite::Result<ShortTermMemoryGoalCriterion> {
    Ok(ShortTermMemoryGoalCriterion {
        id: row.get(0)?,
        criterion_code: row.get(1)?,
        goal_code: row.get(2)?,
        session_code: row.get(3)?,
        position: row.get(4)?,
        text: row.get(5)?,
        status: row.get(6)?,
        created_at: timestamp_from_millis(row.get(7)?),
        updated_at: timestamp_from_millis(row.get(8)?),
        removed_at: option_timestamp_from_millis(row.get(9)?),
    })
}

pub(super) fn column_exists(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
) -> Result<bool, ShortTermMemoryError> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut statement = connection.prepare(&pragma).map_err(storage_error)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(columns.iter().any(|value| value == column_name))
}

pub(super) fn table_exists(
    connection: &Connection,
    table_name: &str,
) -> Result<bool, ShortTermMemoryError> {
    let count: i64 = connection
        .query_row(
            "SELECT count(1) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table_name],
            |row| row.get(0),
        )
        .map_err(storage_error)?;
    Ok(count > 0)
}

pub(super) fn index_exists(
    connection: &Connection,
    index_name: &str,
) -> Result<bool, ShortTermMemoryError> {
    let count: i64 = connection
        .query_row(
            "SELECT count(1) FROM sqlite_master WHERE type = 'index' AND name = ?1",
            [index_name],
            |row| row.get(0),
        )
        .map_err(storage_error)?;
    Ok(count > 0)
}

fn next_id(connection: &Connection, table: &str) -> Result<i64, ShortTermMemoryError> {
    let max_id = connection
        .query_row(&format!("SELECT max(id) FROM {table}"), [], |row| {
            row.get::<_, Option<i64>>(0)
        })
        .map_err(storage_error)?
        .unwrap_or(735_900_000_000_000_000);
    Ok(max_id + 1)
}

pub(super) fn json_to_storage(value: &Value) -> Result<String, ShortTermMemoryError> {
    serde_json::to_string(value).map_err(|_| ShortTermMemoryError::StorageFailure)
}

pub(super) fn option_json_to_storage(
    value: Option<&Value>,
) -> Result<Option<String>, ShortTermMemoryError> {
    value.map(json_to_storage).transpose()
}

fn json_from_storage(value: String) -> rusqlite::Result<Value> {
    serde_json::from_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

pub(super) fn option_json_from_storage(value: Option<String>) -> rusqlite::Result<Option<Value>> {
    value.map(json_from_storage).transpose()
}

pub(super) fn option_timestamp_to_millis(value: Option<&str>) -> Option<i64> {
    value.map(timestamp_to_millis)
}

pub(super) fn option_timestamp_from_millis(value: Option<i64>) -> Option<String> {
    value.map(timestamp_from_millis)
}

pub(super) fn ensure_database_parent(database_url: &str) -> Result<(), ShortTermMemoryError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| ShortTermMemoryError::StorageFailure)?;
    }
    Ok(())
}

pub(super) fn storage_error(error: rusqlite::Error) -> ShortTermMemoryError {
    eprintln!("short-term memory sqlite error: {error}");
    ShortTermMemoryError::StorageFailure
}

fn mapping_error(error: ShortTermMemoryError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}
