use std::{fs, path::Path};

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{Connection, Row, params};

use crate::{
    application::automation::AutomationRepository,
    domain::automation::{
        Automation, AutomationError, AutomationExecutionRecord, automation_execution_status_from_storage,
        automation_execution_status_to_storage, automation_status_from_storage,
        automation_status_to_storage, automation_trigger_source_from_storage,
        automation_trigger_source_to_storage,
    },
    shared::id::SnowflakeId,
};

use super::migrations::AUTOMATIONS_MIGRATION;

pub struct SqliteAutomationRepository {
    connection: Connection,
}

impl SqliteAutomationRepository {
    pub fn connect(database_url: &str) -> Result<Self, AutomationError> {
        ensure_database_parent(database_url)?;
        let connection = if database_url == ":memory:" || database_url.starts_with("file:") {
            Connection::open_with_flags(
                database_url,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                    | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                    | rusqlite::OpenFlags::SQLITE_OPEN_URI,
            )
        } else {
            Connection::open(database_url)
        }
        .map_err(storage_error)?;
        let repository = Self { connection };
        repository
            .connection
            .execute_batch(AUTOMATIONS_MIGRATION)
            .map_err(storage_error)?;
        Ok(repository)
    }
}

impl AutomationRepository for SqliteAutomationRepository {
    fn list_automations(&mut self) -> Result<Vec<Automation>, AutomationError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, automation_code, workspace_code, name, description, prompt_text, cron_expression, scheduler_expression, timezone, status, previous_status, last_execution_status, last_execution_code, last_execution_summary, last_started_at, last_finished_at, created_at, updated_at, archived_at FROM automations ORDER BY id ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], row_to_automation)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn next_id(&mut self) -> Result<i64, AutomationError> {
        let max_id = self
            .connection
            .query_row(
                "SELECT max(id) FROM (SELECT id FROM automations UNION ALL SELECT id FROM automation_execution_records)",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(storage_error)?
            .unwrap_or(734_600_000_000_000_000);
        Ok(max_id + 1)
    }

    fn create_automation(&mut self, automation: &Automation) -> Result<(), AutomationError> {
        self.connection.execute(
            "INSERT INTO automations (id, automation_code, workspace_code, name, description, prompt_text, cron_expression, scheduler_expression, timezone, status, previous_status, last_execution_status, last_execution_code, last_execution_summary, last_started_at, last_finished_at, created_at, updated_at, archived_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                automation.id.value(),
                automation.automation_code,
                automation.workspace_code,
                automation.name,
                automation.description,
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
                timestamp_to_millis(&automation.created_at),
                timestamp_to_millis(&automation.updated_at),
                automation.archived_at.as_deref().map(timestamp_to_millis),
            ],
        ).map_err(storage_error)?;
        Ok(())
    }

    fn update_automation(&mut self, automation: &Automation) -> Result<(), AutomationError> {
        self.connection.execute(
            "UPDATE automations SET workspace_code = ?2, name = ?3, description = ?4, prompt_text = ?5, cron_expression = ?6, scheduler_expression = ?7, timezone = ?8, status = ?9, previous_status = ?10, last_execution_status = ?11, last_execution_code = ?12, last_execution_summary = ?13, last_started_at = ?14, last_finished_at = ?15, updated_at = ?16, archived_at = ?17 WHERE automation_code = ?1",
            params![
                automation.automation_code,
                automation.workspace_code,
                automation.name,
                automation.description,
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

    fn list_execution_records(
        &mut self,
        automation_code: &str,
    ) -> Result<Vec<AutomationExecutionRecord>, AutomationError> {
        let mut statement = self.connection.prepare(
            "SELECT id, execution_code, automation_code, workspace_code, trigger_source, status, summary, scheduled_at, started_at, finished_at, created_at FROM automation_execution_records WHERE automation_code = ?1 ORDER BY started_at DESC, id DESC"
        ).map_err(storage_error)?;
        statement
            .query_map([automation_code], row_to_execution_record)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn create_execution_record(
        &mut self,
        record: &AutomationExecutionRecord,
    ) -> Result<(), AutomationError> {
        self.connection.execute(
            "INSERT INTO automation_execution_records (id, execution_code, automation_code, workspace_code, trigger_source, status, summary, scheduled_at, started_at, finished_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                record.id.value(),
                record.execution_code,
                record.automation_code,
                record.workspace_code,
                automation_trigger_source_to_storage(record.trigger_source),
                automation_execution_status_to_storage(record.status),
                record.summary,
                record.scheduled_at.as_deref().map(timestamp_to_millis),
                timestamp_to_millis(&record.started_at),
                record.finished_at.as_deref().map(timestamp_to_millis),
                timestamp_to_millis(&record.created_at),
            ],
        ).map_err(storage_error)?;
        Ok(())
    }
}

fn row_to_automation(row: &Row<'_>) -> rusqlite::Result<Automation> {
    let status: String = row.get(9)?;
    let previous_status: Option<String> = row.get(10)?;
    let last_execution_status: Option<String> = row.get(11)?;
    Ok(Automation {
        id: SnowflakeId::new(row.get(0)?),
        automation_code: row.get(1)?,
        workspace_code: row.get(2)?,
        name: row.get(3)?,
        description: row.get(4)?,
        prompt_text: row.get(5)?,
        cron_expression: row.get(6)?,
        scheduler_expression: row.get(7)?,
        timezone: row.get(8)?,
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
        last_execution_code: row.get(12)?,
        last_execution_summary: row.get(13)?,
        last_started_at: row.get::<_, Option<i64>>(14)?.map(timestamp_from_millis),
        last_finished_at: row.get::<_, Option<i64>>(15)?.map(timestamp_from_millis),
        created_at: timestamp_from_millis(row.get(16)?),
        updated_at: timestamp_from_millis(row.get(17)?),
        archived_at: row.get::<_, Option<i64>>(18)?.map(timestamp_from_millis),
    })
}

fn row_to_execution_record(row: &Row<'_>) -> rusqlite::Result<AutomationExecutionRecord> {
    let trigger_source: String = row.get(4)?;
    let status: String = row.get(5)?;
    Ok(AutomationExecutionRecord {
        id: SnowflakeId::new(row.get(0)?),
        execution_code: row.get(1)?,
        automation_code: row.get(2)?,
        workspace_code: row.get(3)?,
        trigger_source: automation_trigger_source_from_storage(&trigger_source)
            .map_err(automation_mapping_error)?,
        status: automation_execution_status_from_storage(&status)
            .map_err(automation_mapping_error)?,
        summary: row.get(6)?,
        scheduled_at: row.get::<_, Option<i64>>(7)?.map(timestamp_from_millis),
        started_at: timestamp_from_millis(row.get(8)?),
        finished_at: row.get::<_, Option<i64>>(9)?.map(timestamp_from_millis),
        created_at: timestamp_from_millis(row.get(10)?),
    })
}

fn ensure_database_parent(database_url: &str) -> Result<(), AutomationError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|_| AutomationError::StorageFailure)?;
        }
    }
    Ok(())
}

fn storage_error(error: rusqlite::Error) -> AutomationError {
    eprintln!("automation sqlite error: {error}");
    AutomationError::StorageFailure
}

fn automation_mapping_error(error: AutomationError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

fn timestamp_to_millis(value: &str) -> i64 {
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
