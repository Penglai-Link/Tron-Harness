//! Profile-owned SDK summary cache; initialization shares the host admission lock.
use super::FrameworkHost;
use eliteforge_agent_core as fw;
use eliteforge_agent_official_plugin_storage_local::SqliteSummaryStore;
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
    pub(super) async fn summary_store(&self) -> Result<Arc<SqliteSummaryStore>, fw::AgentError> {
        self.ensure_open()
            .map_err(fw::AgentError::InvalidConfiguration)?;
        self.0
            .summaries
            .get_or_try_init(|| async {
                let path = self.0.path.with_extension("context-summaries.sqlite");
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| fw::AgentError::Run(e.to_string()))?;
                }
                Ok(Arc::new(
                    SqliteSummaryStore::open(path, Default::default()).await?,
                ))
            })
            .await
            .cloned()
    }
}
