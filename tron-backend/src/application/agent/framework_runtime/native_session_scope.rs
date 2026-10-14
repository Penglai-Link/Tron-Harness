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

#[cfg(test)]
mod tests {
    use super::*;

    fn context(id: &str) -> AgentRunContext {
        serde_json::from_value(serde_json::json!({
            "agentCode":"reviewer", "runCode":fw::RunId::new_v4().to_string(),
            "conversationCode":id, "workspaceCode":"workspace", "userCode":"user",
            "locale":"en-US", "agentKind":"REACT",
            "model":{"modelCode":"fixture", "protocol":"CHAT_COMPLETIONS", "baseUrl":"http://127.0.0.1:1/v1", "apiKey":"synthetic", "modelName":"fixture"},
            "variables":[], "skills":[], "tools":[], "executionScope":{}, "limits":{"maxSteps":12,"maxToolCalls":12},
            "messages":[], "observations":[], "startedAt":"fixture"
        })).unwrap()
    }

    #[tokio::test]
    async fn new_collaboration_session_is_created_once_with_scope_and_origin() {
        let host = FrameworkHost::temporary(None).unwrap();
        let context = context("new-child");
        host.create_collaboration_session(&context, "parent")
            .await
            .unwrap();
        let framework = host.storage_framework().await.unwrap();
        let sessions = framework.sessions().unwrap();
        let original = sessions.get("new-child").await.unwrap();
        let scope = NativeCollaborationScope::from_session(&original)
            .unwrap()
            .unwrap();
        assert!(scope.matches(&context));
        assert_eq!(scope.origin_session_code, "parent");
        assert_eq!(original.revision, 0);
        assert!(
            host.create_collaboration_session(&context, "other-parent")
                .await
                .is_err()
        );
        host.validate_collaboration_session(&context).await.unwrap();
        let after = sessions.get("new-child").await.unwrap();
        assert_eq!(after.metadata, original.metadata);
        assert_eq!(after.revision, original.revision);
    }

    #[tokio::test]
    async fn continuation_does_not_create_missing_or_adopt_unbound_sessions() {
        let host = FrameworkHost::temporary(None).unwrap();
        let context = context("missing-child");
        assert!(host.validate_collaboration_session(&context).await.is_err());
        let framework = host.storage_framework().await.unwrap();
        let sessions = framework.sessions().unwrap();
        assert!(matches!(
            sessions.get("missing-child").await,
            Err(fw::StateError::NotFound)
        ));
        let mut record = fw::SessionRecord::new("unbound");
        record.id = "missing-child".into();
        sessions.create(record).await.unwrap();
        assert!(host.validate_collaboration_session(&context).await.is_err());
        assert!(
            NativeCollaborationScope::from_session(&sessions.get("missing-child").await.unwrap())
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn continuation_rejects_different_user_workspace_agent_owner_and_archived_session() {
        let host = FrameworkHost::temporary(None).unwrap();
        let mut original = context("existing-child");
        original.collaboration_owner = Some(crate::domain::agent::AgentCollaborationOwner {
            kind: "DUDE".into(),
            code: "owner".into(),
            origin_session_code: Some("parent".into()),
            task_code: None,
        });
        host.create_collaboration_session(&original, "parent")
            .await
            .unwrap();
        for changed in 0..4 {
            let mut caller = original.clone();
            match changed {
                0 => caller.user_code = Some("other".into()),
                1 => caller.workspace_code = "other".into(),
                2 => caller.agent_code = Some("other".into()),
                _ => caller.collaboration_owner.as_mut().unwrap().code = "other".into(),
            }
            assert!(host.validate_collaboration_session(&caller).await.is_err());
        }
        host.validate_collaboration_session(&original)
            .await
            .unwrap();
        let framework = host.storage_framework().await.unwrap();
        let sessions = framework.sessions().unwrap();
        let mut record = sessions.get("existing-child").await.unwrap();
        record.status = fw::SessionStatus::Archived;
        let revision = record.revision;
        sessions.update(record, revision).await.unwrap();
        assert!(
            host.validate_collaboration_session(&original)
                .await
                .is_err()
        );
    }
}
