use std::{fs, path::Path};

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::{
    application::agent::AgentRunHistoryRepository,
    domain::{
        agent::{AgentRunHistoryError, AgentRunHistoryRecord, AgentRunStatus},
        reasoning::ReasoningEffort,
    },
    shared::time::{timestamp_from_millis, timestamp_to_millis},
};

use super::migrations::{
    AGENT_RUN_HISTORY_MIGRATION, AGENT_RUN_REASONING_EFFORT_MIGRATION,
    AGENT_RUN_TOKEN_USAGE_MIGRATION,
};

pub struct SqliteAgentRunRepository {
    connection: Connection,
}

impl SqliteAgentRunRepository {
    pub fn connect(database_url: &str) -> Result<Self, AgentRunHistoryError> {
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
            .execute_batch(AGENT_RUN_HISTORY_MIGRATION)
            .map_err(storage_error)?;
        if !column_exists(&repository.connection, "agent_runs", "input_tokens")? {
            repository
                .connection
                .execute_batch(AGENT_RUN_TOKEN_USAGE_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(&repository.connection, "agent_runs", "reasoning_effort")? {
            repository
                .connection
                .execute_batch(AGENT_RUN_REASONING_EFFORT_MIGRATION)
                .map_err(storage_error)?;
        }
        Ok(repository)
    }
}

impl AgentRunHistoryRepository for SqliteAgentRunRepository {
    fn list_runs(&mut self) -> Result<Vec<AgentRunHistoryRecord>, AgentRunHistoryError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, run_code, conversation_code, workspace_code, user_code, app_code, task_code, model_code, reasoning_effort, status, outcome, step_count, observation_count, input_tokens, output_tokens, total_tokens, output_preview, error_code, error_message, started_at, finished_at, created_at
                 FROM agent_runs
                 ORDER BY created_at DESC, id DESC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], row_to_agent_run)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn get_run(&mut self, run_code: &str) -> Result<AgentRunHistoryRecord, AgentRunHistoryError> {
        self.connection
            .query_row(
                "SELECT id, run_code, conversation_code, workspace_code, user_code, app_code, task_code, model_code, reasoning_effort, status, outcome, step_count, observation_count, input_tokens, output_tokens, total_tokens, output_preview, error_code, error_message, started_at, finished_at, created_at
                 FROM agent_runs
                 WHERE run_code = ?1",
                [run_code],
                row_to_agent_run,
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(AgentRunHistoryError::RunNotFound)
    }

    fn next_id(&mut self) -> Result<i64, AgentRunHistoryError> {
        let max_id = self
            .connection
            .query_row("SELECT max(id) FROM agent_runs", [], |row| {
                row.get::<_, Option<i64>>(0)
            })
            .map_err(storage_error)?
            .unwrap_or(735_100_000_000_000_000);
        Ok(max_id + 1)
    }

    fn create_run(&mut self, record: &AgentRunHistoryRecord) -> Result<(), AgentRunHistoryError> {
        self.connection
            .execute(
                "INSERT INTO agent_runs (id, run_code, conversation_code, workspace_code, user_code, app_code, task_code, model_code, reasoning_effort, status, outcome, step_count, observation_count, input_tokens, output_tokens, total_tokens, output_preview, error_code, error_message, started_at, finished_at, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
                params![
                    record.id,
                    record.run_code,
                    record.conversation_code,
                    record.workspace_code,
                    record.user_code,
                    record.app_code,
                    record.task_code,
                    record.model_code,
                    record.reasoning_effort.map(ReasoningEffort::storage_value),
                    agent_run_status_to_storage(record.status),
                    record.outcome,
                    record.step_count,
                    record.observation_count,
                    record.input_tokens,
                    record.output_tokens,
                    record.total_tokens,
                    record.output_preview,
                    record.error_code,
                    record.error_message,
                    timestamp_to_millis(&record.started_at),
                    timestamp_to_millis(&record.finished_at),
                    timestamp_to_millis(&record.created_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

fn row_to_agent_run(row: &Row<'_>) -> rusqlite::Result<AgentRunHistoryRecord> {
    let reasoning_effort = row
        .get::<_, Option<String>>(8)?
        .map(|value| {
            ReasoningEffort::from_storage_value(&value)
                .ok_or(AgentRunHistoryError::StorageFailure)
                .map_err(agent_run_mapping_error)
        })
        .transpose()?;
    let status: String = row.get(9)?;
    Ok(AgentRunHistoryRecord {
        id: row.get(0)?,
        run_code: row.get(1)?,
        conversation_code: row.get(2)?,
        workspace_code: row.get(3)?,
        user_code: row.get(4)?,
        app_code: row.get(5)?,
        task_code: row.get(6)?,
        model_code: row.get(7)?,
        reasoning_effort,
        status: agent_run_status_from_storage(&status).map_err(agent_run_mapping_error)?,
        outcome: row.get(10)?,
        step_count: row.get::<_, u32>(11)?,
        observation_count: row.get::<_, u32>(12)?,
        input_tokens: row.get(13)?,
        output_tokens: row.get(14)?,
        total_tokens: row.get(15)?,
        output_preview: row.get(16)?,
        error_code: row.get(17)?,
        error_message: row.get(18)?,
        started_at: timestamp_from_millis(row.get(19)?),
        finished_at: timestamp_from_millis(row.get(20)?),
        created_at: timestamp_from_millis(row.get(21)?),
    })
}

fn column_exists(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
) -> Result<bool, AgentRunHistoryError> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut statement = connection.prepare(&pragma).map_err(storage_error)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(columns.iter().any(|value| value == column_name))
}

fn agent_run_status_to_storage(status: AgentRunStatus) -> &'static str {
    match status {
        AgentRunStatus::Completed => "COMPLETED",
        AgentRunStatus::Failed => "FAILED",
    }
}

fn agent_run_status_from_storage(value: &str) -> Result<AgentRunStatus, AgentRunHistoryError> {
    match value {
        "COMPLETED" => Ok(AgentRunStatus::Completed),
        "FAILED" => Ok(AgentRunStatus::Failed),
        _ => Err(AgentRunHistoryError::StorageFailure),
    }
}

fn ensure_database_parent(database_url: &str) -> Result<(), AgentRunHistoryError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|_| AgentRunHistoryError::StorageFailure)?;
        }
    }
    Ok(())
}

fn storage_error(error: rusqlite::Error) -> AgentRunHistoryError {
    eprintln!("agent run sqlite error: {error}");
    AgentRunHistoryError::StorageFailure
}

fn agent_run_mapping_error(error: AgentRunHistoryError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}
