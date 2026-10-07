use std::{fs, path::Path};

use rusqlite::{Connection, params};

use crate::{
    application::model::ModelRepository,
    domain::model::{
        Model, ModelCapability, ModelError, ModelProtocol, ModelScopeKind, ModelSourceKind,
        ModelType, ModelVerifyStatus, PlatformDefaultModelAssignment,
        WorkspaceDefaultModelAssignment, default_model_capabilities_for_type,
    },
    domain::reasoning::ReasoningEffort,
    shared::{
        id::SnowflakeId,
        time::{
            option_timestamp_from_millis, option_timestamp_to_millis, timestamp_from_millis,
            timestamp_to_millis,
        },
    },
};

use super::migrations::{
    MODEL_CAPABILITIES_MIGRATION, MODEL_CONTEXT_WINDOW_TOKENS_MIGRATION,
    MODEL_DEFAULT_REASONING_EFFORT_MIGRATION, MODEL_REMOVE_EXPLICIT_PLATFORM_RESTORE_MIGRATION,
    MODEL_REMOVE_SEEDED_DATA_MIGRATION, MODEL_RESPONSES_STATEFUL_ENABLED_MIGRATION,
    MODEL_SUPPORTED_REASONING_EFFORTS_MIGRATION, MODELS_MIGRATION,
};

mod repository_impl;
mod schema;
mod storage;

use schema::{column_exists, ensure_database_parent};
use storage::storage_error;

const MODEL_ID_SEED: i64 = 734_600_000_000_000_000;

struct SqliteModelRepositorySql<'a> {
    connection: &'a mut Connection,
}
pub struct SqliteModelRepository {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}

impl<'a> SqliteModelRepositorySql<'a> {
    pub fn initialize(
        database_url: &str,
        connection: &'a mut Connection,
    ) -> Result<Self, ModelError> {
        ensure_database_parent(database_url)?;

        let mut repository = Self { connection };
        repository.apply_migrations()?;
        Ok(repository)
    }

    fn apply_migrations(&mut self) -> Result<(), ModelError> {
        self.connection
            .execute_batch(MODELS_MIGRATION)
            .map_err(storage_error)?;
        if !column_exists(&self.connection, "models", "model_capabilities")? {
            self.connection
                .execute_batch(MODEL_CAPABILITIES_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(&self.connection, "models", "context_window_tokens")? {
            self.connection
                .execute_batch(MODEL_CONTEXT_WINDOW_TOKENS_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(&self.connection, "models", "responses_stateful_enabled")? {
            self.connection
                .execute_batch(MODEL_RESPONSES_STATEFUL_ENABLED_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(&self.connection, "models", "supported_reasoning_efforts")? {
            self.connection
                .execute_batch(MODEL_SUPPORTED_REASONING_EFFORTS_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(&self.connection, "models", "default_reasoning_effort")? {
            self.connection
                .execute_batch(MODEL_DEFAULT_REASONING_EFFORT_MIGRATION)
                .map_err(storage_error)?;
        }
        self.connection
            .execute_batch(MODEL_REMOVE_EXPLICIT_PLATFORM_RESTORE_MIGRATION)
            .map_err(storage_error)?;
        self.connection
            .execute_batch(MODEL_REMOVE_SEEDED_DATA_MIGRATION)
            .map_err(storage_error)?;
        Ok(())
    }
}

impl SqliteModelRepository {
    pub fn connect(database_url: &str) -> Result<Self, ModelError> {
        let connection = crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(database_url).map_err(storage_error)?;
        let database_url = database_url.to_owned();
        connection.call(
            move |connection| {
                SqliteModelRepositorySql::initialize(&database_url, connection).map(|_| ())
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )?;
        Ok(Self { connection })
    }
}
