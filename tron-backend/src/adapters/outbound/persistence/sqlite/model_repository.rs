use crate::{
    application::model::ModelRepository,
    domain::model::{ConfiguredModelProvider, ConfiguredProviderError, ModelError},
};
use rusqlite::{Connection, params};
mod configured_providers;
pub struct SqliteModelRepository {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}
impl SqliteModelRepository {
    pub fn connect(database_url: &str) -> Result<Self, ModelError> {
        let connection = crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(database_url)
            .map_err(|_| ModelError::StorageFailure)?;
        Ok(Self { connection })
    }
}
impl ModelRepository for SqliteModelRepository {
    fn list_configured_providers(
        &mut self,
    ) -> Result<Vec<ConfiguredModelProvider>, ConfiguredProviderError> {
        self.read_configured_providers()
    }
    fn save_configured_provider(
        &mut self,
        provider: &ConfiguredModelProvider,
        expected_updated_at: Option<i64>,
    ) -> Result<(), ConfiguredProviderError> {
        self.write_configured_provider(provider, expected_updated_at)
    }
    fn delete_configured_provider(
        &mut self,
        provider_id: &str,
    ) -> Result<(), ConfiguredProviderError> {
        self.remove_configured_provider(provider_id)
    }
}
