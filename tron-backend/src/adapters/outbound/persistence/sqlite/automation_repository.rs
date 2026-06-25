use std::{fs, path::Path};

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{
    Connection, OptionalExtension, Row, Transaction, TransactionBehavior, params, params_from_iter,
};

use crate::{
    application::automation::AutomationRepository,
    domain::automation::{
        Automation, AutomationError, AutomationExecutionDraft, AutomationExecutionRecord,
        AutomationExecutionStats, AutomationExecutionStatus,
        automation_execution_status_from_storage, automation_execution_status_to_storage,
        automation_status_from_storage, automation_status_to_storage,
        automation_trigger_source_from_storage, automation_trigger_source_to_storage,
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
        if !column_exists(&repository.connection, "automations", "agent_code")? {
            repository
                .connection
                .execute("ALTER TABLE automations ADD COLUMN agent_code TEXT NOT NULL DEFAULT 'main-agent'", [])
                .map_err(storage_error)?;
        }
        if !column_exists(&repository.connection, "automations", "model_code")? {
            repository
                .connection
                .execute(
                    "ALTER TABLE automations ADD COLUMN model_code TEXT NULL",
                    [],
                )
                .map_err(storage_error)?;
        }
        if !column_exists(
            &repository.connection,
            "automation_execution_records",
            "session_code",
        )? {
            repository
                .connection
                .execute(
                    "ALTER TABLE automation_execution_records ADD COLUMN session_code TEXT NULL",
                    [],
                )
                .map_err(storage_error)?;
        }
        backfill_execution_session_codes(&repository.connection)?;
        Ok(repository)
    }
}

impl AutomationRepository for SqliteAutomationRepository {
    fn list_automations(&mut self) -> Result<Vec<Automation>, AutomationError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, automation_code, workspace_code, name, description, agent_code, model_code, prompt_text, cron_expression, scheduler_expression, timezone, status, previous_status, last_execution_status, last_execution_code, last_execution_summary, last_started_at, last_finished_at, created_at, updated_at, archived_at FROM automations ORDER BY id ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], row_to_automation)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn execution_stats(
        &mut self,
        automation_codes: &[String],
    ) -> Result<std::collections::BTreeMap<String, AutomationExecutionStats>, AutomationError> {
        if automation_codes.is_empty() {
            return Ok(std::collections::BTreeMap::new());
        }
        let placeholders = std::iter::repeat_n("?", automation_codes.len())
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!(
            "SELECT automation_code,
                    SUM(CASE WHEN status IN ('Success', 'Failed', 'Canceled', 'Skipped') THEN 1 ELSE 0 END) AS total_count,
                    SUM(CASE WHEN status = 'Success' THEN 1 ELSE 0 END) AS success_count
             FROM automation_execution_records
             WHERE automation_code IN ({placeholders})
             GROUP BY automation_code"
        );
        let mut statement = self.connection.prepare(&query).map_err(storage_error)?;
        statement
            .query_map(
                params_from_iter(automation_codes.iter().map(String::as_str)),
                |row| {
                    let automation_code: String = row.get(0)?;
                    let total_count: i64 = row.get(1)?;
                    let success_count: i64 = row.get(2)?;
                    Ok((
                        automation_code,
                        AutomationExecutionStats {
                            success_count: success_count.max(0) as u64,
                            total_count: total_count.max(0) as u64,
                        },
                    ))
                },
            )
            .map_err(storage_error)?
            .collect::<Result<std::collections::BTreeMap<_, _>, _>>()
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
            "INSERT INTO automations (id, automation_code, workspace_code, name, description, agent_code, model_code, prompt_text, cron_expression, scheduler_expression, timezone, status, previous_status, last_execution_status, last_execution_code, last_execution_summary, last_started_at, last_finished_at, created_at, updated_at, archived_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                automation.id.value(),
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
                timestamp_to_millis(&automation.created_at),
                timestamp_to_millis(&automation.updated_at),
                automation.archived_at.as_deref().map(timestamp_to_millis),
            ],
        ).map_err(storage_error)?;
        Ok(())
    }

    fn update_automation(&mut self, automation: &Automation) -> Result<(), AutomationError> {
        self.connection.execute(
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

    fn list_execution_records(
        &mut self,
        automation_code: &str,
    ) -> Result<Vec<AutomationExecutionRecord>, AutomationError> {
        let records = {
            let mut statement = self.connection.prepare(
                "SELECT id, execution_code, automation_code, workspace_code, session_code, trigger_source, status, summary, scheduled_at, started_at, finished_at, created_at FROM automation_execution_records WHERE automation_code = ?1 ORDER BY started_at DESC, id DESC"
            ).map_err(storage_error)?;
            statement
                .query_map([automation_code], row_to_execution_record)
                .map_err(storage_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage_error)?
        };
        records
            .into_iter()
            .map(|record| hydrate_execution_summary(&self.connection, record))
            .collect()
    }

    fn record_execution(
        &mut self,
        automation: &Automation,
        draft: &AutomationExecutionDraft,
    ) -> Result<Option<AutomationExecutionRecord>, AutomationError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let next_id = next_id_in_transaction(&tx)?;
        let record = AutomationExecutionRecord {
            id: SnowflakeId::new(next_id),
            execution_code: format!("AEX-{next_id}"),
            automation_code: draft.automation_code.clone(),
            workspace_code: draft.workspace_code.clone(),
            session_code: draft.session_code.clone(),
            trigger_source: draft.trigger_source,
            status: draft.status,
            summary: draft.summary.clone(),
            scheduled_at: draft.scheduled_at.clone(),
            started_at: draft.started_at.clone(),
            finished_at: draft.finished_at.clone(),
            created_at: draft.created_at.clone(),
        };
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO automation_execution_records (id, execution_code, automation_code, workspace_code, session_code, trigger_source, status, summary, scheduled_at, started_at, finished_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                record.id.value(),
                record.execution_code,
                record.automation_code,
                record.workspace_code,
                record.session_code,
                automation_trigger_source_to_storage(record.trigger_source),
                automation_execution_status_to_storage(record.status),
                record.summary,
                record.scheduled_at.as_deref().map(timestamp_to_millis),
                timestamp_to_millis(&record.started_at),
                record.finished_at.as_deref().map(timestamp_to_millis),
                timestamp_to_millis(&record.created_at),
            ],
        ).map_err(storage_error)?;
        if inserted == 0 {
            tx.commit().map_err(storage_error)?;
            return Ok(None);
        }

        let mut updated = automation.clone();
        updated.last_execution_status = Some(record.status);
        updated.last_execution_code = Some(record.execution_code.clone());
        updated.last_execution_summary = record.summary.clone();
        updated.last_started_at = Some(record.started_at.clone());
        updated.last_finished_at = record.finished_at.clone();
        updated.updated_at = draft.created_at.clone();
        update_automation_in_transaction(&tx, &updated)?;
        tx.commit().map_err(storage_error)?;
        Ok(Some(record))
    }

    fn complete_execution_record(
        &mut self,
        automation_code: &str,
        execution_code: &str,
        status: AutomationExecutionStatus,
        summary: Option<String>,
        session_code: Option<String>,
        finished_at: &str,
    ) -> Result<(), AutomationError> {
        let effective_summary = match session_code.as_deref() {
            Some(session_code) => {
                latest_assistant_reply_for_session(&self.connection, session_code)?.or(summary)
            }
            None => summary,
        };
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let changed = tx
            .execute(
                "UPDATE automation_execution_records
                 SET status = ?3, summary = ?4, session_code = COALESCE(?5, session_code), finished_at = ?6
                 WHERE automation_code = ?1 AND execution_code = ?2",
                params![
                    automation_code,
                    execution_code,
                    automation_execution_status_to_storage(status),
                    effective_summary.as_deref(),
                    session_code.as_deref(),
                    timestamp_to_millis(finished_at),
                ],
            )
            .map_err(storage_error)?;
        if changed == 0 {
            tx.commit().map_err(storage_error)?;
            return Err(AutomationError::NotFound);
        }
        tx.execute(
            "UPDATE automations
             SET last_execution_status = ?3,
                 last_execution_summary = ?4,
                 last_finished_at = ?5,
                 updated_at = ?5
             WHERE automation_code = ?1 AND last_execution_code = ?2",
            params![
                automation_code,
                execution_code,
                automation_execution_status_to_storage(status),
                effective_summary.as_deref(),
                timestamp_to_millis(finished_at),
            ],
        )
        .map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        Ok(())
    }

    fn delete_execution_record(
        &mut self,
        automation_code: &str,
        execution_code: &str,
        deleted_at: &str,
    ) -> Result<AutomationExecutionRecord, AutomationError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let deleted = tx
            .query_row(
                "SELECT id, execution_code, automation_code, workspace_code, session_code, trigger_source, status, summary, scheduled_at, started_at, finished_at, created_at
                 FROM automation_execution_records
                 WHERE automation_code = ?1 AND execution_code = ?2",
                params![automation_code, execution_code],
                row_to_execution_record,
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(AutomationError::NotFound)?;

        tx.execute(
            "DELETE FROM automation_execution_records WHERE automation_code = ?1 AND execution_code = ?2",
            params![automation_code, execution_code],
        )
        .map_err(storage_error)?;

        let latest = tx
            .query_row(
                "SELECT id, execution_code, automation_code, workspace_code, session_code, trigger_source, status, summary, scheduled_at, started_at, finished_at, created_at
                 FROM automation_execution_records
                 WHERE automation_code = ?1
                 ORDER BY started_at DESC, id DESC
                 LIMIT 1",
                params![automation_code],
                row_to_execution_record,
            )
            .optional()
            .map_err(storage_error)?;

        match latest {
            Some(record) => {
                tx.execute(
                    "UPDATE automations
                     SET last_execution_status = ?2,
                         last_execution_code = ?3,
                         last_execution_summary = ?4,
                         last_started_at = ?5,
                         last_finished_at = ?6,
                         updated_at = ?7
                     WHERE automation_code = ?1",
                    params![
                        automation_code,
                        automation_execution_status_to_storage(record.status),
                        record.execution_code,
                        record.summary,
                        timestamp_to_millis(&record.started_at),
                        record.finished_at.as_deref().map(timestamp_to_millis),
                        timestamp_to_millis(deleted_at),
                    ],
                )
                .map_err(storage_error)?;
            }
            None => {
                tx.execute(
                    "UPDATE automations
                     SET last_execution_status = NULL,
                         last_execution_code = NULL,
                         last_execution_summary = NULL,
                         last_started_at = NULL,
                         last_finished_at = NULL,
                         updated_at = ?2
                     WHERE automation_code = ?1",
                    params![automation_code, timestamp_to_millis(deleted_at)],
                )
                .map_err(storage_error)?;
            }
        }

        tx.commit().map_err(storage_error)?;
        Ok(deleted)
    }
}

fn next_id_in_transaction(tx: &Transaction<'_>) -> Result<i64, AutomationError> {
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

fn update_automation_in_transaction(
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

fn row_to_automation(row: &Row<'_>) -> rusqlite::Result<Automation> {
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

fn column_exists(
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

fn table_exists(connection: &Connection, table_name: &str) -> Result<bool, AutomationError> {
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

fn row_to_execution_record(row: &Row<'_>) -> rusqlite::Result<AutomationExecutionRecord> {
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

fn hydrate_execution_summary(
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

fn latest_assistant_reply_for_session(
    connection: &Connection,
    session_code: &str,
) -> Result<Option<String>, AutomationError> {
    if session_code.trim().is_empty() {
        return Ok(None);
    }
    if !chat_message_schema_available(connection)? {
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

fn chat_message_schema_available(connection: &Connection) -> Result<bool, AutomationError> {
    Ok(table_exists(connection, "chat_messages")?
        && column_exists(connection, "chat_messages", "session_code")?
        && column_exists(connection, "chat_messages", "role")?
        && column_exists(connection, "chat_messages", "content")?
        && column_exists(connection, "chat_messages", "created_at")?
        && column_exists(connection, "chat_messages", "id")?)
}

fn backfill_execution_session_codes(connection: &Connection) -> Result<(), AutomationError> {
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
