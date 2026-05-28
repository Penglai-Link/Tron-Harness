use std::{fs, path::Path};

use rusqlite::{Connection, params};

use crate::{
    application::model::ModelRepository,
    domain::model::{
        Model, ModelCapability, ModelError, ModelProtocol, ModelScopeKind, ModelSourceKind,
        ModelType, ModelVerifyStatus, PlatformDefaultModelAssignment,
        WorkspaceDefaultModelAssignment, default_model_capabilities_for_type,
    },
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
    MODEL_REMOVE_EXPLICIT_PLATFORM_RESTORE_MIGRATION, MODEL_REMOVE_SEEDED_DATA_MIGRATION,
    MODELS_MIGRATION,
};

mod repository_impl;
mod schema;
mod storage;

use schema::{column_exists, ensure_database_parent};
use storage::storage_error;

const MODEL_ID_SEED: i64 = 734_600_000_000_000_000;

pub struct SqliteModelRepository {
    connection: Connection,
}

impl SqliteModelRepository {
    pub fn connect(database_url: &str) -> Result<Self, ModelError> {
        ensure_database_parent(database_url)?;
        let connection = if database_url == ":memory:" || database_url.starts_with("file:") {
            Connection::open_with_flags(
                database_url,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                    | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                    | rusqlite::OpenFlags::SQLITE_OPEN_URI,
            )
        } else {
            Connection::open(database_url)
        }
        .map_err(storage_error)?;

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
        self.connection
            .execute_batch(MODEL_REMOVE_EXPLICIT_PLATFORM_RESTORE_MIGRATION)
            .map_err(storage_error)?;
        self.connection
            .execute_batch(MODEL_REMOVE_SEEDED_DATA_MIGRATION)
            .map_err(storage_error)?;
        Ok(())
    }
}
