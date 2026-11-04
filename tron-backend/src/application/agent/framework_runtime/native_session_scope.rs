//! Trusted ownership of native collaboration sessions, stored before execution.
use super::FrameworkHost;
use crate::domain::agent::AgentRunContext;
use eliteforge_agent_core as fw;
use serde::{Deserialize, Serialize};

const METADATA_KEY: &str = "tronCollaborationScope";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeCollaborationScope {
    pub workspace_code: String,
    pub user_code: Option<String>,
    pub owner_kind: String,
    pub owner_code: String,
    pub agent_code: Option<String>,
    pub origin_session_code: String,
}

impl NativeCollaborationScope {
    pub(crate) fn from_session(
        session: &fw::SessionRecord,
    ) -> Result<Option<Self>, fw::StateError> {
        session
            .metadata
            .get(METADATA_KEY)
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| fw::StateError::Backend(error.to_string()))
    }

    pub(crate) fn matches(&self, context: &AgentRunContext) -> bool {
        let (kind, code) = context
            .collaboration_owner
            .as_ref()
            .map(|owner| (owner.kind.as_str(), owner.code.as_str()))
            .unwrap_or(("USER", context.user_code.as_deref().unwrap_or("system")));
        self.workspace_code == context.workspace_code
            && self.user_code == context.user_code
            && self.owner_kind == kind
            && self.owner_code == code
            && self.agent_code == context.agent_code
    }
}

impl NativeCollaborationScope {
    fn for_context(context: &AgentRunContext, origin_session: &str) -> Self {
        let (kind, code) = context
            .collaboration_owner
            .as_ref()
            .map(|owner| (owner.kind.clone(), owner.code.clone()))
            .unwrap_or_else(|| {
                (
                    "USER".into(),
                    context.user_code.clone().unwrap_or_else(|| "system".into()),
                )
            });
        Self {
            workspace_code: context.workspace_code.clone(),
            user_code: context.user_code.clone(),
            owner_kind: kind,
            owner_code: code,
            agent_code: context.agent_code.clone(),
            origin_session_code: context
                .collaboration_owner
                .as_ref()
                .and_then(|owner| owner.origin_session_code.clone())
                .unwrap_or_else(|| origin_session.into()),
        }
    }
}

fn session_id(context: &AgentRunContext) -> Result<&str, fw::AgentError> {
    context.conversation_code.as_deref().ok_or_else(|| {
        fw::AgentError::InvalidConfiguration("collaboration session is missing".into())
    })
}

impl FrameworkHost {
    /// A fresh delegation owns a fresh identity. Never query, adopt or update an
    /// existing session; duplicate identities are rejected by the storage layer.
    pub(crate) async fn create_collaboration_session(
        &self,
        context: &AgentRunContext,
        origin_session: &str,
    ) -> Result<(), fw::AgentError> {
        let mut record = fw::SessionRecord::new("");
        record.id = session_id(context)?.into();
        record.metadata = serde_json::json!({
            METADATA_KEY: NativeCollaborationScope::for_context(context, origin_session),
        });
        self.storage_framework()
            .await?
            .sessions()?
            .create(record)
            .await?;
        Ok(())
    }

    /// Continuation requires an existing, active session with the same trusted
    /// binding. It must never create a replacement or change ownership metadata.
    pub(crate) async fn validate_collaboration_session(
        &self,
        context: &AgentRunContext,
    ) -> Result<(), fw::AgentError> {
        let record = self
            .storage_framework()
            .await?
            .sessions()?
            .get(session_id(context)?)
            .await?;
        if !NativeCollaborationScope::from_session(&record)?
            .is_some_and(|scope| scope.matches(context))
        {
            return Err(fw::StateError::NotFound.into());
        }
        if record.status != fw::SessionStatus::Active {
            return Err(fw::StateError::Conflict.into());
        }
        Ok(())
    }
}
