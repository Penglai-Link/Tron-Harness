use super::*;
use crate::domain::model::{ConfiguredModelProvider, ConfiguredProviderError};
use rusqlite::OptionalExtension;

impl SqliteModelRepository {
    pub(super) fn read_configured_providers(
        &self,
    ) -> Result<Vec<ConfiguredModelProvider>, ConfiguredProviderError> {
        self.connection.call(
            |connection| read_providers(connection),
            || ConfiguredProviderError::Storage,
        )
    }

    pub(super) fn write_configured_provider(
        &self,
        provider: &ConfiguredModelProvider,
        expected_updated_at: Option<i64>,
        models: &[Model],
    ) -> Result<(), ConfiguredProviderError> {
        let provider = provider.clone();
        let models = models.to_vec();
        self.connection.call(move |connection| {
            let transaction = connection.transaction().map_err(|_| ConfiguredProviderError::Storage)?;
            let existing = transaction.query_row("SELECT updated_at FROM configured_model_providers WHERE provider_id = ?1", [&provider.provider_id], |row| row.get::<_, i64>(0)).optional().map_err(|_| ConfiguredProviderError::Storage)?;
            if existing != expected_updated_at { return Err(if expected_updated_at.is_none() { ConfiguredProviderError::AlreadyExists } else { ConfiguredProviderError::Conflict }); }
            let old_codes = linked_model_codes(&transaction, &provider.provider_id)?;
            let configuration = serde_json::to_string(&provider).map_err(|_| ConfiguredProviderError::Storage)?;
            transaction.execute("INSERT INTO configured_model_providers(provider_id, configuration_json, updated_at) VALUES (?1,?2,?3) ON CONFLICT(provider_id) DO UPDATE SET configuration_json=excluded.configuration_json, updated_at=excluded.updated_at", params![provider.provider_id, configuration, provider.updated_at]).map_err(|_| ConfiguredProviderError::Storage)?;
            for model in &models {
                let mut store = SqliteModelRepositorySql { connection: &transaction };
                if old_codes.contains(&model.model_code) { store.update_model(model) } else { store.create_model(model) }.map_err(|_| ConfiguredProviderError::Storage)?;
                transaction.execute("INSERT OR IGNORE INTO configured_provider_models(provider_id,model_code) VALUES (?1,?2)", params![provider.provider_id, model.model_code]).map_err(|_| ConfiguredProviderError::Storage)?;
                if !provider.enabled || !model.enabled { clear_defaults(&transaction, &model.model_code)?; }
            }
            for code in old_codes.iter().filter(|code| !models.iter().any(|model| &model.model_code == *code)) { remove_model(&transaction, code)?; }
            transaction.commit().map_err(|_| ConfiguredProviderError::Storage)
        }, || ConfiguredProviderError::Storage)
    }

    pub(super) fn remove_configured_provider(
        &self,
        provider_id: &str,
    ) -> Result<(), ConfiguredProviderError> {
        let provider_id = provider_id.to_owned();
        self.connection.call(
            move |connection| {
                let transaction = connection
                    .transaction()
                    .map_err(|_| ConfiguredProviderError::Storage)?;
                let codes = linked_model_codes(&transaction, &provider_id)?;
                for code in codes {
                    remove_model(&transaction, &code)?;
                }
                let removed = transaction
                    .execute(
                        "DELETE FROM configured_model_providers WHERE provider_id=?1",
                        [&provider_id],
                    )
                    .map_err(|_| ConfiguredProviderError::Storage)?;
                if removed == 0 {
                    return Err(ConfiguredProviderError::NotFound);
                }
                transaction
                    .commit()
                    .map_err(|_| ConfiguredProviderError::Storage)
            },
            || ConfiguredProviderError::Storage,
        )
    }
}
fn linked_model_codes(
    connection: &Connection,
    provider_id: &str,
) -> Result<Vec<String>, ConfiguredProviderError> {
    let mut statement = connection
        .prepare("SELECT model_code FROM configured_provider_models WHERE provider_id=?1")
        .map_err(|_| ConfiguredProviderError::Storage)?;
    statement
        .query_map([provider_id], |row| row.get(0))
        .map_err(|_| ConfiguredProviderError::Storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ConfiguredProviderError::Storage)
}
fn clear_defaults(connection: &Connection, code: &str) -> Result<(), ConfiguredProviderError> {
    for table in [
        "platform_default_models",
        "workspace_default_models",
        "default_model_selections",
    ] {
        connection.execute(&format!("DELETE FROM {table} WHERE model_code=?1 OR model_code IN (SELECT model_code FROM models WHERE source_model_code=?1)"), [code]).map_err(|_| ConfiguredProviderError::Storage)?;
    }
    Ok(())
}
fn remove_model(connection: &Connection, code: &str) -> Result<(), ConfiguredProviderError> {
    clear_defaults(connection, code)?;
    connection
        .execute(
            "DELETE FROM models WHERE source_model_code=?1 OR model_code=?1",
            [code],
        )
        .map_err(|_| ConfiguredProviderError::Storage)?;
    Ok(())
}

pub(super) fn read_providers(
    connection: &Connection,
) -> Result<Vec<ConfiguredModelProvider>, ConfiguredProviderError> {
    let mut statement = connection
        .prepare("SELECT configuration_json FROM configured_model_providers ORDER BY provider_id")
        .map_err(|_| ConfiguredProviderError::Storage)?;
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|_| ConfiguredProviderError::Storage)?
        .map(|row| {
            serde_json::from_str(&row.map_err(|_| ConfiguredProviderError::Storage)?)
                .map_err(|_| ConfiguredProviderError::Storage)
        })
        .collect()
}
