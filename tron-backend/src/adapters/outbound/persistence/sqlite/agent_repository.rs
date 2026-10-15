//! Business display facts augment native Framework runs through durable events.
use super::framework_storage::blocking;
use crate::{
    application::agent::{AgentRunHistoryRepository, FrameworkHost},
    domain::agent::{AgentRunHistoryError, AgentRunHistoryRecord, AgentRunStatus},
};
use eliteforge_agent_core as fw;

pub struct SqliteAgentRunRepository {
    host: FrameworkHost,
}
impl SqliteAgentRunRepository {
    pub fn connect(path: &str) -> Result<Self, AgentRunHistoryError> {
        let host = if path == ":memory:" {
            FrameworkHost::temporary(None).map_err(|_| AgentRunHistoryError::StorageFailure)?
        } else {
            FrameworkHost::persistent(path, None)
        };
        Self::with_host(host)
    }
    pub fn with_host(host: FrameworkHost) -> Result<Self, AgentRunHistoryError> {
        blocking(host.storage_framework()).map_err(|_| AgentRunHistoryError::StorageFailure)?;
        Ok(Self { host })
    }
    /// Batched projection: one query joins run-display events with their
    /// native runs instead of an event scan plus a run lookup per row.
    async fn project_records(
        &self,
        filter: &str,
        parameters: Vec<String>,
    ) -> Result<Vec<AgentRunHistoryRecord>, AgentRunHistoryError> {
        // Building the framework lazily creates the ef_* tables this query
        // reads; the constructor path already did this in production hosts.
        self.host
            .storage_framework()
            .await
            .map_err(|_| AgentRunHistoryError::StorageFailure)?;
        let sql = format!(
            "SELECT e.record, r.record FROM ef_events e \
             LEFT JOIN ef_runs r ON r.id=e.run_id \
             WHERE e.kind='tron.run-display' {filter} \
             ORDER BY e.session_id, e.sequence"
        );
        let raw = self
            .host
            .shared_database()
            .await
            .map_err(|_| AgentRunHistoryError::StorageFailure)?
            .call_read(move |c| -> Result<Vec<(String, Option<String>)>, AgentRunHistoryError> {
                let mut statement = c.prepare(&sql).map_err(|_| AgentRunHistoryError::StorageFailure)?;
                let rows = statement
                    .query_map(rusqlite::params_from_iter(parameters.iter()), |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
                    })
                    .map_err(|_| AgentRunHistoryError::StorageFailure)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| AgentRunHistoryError::StorageFailure)?;
                Ok(rows)
            })
            .await
            .map_err(|_| AgentRunHistoryError::StorageFailure)?;
        // The latest display event per run wins, matching the prior fold that
        // inserted every event into a map keyed by run code.
        let mut records = std::collections::BTreeMap::new();
        for (event_json, run_json) in raw {
            let event: fw::SessionEvent =
                serde_json::from_str(&event_json).map_err(|_| AgentRunHistoryError::StorageFailure)?;
            let mut record: AgentRunHistoryRecord = serde_json::from_value(event.payload)
                .map_err(|_| AgentRunHistoryError::StorageFailure)?;
            let native: fw::ManagedRun = run_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()
                .map_err(|_| AgentRunHistoryError::StorageFailure)?
                .ok_or(AgentRunHistoryError::StorageFailure)?;
            record.status = if native.status == fw::ManagedRunStatus::Completed {
                AgentRunStatus::Completed
            } else {
                AgentRunStatus::Failed
            };
            record.started_at =
                crate::shared::time::timestamp_from_millis(native.started_at_ms as i64);
            record.finished_at = crate::shared::time::timestamp_from_millis(
                native.finished_at_ms.unwrap_or(native.started_at_ms) as i64,
            );
            records.insert(record.run_code.clone(), record);
        }
        Ok(records.into_values().collect())
    }
}
impl AgentRunHistoryRepository for SqliteAgentRunRepository {
    fn list_runs(&mut self) -> Result<Vec<AgentRunHistoryRecord>, AgentRunHistoryError> {
        // One query across all sessions instead of per-session event scans.
        blocking(async { self.project_records("", Vec::new()).await })
    }
    fn get_run(&mut self, code: &str) -> Result<AgentRunHistoryRecord, AgentRunHistoryError> {
        blocking(async {
            let records = self
                .project_records("AND e.run_id=?1", vec![code.to_owned()])
                .await?;
            records
                .into_iter()
                .find(|record| record.run_code == code)
                .ok_or(AgentRunHistoryError::RunNotFound)
        })
    }
    fn next_id(&mut self) -> Result<i64, AgentRunHistoryError> {
        use crate::shared::id::SnowflakeIdProvider;
        crate::shared::id::SnowflakedSnowflakeIdProvider::default()
            .next_id()
            .map(|id| id.value())
            .map_err(|_| AgentRunHistoryError::StorageFailure)
    }
    fn create_run(&mut self, record: &AgentRunHistoryRecord) -> Result<(), AgentRunHistoryError> {
        blocking(async {
            let framework = self
                .host
                .storage_framework()
                .await
                .map_err(|_| AgentRunHistoryError::StorageFailure)?;
            let id = fw::RunId::parse_str(&record.run_code)
                .map_err(|_| AgentRunHistoryError::StorageFailure)?;
            let runs = framework
                .runs()
                .map_err(|_| AgentRunHistoryError::StorageFailure)?;
            let run = match runs.get(id).await {
                Ok(run) => run,
                Err(fw::StateError::NotFound) => return Ok(()),
                Err(_) => return Err(AgentRunHistoryError::StorageFailure),
            };
            let mut event = fw::SessionEvent::new(
                run.session_id,
                "tron.run-display",
                serde_json::to_value(record).map_err(|_| AgentRunHistoryError::StorageFailure)?,
            );
            event.run_id = Some(id);
            framework
                .sessions()
                .map_err(|_| AgentRunHistoryError::StorageFailure)?
                .store
                .append_event(event)
                .await
                .map_err(|_| AgentRunHistoryError::StorageFailure)?;
            Ok(())
        })
    }
}
