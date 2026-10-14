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

impl FrameworkHost {
    pub(crate) async fn bind_collaboration_session(
        &self,
        context: &AgentRunContext,
        origin_session: &str,
    ) -> Result<(), fw::AgentError> {
        let id = context.conversation_code.as_deref().ok_or_else(|| {
            fw::AgentError::InvalidConfiguration("collaboration session is missing".into())
        })?;
        let scope = NativeCollaborationScope {
            workspace_code: context.workspace_code.clone(),
            user_code: context.user_code.clone(),
            owner_kind: context
                .collaboration_owner
                .as_ref()
                .map(|owner| owner.kind.clone())
                .unwrap_or_else(|| "USER".into()),
            owner_code: context
                .collaboration_owner
                .as_ref()
                .map(|owner| owner.code.clone())
                .unwrap_or_else(|| context.user_code.clone().unwrap_or_else(|| "system".into())),
            agent_code: context.agent_code.clone(),
            origin_session_code: context
                .collaboration_owner
                .as_ref()
                .and_then(|owner| owner.origin_session_code.clone())
                .unwrap_or_else(|| origin_session.into()),
        };
        let framework = self.storage_framework().await?;
        let sessions = framework.sessions()?;
        self.shared_database()
            .await?
            .transaction(async {
                let (mut record, existing) = match sessions.get(id).await {
                    Ok(record) => (record, true),
                    Err(fw::StateError::NotFound) => {
                        let mut record = fw::SessionRecord::new("");
                        record.id = id.into();
                        (record, false)
                    }
                    Err(error) => return Err(error),
                };
                if let Some(saved) = NativeCollaborationScope::from_session(&record)? {
                    return if saved.matches(context) {
                        Ok(())
                    } else {
                        Err(fw::StateError::NotFound)
                    };
                }
                if record.metadata.is_null() {
                    record.metadata = serde_json::json!({});
                }
                record
                    .metadata
                    .as_object_mut()
                    .ok_or_else(|| {
                        fw::StateError::Invalid("session metadata must be an object".into())
                    })?
                    .insert(
                        METADATA_KEY.into(),
                        serde_json::to_value(&scope)
                            .map_err(|error| fw::StateError::Backend(error.to_string()))?,
                    );
                if existing {
                    let revision = record.revision;
                    sessions.update(record, revision).await?;
                } else {
                    sessions.create(record).await?;
                }
                Ok(())
            })
            .await?;
        Ok(())
    }
}
