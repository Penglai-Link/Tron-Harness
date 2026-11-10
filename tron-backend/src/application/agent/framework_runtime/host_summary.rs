//! Profile-owned SDK summary cache; initialization shares the host admission lock.
use super::FrameworkHost;
use eliteforge_agent_core as fw;
use eliteforge_agent_official_plugin_storage_local::{SummaryStore, SummaryStoreService};
use std::sync::Arc;
impl FrameworkHost {
    pub fn bind_model_client(
        &self,
        client: Arc<dyn crate::application::agent::ModelClient>,
    ) -> Result<(), String> {
        let mut binding = self.0.model_client.write().unwrap();
        if self.0.closed.load(std::sync::atomic::Ordering::Acquire) || binding.is_some() {
            return Err("model client binding is closed or already installed".into());
        }
        *binding = Some(client);
        Ok(())
    }
    pub(crate) async fn summary_store(&self) -> Result<Arc<dyn SummaryStore>, fw::AgentError> {
        self.ensure_open()
            .map_err(fw::AgentError::InvalidConfiguration)?;
        self.storage_framework()
            .await?
            .services()?
            .get::<SummaryStoreService>()
            .map(|store| store.0.clone())
            .ok_or(fw::AgentError::MissingService("SummaryStore"))
    }
}
