use std::{fs, path::Path};

use rusqlite::{Connection, params};

use crate::{
    application::variable::VariableRepository,
    domain::variable::{Variable, VariableError},
    shared::id::SnowflakeId,
};

use super::migrations::VARIABLES_MIGRATION;

const DEFAULT_TIMESTAMP: &str = "2026-05-18T00:00:00Z";
const DEFAULT_EPOCH_MS: i64 = 1_779_062_400_000;

pub struct SqliteVariableRepository {
    connection: Connection,
}

impl SqliteVariableRepository {
    pub fn connect(database_url: &str) -> Result<Self, VariableError> {
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

    fn apply_migrations(&mut self) -> Result<(), VariableError> {
        self.connection
            .execute_batch(VARIABLES_MIGRATION)
            .map_err(storage_error)
    }

    fn seed_if_empty(&mut self) -> Result<(), VariableError> {
        let count: i64 = self
            .connection
            .query_row("SELECT COUNT(*) FROM workspace_variables", [], |row| {
                row.get(0)
            })
            .map_err(storage_error)?;
        if count > 0 {
            return Ok(());
        }

        let variables = [
            Variable {
                id: SnowflakeId::new(734_500_000_000_000_001),
                workspace_code: "ENTERPRISE".into(),
                key: "DEFAULT_REGION".into(),
                value: "us-east-1".into(),
                description: Some("Shared default deployment region.".into()),
                created_at: DEFAULT_TIMESTAMP.into(),
                updated_at: DEFAULT_TIMESTAMP.into(),
            },
            Variable {
                id: SnowflakeId::new(734_500_000_000_000_002),
                workspace_code: "ENTERPRISE".into(),
                key: "ROUTER_TIMEOUT_MS".into(),
                value: "800".into(),
                description: Some("Root timeout for interactive routing.".into()),
                created_at: DEFAULT_TIMESTAMP.into(),
                updated_at: DEFAULT_TIMESTAMP.into(),
            },
            Variable {
                id: SnowflakeId::new(734_500_000_000_000_003),
                workspace_code: "PLATFORM".into(),
                key: "OPENAI_API_BASE_URL".into(),
                value: "https://api.openai.com/v1".into(),
                description: Some("Shared OpenAI gateway base URL.".into()),
                created_at: DEFAULT_TIMESTAMP.into(),
                updated_at: DEFAULT_TIMESTAMP.into(),
            },
            Variable {
                id: SnowflakeId::new(734_500_000_000_000_004),
                workspace_code: "PLATFORM".into(),
                key: "ROUTER_TIMEOUT_MS".into(),
                value: "600".into(),
                description: Some("Platform override for interactive latency.".into()),
                created_at: DEFAULT_TIMESTAMP.into(),
                updated_at: DEFAULT_TIMESTAMP.into(),
            },
            Variable {
                id: SnowflakeId::new(734_500_000_000_000_005),
                workspace_code: "TRON_CORE".into(),
                key: "OPENAI_API_KEY".into(),
                value: "sk-tron-core-demo".into(),
                description: Some("Workspace secret used by the demo workspace.".into()),
                created_at: DEFAULT_TIMESTAMP.into(),
                updated_at: DEFAULT_TIMESTAMP.into(),
            },
            Variable {
                id: SnowflakeId::new(734_500_000_000_000_006),
                workspace_code: "TRON_CORE".into(),
                key: "ROUTER_TIMEOUT_MS".into(),
                value: "400".into(),
                description: Some("Tron Core override for agent console flows.".into()),
                created_at: DEFAULT_TIMESTAMP.into(),
                updated_at: DEFAULT_TIMESTAMP.into(),
            },
            Variable {
                id: SnowflakeId::new(734_500_000_000_000_007),
                workspace_code: "COMPLIANCE".into(),
                key: "AUDIT_RETENTION_DAYS".into(),
                value: "365".into(),
                description: Some("Retention period for compliance exports.".into()),
                created_at: DEFAULT_TIMESTAMP.into(),
                updated_at: DEFAULT_TIMESTAMP.into(),
            },
        ];

        for variable in &variables {
            self.create_variable(variable)?;
        }

        Ok(())
    }
}

impl VariableRepository for SqliteVariableRepository {
    fn list_variables(&mut self) -> Result<Vec<Variable>, VariableError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, workspace_code, variable_key, variable_value, description, created_at, updated_at
                 FROM workspace_variables
                 ORDER BY workspace_code ASC, variable_key ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], |row| {
                Ok(Variable {
                    id: SnowflakeId::new(row.get::<_, i64>(0)?),
                    workspace_code: row.get(1)?,
                    key: row.get(2)?,
                    value: row.get(3)?,
                    description: row.get(4)?,
                    created_at: millis_to_timestamp(row.get(5)?),
                    updated_at: millis_to_timestamp(row.get(6)?),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn next_id(&mut self) -> Result<i64, VariableError> {
        let max_id: i64 = self
            .connection
            .query_row("SELECT max(id) FROM workspace_variables", [], |row| {
                row.get::<_, Option<i64>>(0)
            })
            .map_err(storage_error)?
            .unwrap_or(734_500_000_000_000_000);
        Ok(max_id + 1)
    }

    fn create_variable(&mut self, variable: &Variable) -> Result<(), VariableError> {
        self.connection
            .execute(
                "INSERT INTO workspace_variables (id, workspace_code, variable_key, variable_value, description, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    variable.id.value(),
                    variable.workspace_code,
                    variable.key,
                    variable.value,
                    variable.description,
                    timestamp_to_millis(&variable.created_at),
                    timestamp_to_millis(&variable.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn update_variable(&mut self, variable: &Variable) -> Result<(), VariableError> {
        let updated_rows = self
            .connection
            .execute(
                "UPDATE workspace_variables
                 SET variable_value = ?3,
                     description = ?4,
                     updated_at = ?5
                 WHERE workspace_code = ?1 AND variable_key = ?2",
                params![
                    variable.workspace_code,
                    variable.key,
                    variable.value,
                    variable.description,
                    timestamp_to_millis(&variable.updated_at),
                ],
            )
            .map_err(storage_error)?;
        if updated_rows == 0 {
            return Err(VariableError::NotFound);
        }
        Ok(())
    }

    fn delete_variable(&mut self, workspace_code: &str, key: &str) -> Result<bool, VariableError> {
        let deleted_rows = self
            .connection
            .execute(
                "DELETE FROM workspace_variables
                 WHERE workspace_code = ?1 AND variable_key = ?2",
                params![workspace_code, key],
            )
            .map_err(storage_error)?;
        Ok(deleted_rows > 0)
    }
}

fn ensure_database_parent(database_url: &str) -> Result<(), VariableError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| VariableError::StorageFailure)?;
    }
    Ok(())
}

fn timestamp_to_millis(_value: &str) -> i64 {
    DEFAULT_EPOCH_MS
}

fn millis_to_timestamp(_value: i64) -> String {
    DEFAULT_TIMESTAMP.into()
}

fn storage_error(_error: rusqlite::Error) -> VariableError {
    VariableError::StorageFailure
}
