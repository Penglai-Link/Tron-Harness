use std::{fs, path::Path};

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};

use crate::{
    domain::automation::{
        Automation, AutomationError, AutomationExecutionRecord,
        automation_execution_status_from_storage, automation_execution_status_to_storage,
        automation_status_from_storage, automation_status_to_storage,
        automation_trigger_source_from_storage,
    },
    shared::id::SnowflakeId,
};

pub(super) fn ensure_database_parent(database_url: &str) -> Result<(), AutomationError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| AutomationError::StorageFailure)?;
    }
    Ok(())
}

pub(super) fn storage_error(error: rusqlite::Error) -> AutomationError {
    eprintln!("automation sqlite error: {error}");
    AutomationError::StorageFailure
}

pub(super) fn column_exists(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
) -> Result<bool, AutomationError> {
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
) -> Result<bool, AutomationError> {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
            [table_name],
            |_| Ok(true),
        )
        .optional()
        .map(|value| value.unwrap_or(false))
        .map_err(storage_error)
}

pub(super) fn next_id_in_transaction(tx: &Transaction<'_>) -> Result<i64, AutomationError> {
    let max_id = tx
        .query_row(
            "SELECT max(id) FROM (SELECT id FROM automations UNION ALL SELECT id FROM automation_execution_records)",
            [],
            |row| row.get::<_, Option<i64>>(0),
        )
        .map_err(storage_error)?
        .unwrap_or(734_600_000_000_000_000);
    Ok(max_id + 1)
}

pub(super) fn update_automation_in_transaction(
    tx: &Transaction<'_>,
    automation: &Automation,
) -> Result<(), AutomationError> {
    tx.execute(
        "UPDATE automations SET workspace_code = ?2, name = ?3, description = ?4, agent_code = ?5, model_code = ?6, prompt_text = ?7, cron_expression = ?8, scheduler_expression = ?9, timezone = ?10, status = ?11, previous_status = ?12, last_execution_status = ?13, last_execution_code = ?14, last_execution_summary = ?15, last_started_at = ?16, last_finished_at = ?17, updated_at = ?18, archived_at = ?19 WHERE automation_code = ?1",
        params![
            automation.automation_code,
            automation.workspace_code,
            automation.name,
            automation.description,
            automation.agent_code,
            automation.model_code,
            automation.prompt_text,
            automation.cron_expression,
            automation.scheduler_expression,
            automation.timezone,
            automation_status_to_storage(automation.status),
            automation.previous_status.map(automation_status_to_storage),
            automation.last_execution_status.map(automation_execution_status_to_storage),
            automation.last_execution_code,
            automation.last_execution_summary,
            automation.last_started_at.as_deref().map(timestamp_to_millis),
            automation.last_finished_at.as_deref().map(timestamp_to_millis),
            timestamp_to_millis(&automation.updated_at),
            automation.archived_at.as_deref().map(timestamp_to_millis),
        ],
    ).map_err(storage_error)?;
    Ok(())
}

pub(super) fn row_to_automation(row: &Row<'_>) -> rusqlite::Result<Automation> {
    let status: String = row.get(11)?;
    let previous_status: Option<String> = row.get(12)?;
    let last_execution_status: Option<String> = row.get(13)?;
    Ok(Automation {
        id: SnowflakeId::new(row.get(0)?),
        automation_code: row.get(1)?,
        workspace_code: row.get(2)?,
        name: row.get(3)?,
        description: row.get(4)?,
        agent_code: row.get(5)?,
        model_code: row.get(6)?,
        prompt_text: row.get(7)?,
        cron_expression: row.get(8)?,
        scheduler_expression: row.get(9)?,
        timezone: row.get(10)?,
        status: automation_status_from_storage(&status).map_err(automation_mapping_error)?,
        previous_status: previous_status
            .as_deref()
            .map(automation_status_from_storage)
            .transpose()
            .map_err(automation_mapping_error)?,
        last_execution_status: last_execution_status
            .as_deref()
            .map(automation_execution_status_from_storage)
            .transpose()
            .map_err(automation_mapping_error)?,
        last_execution_code: row.get(14)?,
        last_execution_summary: row.get(15)?,
        last_started_at: row.get::<_, Option<i64>>(16)?.map(timestamp_from_millis),
        last_finished_at: row.get::<_, Option<i64>>(17)?.map(timestamp_from_millis),
        created_at: timestamp_from_millis(row.get(18)?),
        updated_at: timestamp_from_millis(row.get(19)?),
        archived_at: row.get::<_, Option<i64>>(20)?.map(timestamp_from_millis),
    })
}

pub(super) fn row_to_execution_record(
    row: &Row<'_>,
) -> rusqlite::Result<AutomationExecutionRecord> {
    let trigger_source: String = row.get(5)?;
    let status: String = row.get(6)?;
    Ok(AutomationExecutionRecord {
        id: SnowflakeId::new(row.get(0)?),
        execution_code: row.get(1)?,
        automation_code: row.get(2)?,
        workspace_code: row.get(3)?,
        session_code: row.get(4)?,
        trigger_source: automation_trigger_source_from_storage(&trigger_source)
            .map_err(automation_mapping_error)?,
        status: automation_execution_status_from_storage(&status)
            .map_err(automation_mapping_error)?,
        summary: row.get(7)?,
        scheduled_at: row.get::<_, Option<i64>>(8)?.map(timestamp_from_millis),
        started_at: timestamp_from_millis(row.get(9)?),
        finished_at: row.get::<_, Option<i64>>(10)?.map(timestamp_from_millis),
        created_at: timestamp_from_millis(row.get(11)?),
    })
}

pub(super) fn hydrate_execution_summary(
    connection: &Connection,
    mut record: AutomationExecutionRecord,
) -> Result<AutomationExecutionRecord, AutomationError> {
    if let Some(session_code) = record.session_code.as_deref()
        && let Some(reply) = latest_assistant_reply_for_session(connection, session_code)?
    {
        record.summary = Some(reply);
    }
    Ok(record)
}

pub(super) fn latest_assistant_reply_for_session(
    connection: &Connection,
    session_code: &str,
) -> Result<Option<String>, AutomationError> {
    if session_code.trim().is_empty() || !chat_message_schema_available(connection)? {
        return Ok(None);
    }
    let reply = connection
        .query_row(
            "SELECT content
             FROM chat_messages
             WHERE session_code = ?1
               AND lower(role) = 'assistant'
               AND trim(content) <> ''
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
            [session_code],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?;
    Ok(reply
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty()))
}

pub(super) fn backfill_execution_session_codes(
    connection: &Connection,
) -> Result<(), AutomationError> {
    let mut statement = connection
        .prepare(
            "SELECT execution_code, summary FROM automation_execution_records WHERE session_code IS NULL AND summary IS NOT NULL",
        )
        .map_err(storage_error)?;
    let candidates = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    for (execution_code, summary) in candidates {
        let Some(session_code) = summary
            .as_deref()
            .and_then(extract_session_code_from_summary)
        else {
            continue;
        };
        connection
            .execute(
                "UPDATE automation_execution_records SET session_code = ?2 WHERE execution_code = ?1 AND session_code IS NULL",
                params![execution_code, session_code],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn chat_message_schema_available(connection: &Connection) -> Result<bool, AutomationError> {
    Ok(table_exists(connection, "chat_messages")?
        && column_exists(connection, "chat_messages", "session_code")?
        && column_exists(connection, "chat_messages", "role")?
        && column_exists(connection, "chat_messages", "content")?
        && column_exists(connection, "chat_messages", "created_at")?
        && column_exists(connection, "chat_messages", "id")?)
}

fn extract_session_code_from_summary(summary: &str) -> Option<String> {
    let marker = " in session ";
    let start = summary.find(marker)? + marker.len();
    let value = summary[start..]
        .split_whitespace()
        .next()?
        .trim_matches(|item: char| matches!(item, '.' | ',' | ';'));
    if value.starts_with("SESSION-") {
        Some(value.to_string())
    } else {
        None
    }
}

fn automation_mapping_error(error: AutomationError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

pub(super) fn timestamp_to_millis(value: &str) -> i64 {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.timestamp_millis())
        .unwrap_or_else(|_| Utc::now().timestamp_millis())
}

fn timestamp_from_millis(value: i64) -> String {
    Utc.timestamp_millis_opt(value)
        .single()
        .unwrap_or_else(Utc::now)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
