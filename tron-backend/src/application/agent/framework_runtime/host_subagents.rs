//! Product discoverability over native child runs. Execution and status remain SDK owned.
use super::*;
use serde::{Deserialize, Serialize};

const CHILD_INDEX: &str = "tron.subagents";

#[derive(Clone)]
pub(super) struct SubagentOwner(pub String);

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentRecord {
    pub run_id: fw::RunId,
    pub session_id: String,
    pub parent_run_id: fw::RunId,
    pub agent_code: String,
    pub agent_name: String,
    #[serde(default)]
    pub avatar_id: Option<String>,
    pub task: String,
    pub user_code: Option<String>,
}

pub struct SubagentView {
    pub record: SubagentRecord,
    pub run: fw::HistoryRun,
}

impl FrameworkHost {
    pub(super) async fn record_subagent(
        &self,
        owner: &str,
        record: SubagentRecord,
    ) -> Result<(), fw::AgentError> {
        let framework = self.storage_framework().await?;
        let sessions = framework.sessions()?;
        self.shared_database()
            .await?
            .transaction(async {
                let previous = sessions.store.read_context(owner, CHILD_INDEX).await?;
                let revision = previous.as_ref().map(|(revision, _)| *revision);
                let mut records: Vec<SubagentRecord> = previous
                    .map(|(_, value)| serde_json::from_value(value))
                    .transpose()
                    .map_err(|e| fw::StateError::Backend(e.to_string()))?
                    .unwrap_or_default();
                records.push(record);
                sessions
                    .store
                    .write_context(
                        owner,
                        CHILD_INDEX,
                        revision,
                        serde_json::to_value(records)
                            .map_err(|e| fw::StateError::Backend(e.to_string()))?,
                    )
                    .await?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    pub async fn subagents(
        &self,
        session: &str,
        user: &str,
    ) -> Result<Vec<SubagentView>, fw::StateError> {
        let framework = self
            .storage_framework()
            .await
            .map_err(|e| fw::StateError::Backend(e.to_string()))?;
        let sessions = framework
            .sessions()
            .map_err(|e| fw::StateError::Backend(e.to_string()))?;
        let history = framework
            .history()
            .map_err(|e| fw::StateError::Backend(e.to_string()))?;
        let records: Vec<SubagentRecord> = sessions
            .store
            .read_context(session, CHILD_INDEX)
            .await?
            .map(|(_, value)| serde_json::from_value(value))
            .transpose()
            .map_err(|e| fw::StateError::Backend(e.to_string()))?
            .unwrap_or_default();
        let records = records
            .into_iter()
            .filter(|record| record.user_code.as_deref() == Some(user))
            .collect::<Vec<_>>();
        let ids = records
            .iter()
            .map(|record| record.run_id)
            .collect::<Vec<_>>();
        let runs = history.runs(&ids).await?;
        Ok(records
            .into_iter()
            .zip(runs)
            .map(|(record, run)| SubagentView { record, run })
            .collect())
    }

    pub async fn subagent_history(
        &self,
        session: &str,
        id: fw::RunId,
        user: &str,
    ) -> Result<(SubagentView, fw::SessionHistory), fw::StateError> {
        let framework = self
            .storage_framework()
            .await
            .map_err(|e| fw::StateError::Backend(e.to_string()))?;
        let sessions = framework
            .sessions()
            .map_err(|e| fw::StateError::Backend(e.to_string()))?;
        let records: Vec<SubagentRecord> = sessions
            .store
            .read_context(session, CHILD_INDEX)
            .await?
            .map(|(_, value)| serde_json::from_value(value))
            .transpose()
            .map_err(|e| fw::StateError::Backend(e.to_string()))?
            .unwrap_or_default();
        let record = records
            .into_iter()
            .find(|record| record.run_id == id && record.user_code.as_deref() == Some(user))
            .ok_or(fw::StateError::NotFound)?;
        let service = framework
            .history()
            .map_err(|e| fw::StateError::Backend(e.to_string()))?;
        let run = service.run(id).await?;
        let history = service
            .read_session(
                &record.session_id,
                fw::HistoryQuery {
                    limit: 1,
                    cursor: None,
                },
            )
            .await?;
        let item = SubagentView { record, run };
        Ok((item, history))
    }
}
