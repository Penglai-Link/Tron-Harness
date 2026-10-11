//! Durable Thread metadata and completion outbox. Conversations remain Framework-owned.
use super::*;
use crate::domain::agent::{AgentCollaborationOwner, AgentExecutionScope};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ThreadRecord {
    pub summary: fw::ThreadSummary,
    pub workspace_code: String,
    pub user_code: Option<String>,
    pub agent_code: String,
    pub owner: AgentCollaborationOwner,
    pub initiator_agent_code: Option<String>,
    pub origin_session_code: String,
    pub execution_scope: AgentExecutionScope,
    pub completion: Option<Completion>,
    pub closed: bool,
    pub feedback_pending: bool,
    pub revision: u64,
    pub run_admitted_at_ms: u64,
}
impl ThreadRecord {
    pub fn authorized(&self, context: &AgentRunContext) -> bool {
        let owner = owner(context);
        self.authorized_owner(
            &context.workspace_code,
            context.user_code.as_deref(),
            &owner,
        )
    }
    pub fn authorized_owner(
        &self,
        workspace: &str,
        user: Option<&str>,
        owner: &AgentCollaborationOwner,
    ) -> bool {
        self.workspace_code == workspace
            && self.user_code.as_deref() == user
            && self.owner.kind == owner.kind
            && self.owner.code == owner.code
    }
}
pub(super) fn owner(context: &AgentRunContext) -> AgentCollaborationOwner {
    context
        .collaboration_owner
        .clone()
        .unwrap_or_else(|| AgentCollaborationOwner {
            kind: "USER".into(),
            code: context.user_code.clone().unwrap_or_else(|| "system".into()),
            origin_session_code: context.conversation_code.clone(),
            task_code: context.execution_scope.task_code.clone(),
        })
}
fn state_error(error: impl std::fmt::Display) -> fw::StateError {
    fw::StateError::Backend(error.to_string())
}
impl FrameworkHost {
    pub(super) async fn thread_records(&self) -> Result<Vec<ThreadRecord>, fw::AgentError> {
        self.shared_database()
            .await?
            .call(|connection| {
                let mut query = connection
                    .prepare("SELECT record_json FROM collaboration_threads ORDER BY thread_code")
                    .map_err(state_error)?;
                let rows = query
                    .query_map([], |row| row.get::<_, String>(0))
                    .map_err(state_error)?;
                rows.map(|row| {
                    serde_json::from_str::<ThreadRecord>(&row.map_err(state_error)?)
                        .map_err(state_error)
                })
                .collect()
            })
            .await
            .map_err(Into::into)
    }
    pub(super) async fn thread_record(
        &self,
        id: &str,
    ) -> Result<Option<ThreadRecord>, fw::AgentError> {
        let id = id.to_owned();
        self.shared_database()
            .await?
            .call(move |connection| {
                let value = connection
                    .query_row(
                        "SELECT record_json FROM collaboration_threads WHERE thread_code=?1",
                        [id],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(state_error)?;
                value
                    .map(|value| serde_json::from_str::<ThreadRecord>(&value).map_err(state_error))
                    .transpose()
            })
            .await
            .map_err(Into::into)
    }
    pub(super) async fn persist_thread(
        &self,
        record: &mut ThreadRecord,
        create: bool,
    ) -> Result<(), fw::AgentError> {
        let mut next = record.clone();
        next.revision = if create {
            0
        } else {
            record
                .revision
                .checked_add(1)
                .ok_or_else(|| failure("thread revision overflow"))?
        };
        let value = serde_json::to_string(&next).map_err(failure)?;
        let id = next.summary.thread_id.clone();
        let scope = next.workspace_code.clone();
        let user = next.user_code.clone();
        let kind = next.owner.kind.clone();
        let code = next.owner.code.clone();
        let revision = record.revision;
        let feedback = if next.feedback_pending {
            next.completion.as_ref().and_then(|(status, result)| {
                next.summary.latest_run_id.map(|run_id| ThreadFeedback {
                    event_key: format!("thread:{}:{}:finished", next.summary.thread_id, run_id),
                    owner: next.owner.clone(),
                    thread_code: next.summary.thread_id.clone(),
                    origin_session_code: next.origin_session_code.clone(),
                    initiator_agent_code: next.initiator_agent_code.clone(),
                    status: status.clone(),
                    result: result.as_ref().ok().cloned(),
                    error: result.as_ref().err().cloned(),
                })
            })
        } else {
            None
        };
        let feedback = feedback
            .map(|feedback| {
                serde_json::to_string(&feedback)
                    .map(|value| (feedback.event_key, feedback.thread_code, value))
            })
            .transpose()
            .map_err(failure)?;
        let database = self.shared_database().await?;
        database.transaction(async {
        database.call(move |connection| {
            let changed = if create {
                connection.execute("INSERT INTO collaboration_threads(thread_code,workspace_code,user_code,owner_kind,owner_code,record_json) VALUES (?1,?2,?3,?4,?5,?6)", rusqlite::params![id,scope,user,kind,code,value])
            } else {
                connection.execute("UPDATE collaboration_threads SET record_json=?2,updated_at=CURRENT_TIMESTAMP WHERE thread_code=?1 AND json_extract(record_json,'$.revision')=?3", rusqlite::params![id,value,revision])
            }.map_err(state_error)?;
            if changed != 1 { return Err(fw::StateError::Conflict); }
            if let Some((event_key,thread_code,payload)) = feedback {
                connection.execute("INSERT OR IGNORE INTO collaboration_thread_feedback(event_key,thread_code,payload_json) VALUES (?1,?2,?3)",rusqlite::params![event_key,thread_code,payload]).map_err(state_error)?;
            }
            Ok(())
        }).await.map_err(state_error)
        }).await?;
        *record = next;
        Ok(())
    }
}
