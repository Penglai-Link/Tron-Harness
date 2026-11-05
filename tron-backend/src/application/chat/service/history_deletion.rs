use super::ChatService;
use crate::application::agent::FrameworkHost;
use eliteforge_agent_core as fw;

impl ChatService {
    pub async fn delete_history_turn(
        &self,
        host: &FrameworkHost,
        session: &str,
        root: fw::RunId,
        version: &str,
    ) -> Result<String, fw::StateError> {
        let run = fw::RunId::new_v4().to_string();
        let _reservation = self
            .active_runs
            .reserve(session, &run, "HISTORY_EDIT", false)
            .map_err(|_| fw::StateError::Busy)?;
        host.delete_history_turn(session, root, version).await
    }
}
