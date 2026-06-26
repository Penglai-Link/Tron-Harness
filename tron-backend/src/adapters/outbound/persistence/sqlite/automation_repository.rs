use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params, params_from_iter};

use crate::{
    application::automation::AutomationRepository,
    domain::automation::{
        Automation, AutomationError, AutomationExecutionDraft, AutomationExecutionRecord,
        AutomationExecutionStats, AutomationExecutionStatus,
        automation_execution_status_to_storage, automation_status_to_storage,
        automation_trigger_source_to_storage,
    },
    shared::id::SnowflakeId,
};

use super::automation_repository_helpers::{
    backfill_execution_session_codes, column_exists, ensure_database_parent,
    hydrate_execution_summary, latest_assistant_reply_for_session, next_id_in_transaction,
    row_to_automation, row_to_execution_record, storage_error, timestamp_to_millis,
    update_automation_in_transaction,
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
