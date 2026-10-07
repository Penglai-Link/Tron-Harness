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
    async fn records(
        &self,
        session: &str,
    ) -> Result<Vec<AgentRunHistoryRecord>, AgentRunHistoryError> {
        let framework = self
            .host
            .storage_framework()
            .await
            .map_err(|_| AgentRunHistoryError::StorageFailure)?;
        let events = framework
            .events()
            .map_err(|_| AgentRunHistoryError::StorageFailure)?;
        let runs = framework
            .runs()
            .map_err(|_| AgentRunHistoryError::StorageFailure)?;
        let mut after = 0;
        let mut records = std::collections::BTreeMap::new();
        loop {
            let page = events
                .list(session, after, 1000)
                .await
                .map_err(|_| AgentRunHistoryError::StorageFailure)?;
            if page.is_empty() {
                break;
            }
            after = page.last().unwrap().sequence;
            for event in page {
                if event.kind == "tron.run-display" {
                    let mut record: AgentRunHistoryRecord =
                        serde_json::from_value(event.payload)
                            .map_err(|_| AgentRunHistoryError::StorageFailure)?;
                    let id = fw::RunId::parse_str(&record.run_code)
                        .map_err(|_| AgentRunHistoryError::StorageFailure)?;
                    let native = runs
                        .get(id)
                        .await
                        .map_err(|_| AgentRunHistoryError::StorageFailure)?;
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
            }
        }
        Ok(records.into_values().collect())
    }
}
impl AgentRunHistoryRepository for SqliteAgentRunRepository {
    fn list_runs(&mut self) -> Result<Vec<AgentRunHistoryRecord>, AgentRunHistoryError> {
        blocking(async {
            let framework = self
                .host
                .storage_framework()
                .await
                .map_err(|_| AgentRunHistoryError::StorageFailure)?;
            let sessions = framework
                .sessions()
                .map_err(|_| AgentRunHistoryError::StorageFailure)?;
            let mut after = None;
            let mut result = vec![];
            loop {
                let page = sessions
                    .list(after.as_deref(), 1000)
                    .await
                    .map_err(|_| AgentRunHistoryError::StorageFailure)?;
                if page.is_empty() {
                    break;
                }
                after = page.last().map(|s| s.id.clone());
                for session in page {
                    result.extend(self.records(&session.id).await?);
                }
            }
            Ok(result)
        })
    }
    fn get_run(&mut self, code: &str) -> Result<AgentRunHistoryRecord, AgentRunHistoryError> {
        blocking(async {
            let framework = self
                .host
                .storage_framework()
                .await
                .map_err(|_| AgentRunHistoryError::StorageFailure)?;
            let id = fw::RunId::parse_str(code).map_err(|_| AgentRunHistoryError::RunNotFound)?;
            let run = framework
                .runs()
                .map_err(|_| AgentRunHistoryError::StorageFailure)?
                .get(id)
                .await
                .map_err(|e| {
                    if matches!(e, fw::StateError::NotFound) {
                        AgentRunHistoryError::RunNotFound
                    } else {
                        AgentRunHistoryError::StorageFailure
                    }
                })?;
            self.records(&run.session_id)
                .await?
                .into_iter()
                .find(|r| r.run_code == code)
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
