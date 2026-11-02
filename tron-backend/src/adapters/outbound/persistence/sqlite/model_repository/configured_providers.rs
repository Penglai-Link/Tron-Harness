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
    ) -> Result<(), ConfiguredProviderError> {
        let provider = provider.clone();
        self.connection.call(move |connection| {
            let transaction = connection.transaction().map_err(|_| ConfiguredProviderError::Storage)?;
            let existing = transaction.query_row("SELECT updated_at FROM configured_model_providers WHERE provider_id = ?1", [&provider.provider_id], |row| row.get::<_, i64>(0)).optional().map_err(|_| ConfiguredProviderError::Storage)?;
            if existing != expected_updated_at { return Err(if expected_updated_at.is_none() { ConfiguredProviderError::AlreadyExists } else { ConfiguredProviderError::Conflict }); }
            let old_codes = linked_model_codes(&transaction, &provider.provider_id)?;
            let configuration = serde_json::to_string(&provider).map_err(|_| ConfiguredProviderError::Storage)?;
            transaction.execute("INSERT INTO configured_model_providers(provider_id, configuration_json, updated_at) VALUES (?1,?2,?3) ON CONFLICT(provider_id) DO UPDATE SET configuration_json=excluded.configuration_json, updated_at=excluded.updated_at", params![provider.provider_id, configuration, provider.updated_at]).map_err(|_| ConfiguredProviderError::Storage)?;
            transaction.execute("DELETE FROM configured_provider_models WHERE provider_id=?1", [&provider.provider_id]).map_err(|_| ConfiguredProviderError::Storage)?;
            for model in &provider.models {
                let code = model.model_code.as_deref().ok_or(ConfiguredProviderError::InvalidModels)?;
                transaction.execute("INSERT INTO configured_provider_models(provider_id,model_code) VALUES (?1,?2)", params![provider.provider_id,code]).map_err(|_| ConfiguredProviderError::Storage)?;
                if !provider.enabled || !model.enabled { clear_defaults(&transaction, code)?; }
            }
            for code in old_codes.iter().filter(|code| !provider.models.iter().any(|model| model.model_code.as_deref() == Some(code.as_str()))) { clear_defaults(&transaction, code)?; }
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
                    clear_defaults(&transaction, &code)?;
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
    connection
        .execute(
            "DELETE FROM default_model_selections WHERE model_code=?1 AND provider_code IS NULL",
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
