use super::FrameworkHost;
use eliteforge_agent_core as fw;
use eliteforge_agent_official_plugin_storage_local::{
    DeleteSession, DeleteSessions, SessionDeletionReport,
};

impl FrameworkHost {
    pub async fn delete_persisted_session(&self, session_id: &str) -> Result<bool, fw::AgentError> {
        self.ensure_open()
            .map_err(fw::AgentError::InvalidConfiguration)?;
        self.framework()
            .await?
            .execute(DeleteSession(session_id.to_owned()))
            .await
    }

    pub async fn delete_persisted_sessions(
        &self,
        session_ids: Vec<String>,
    ) -> Result<SessionDeletionReport, fw::AgentError> {
        self.ensure_open()
            .map_err(fw::AgentError::InvalidConfiguration)?;
        self.framework()
            .await?
            .execute(DeleteSessions(session_ids))
            .await
    }
}
