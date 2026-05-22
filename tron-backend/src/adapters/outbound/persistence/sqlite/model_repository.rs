use std::{fs, path::Path};

use rusqlite::{Connection, params};

use crate::{
    application::model::ModelRepository,
    domain::model::{
        ModelError, ModelVendor, PlatformModel, PlatformModelStatus, WorkspaceModelRemoval,
        platform_status_from_storage, platform_status_to_storage, vendor_from_storage,
        vendor_to_storage,
    },
    shared::id::SnowflakeId,
};

use super::migrations::MODELS_MIGRATION;

const DEFAULT_TIMESTAMP: &str = "2026-05-22T00:00:00Z";
const DEFAULT_EPOCH_MS: i64 = 1_779_408_000_000;

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
        repository.seed_if_empty()?;
        Ok(repository)
    }

    fn apply_migrations(&mut self) -> Result<(), ModelError> {
        self.connection
            .execute_batch(MODELS_MIGRATION)
            .map_err(storage_error)
    }

    fn seed_if_empty(&mut self) -> Result<(), ModelError> {
        let count: i64 = self
            .connection
            .query_row("SELECT COUNT(*) FROM platform_models", [], |row| row.get(0))
            .map_err(storage_error)?;
        if count > 0 {
            return Ok(());
        }

        let models = [
            PlatformModel {
                id: SnowflakeId::new(734_600_000_000_000_001),
                model_code: "gpt-4-turbo".into(),
                name: "gpt-4-turbo".into(),
                purpose: "Default Reasoning".into(),
                vendor: ModelVendor::OpenAI,
                context_window: "128k tokens".into(),
                max_output: "4,096 tokens".into(),
                training_data: "Dec 2023".into(),
                speed: "~42 tk/s".into(),
                price: "$10.00 / 1M Input".into(),
                status: PlatformModelStatus::Enabled,
                created_at: DEFAULT_TIMESTAMP.into(),
                updated_at: DEFAULT_TIMESTAMP.into(),
            },
            PlatformModel {
                id: SnowflakeId::new(734_600_000_000_000_002),
                model_code: "claude-3-opus-20240229".into(),
                name: "claude-3-opus-20240229".into(),
                purpose: "Complex Tasks".into(),
                vendor: ModelVendor::Anthropic,
                context_window: "200k tokens".into(),
                max_output: "4,096 tokens".into(),
                training_data: "Aug 2023".into(),
                speed: "~31 tk/s".into(),
                price: "$15.00 / 1M Input".into(),
                status: PlatformModelStatus::Enabled,
                created_at: DEFAULT_TIMESTAMP.into(),
                updated_at: DEFAULT_TIMESTAMP.into(),
            },
            PlatformModel {
                id: SnowflakeId::new(734_600_000_000_000_003),
                model_code: "gemini-1.5-pro".into(),
                name: "gemini-1.5-pro".into(),
                purpose: "Multimodal Review".into(),
                vendor: ModelVendor::Google,
                context_window: "2M tokens".into(),
                max_output: "8,192 tokens".into(),
                training_data: "Apr 2024".into(),
                speed: "~36 tk/s".into(),
                price: "$7.00 / 1M Input".into(),
                status: PlatformModelStatus::Enabled,
                created_at: DEFAULT_TIMESTAMP.into(),
                updated_at: DEFAULT_TIMESTAMP.into(),
            },
            PlatformModel {
                id: SnowflakeId::new(734_600_000_000_000_004),
                model_code: "gpt-3.5-turbo".into(),
                name: "gpt-3.5-turbo".into(),
                purpose: "Legacy Fast".into(),
                vendor: ModelVendor::OpenAI,
                context_window: "16k tokens".into(),
                max_output: "2,048 tokens".into(),
                training_data: "Sep 2021".into(),
                speed: "~91 tk/s".into(),
                price: "$0.50 / 1M Input".into(),
                status: PlatformModelStatus::Disabled,
                created_at: DEFAULT_TIMESTAMP.into(),
                updated_at: DEFAULT_TIMESTAMP.into(),
            },
        ];

        for model in &models {
            self.connection
                .execute(
                    "INSERT INTO platform_models
                     (id, model_code, model_name, purpose, vendor, context_window, max_output, training_data, speed, price, status, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    params![
                        model.id.value(),
                        model.model_code,
                        model.name,
                        model.purpose,
                        vendor_to_storage(model.vendor),
                        model.context_window,
                        model.max_output,
                        model.training_data,
                        model.speed,
                        model.price,
                        platform_status_to_storage(model.status),
                        timestamp_to_millis(&model.created_at),
                        timestamp_to_millis(&model.updated_at),
                    ],
                )
                .map_err(storage_error)?;
        }

        Ok(())
    }
}

impl ModelRepository for SqliteModelRepository {
    fn list_platform_models(&mut self) -> Result<Vec<PlatformModel>, ModelError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, model_code, model_name, purpose, vendor, context_window, max_output, training_data, speed, price, status, created_at, updated_at
                 FROM platform_models
                 ORDER BY model_name ASC, model_code ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], |row| {
                Ok(PlatformModel {
                    id: SnowflakeId::new(row.get::<_, i64>(0)?),
                    model_code: row.get(1)?,
                    name: row.get(2)?,
                    purpose: row.get(3)?,
                    vendor: vendor_from_storage(&row.get::<_, String>(4)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    context_window: row.get(5)?,
                    max_output: row.get(6)?,
                    training_data: row.get(7)?,
                    speed: row.get(8)?,
                    price: row.get(9)?,
                    status: platform_status_from_storage(&row.get::<_, String>(10)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    created_at: millis_to_timestamp(row.get(11)?),
                    updated_at: millis_to_timestamp(row.get(12)?),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn list_workspace_model_removals(
        &mut self,
        workspace_code: &str,
    ) -> Result<Vec<WorkspaceModelRemoval>, ModelError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, workspace_code, model_code, created_at
                 FROM workspace_model_removals
                 WHERE workspace_code = ?1
                 ORDER BY model_code ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([workspace_code], |row| {
                Ok(WorkspaceModelRemoval {
                    id: SnowflakeId::new(row.get::<_, i64>(0)?),
                    workspace_code: row.get(1)?,
                    model_code: row.get(2)?,
                    removed_at: millis_to_timestamp(row.get(3)?),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn next_id(&mut self) -> Result<i64, ModelError> {
        let max_id: i64 = self
            .connection
            .query_row(
                "SELECT max(id) FROM (
                    SELECT id FROM platform_models
                    UNION ALL SELECT id FROM workspace_model_removals
                )",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(storage_error)?
            .unwrap_or(734_600_000_000_000_000);
        Ok(max_id + 1)
    }

    fn create_workspace_model_removal(
        &mut self,
        removal: &WorkspaceModelRemoval,
    ) -> Result<(), ModelError> {
        self.connection
            .execute(
                "INSERT OR IGNORE INTO workspace_model_removals (id, workspace_code, model_code, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    removal.id.value(),
                    removal.workspace_code,
                    removal.model_code,
                    timestamp_to_millis(&removal.removed_at),
                    timestamp_to_millis(&removal.removed_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn delete_workspace_model_removal(
        &mut self,
        workspace_code: &str,
        model_code: &str,
    ) -> Result<bool, ModelError> {
        let deleted_rows = self
            .connection
            .execute(
                "DELETE FROM workspace_model_removals
                 WHERE workspace_code = ?1 AND model_code = ?2",
                params![workspace_code, model_code],
            )
            .map_err(storage_error)?;
        Ok(deleted_rows > 0)
    }
}

fn ensure_database_parent(database_url: &str) -> Result<(), ModelError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| ModelError::StorageFailure)?;
    }
    Ok(())
}

fn timestamp_to_millis(_value: &str) -> i64 {
    DEFAULT_EPOCH_MS
}

fn millis_to_timestamp(_value: i64) -> String {
    DEFAULT_TIMESTAMP.into()
}

fn storage_error(_error: rusqlite::Error) -> ModelError {
    ModelError::StorageFailure
}
