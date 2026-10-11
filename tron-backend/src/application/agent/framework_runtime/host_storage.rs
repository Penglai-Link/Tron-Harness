use super::FrameworkHost;
impl FrameworkHost {
    /// The host owns one connection; Tron link writes join Framework transactions.
    pub async fn shared_database(
        &self,
    ) -> Result<
        eliteforge_agent_official_plugin_storage_local::SqliteDatabase,
        eliteforge_agent_core::StateError,
    > {
        self.0.database.get_or_try_init(|| async {
            if let Some(parent) = self.0.path.parent() { std::fs::create_dir_all(parent).map_err(|e| eliteforge_agent_core::StateError::Backend(e.to_string()))?; }
            crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(&self.0.path.to_string_lossy()).map_err(|e| eliteforge_agent_core::StateError::Backend(e.to_string()))
        }).await.map(|database| database.runtime())
    }

    pub async fn storage_framework(
        &self,
    ) -> Result<eliteforge_agent_framework::Framework, eliteforge_agent_core::AgentError> {
        self.framework().await
    }
}
