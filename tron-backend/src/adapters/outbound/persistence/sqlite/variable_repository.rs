use std::{fs, path::Path};

use rusqlite::{Connection, params};

use crate::{
    application::variable::VariableRepository,
    domain::variable::{PlatformVariable, Variable, VariableError},
    shared::{
        id::SnowflakeId,
        time::{timestamp_from_millis, timestamp_to_millis},
    },
};

use super::migrations::{PLATFORM_VARIABLES_MIGRATION, VARIABLES_MIGRATION};

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
        Ok(repository)
    }

    fn apply_migrations(&mut self) -> Result<(), VariableError> {
        self.connection
            .execute_batch(VARIABLES_MIGRATION)
            .map_err(storage_error)?;
        self.connection
            .execute_batch(PLATFORM_VARIABLES_MIGRATION)
            .map_err(storage_error)
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
                    created_at: timestamp_from_millis(row.get(5)?),
                    updated_at: timestamp_from_millis(row.get(6)?),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn list_platform_variables(&mut self) -> Result<Vec<PlatformVariable>, VariableError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, variable_key, variable_value, description, created_at, updated_at
                 FROM platform_variables
                 ORDER BY variable_key ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], |row| {
                Ok(PlatformVariable {
                    id: SnowflakeId::new(row.get::<_, i64>(0)?),
                    key: row.get(1)?,
                    value: row.get(2)?,
                    description: row.get(3)?,
                    created_at: timestamp_from_millis(row.get(4)?),
                    updated_at: timestamp_from_millis(row.get(5)?),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn next_id(&mut self) -> Result<i64, VariableError> {
        let max_id: i64 = self
            .connection
            .query_row("SELECT max(id) FROM (SELECT id FROM workspace_variables UNION ALL SELECT id FROM platform_variables)", [], |row| {
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

    fn create_platform_variable(
        &mut self,
        variable: &PlatformVariable,
    ) -> Result<(), VariableError> {
        self.connection
            .execute(
                "INSERT INTO platform_variables (id, variable_key, variable_value, description, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    variable.id.value(),
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

    fn update_platform_variable(
        &mut self,
        variable: &PlatformVariable,
    ) -> Result<(), VariableError> {
        let updated_rows = self
            .connection
            .execute(
                "UPDATE platform_variables
                 SET variable_value = ?2,
                     description = ?3,
                     updated_at = ?4
                 WHERE variable_key = ?1",
                params![
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

    fn delete_platform_variable(&mut self, key: &str) -> Result<bool, VariableError> {
        let deleted_rows = self
            .connection
            .execute(
                "DELETE FROM platform_variables WHERE variable_key = ?1",
                params![key],
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

fn storage_error(_error: rusqlite::Error) -> VariableError {
    VariableError::StorageFailure
}
