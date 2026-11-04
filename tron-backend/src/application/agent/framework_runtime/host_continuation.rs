//! A profile-owned native continuation driver and its fenced SDK journal.
use super::FrameworkHost;
use eliteforge_agent_framework::Agent;
impl FrameworkHost {
    pub(crate) async fn continuation_agent(&self) -> Result<Agent, String> {
        let _admission = self.0.config_lock.lock().await;
        self.ensure_open()?;
        let agent = self
            .0
            .continuation_agent
            .get_or_try_init(|| async { self.build_driver(false).await.map_err(|e| e.to_string()) })
            .await?;
        Ok(agent.clone())
    }
}
