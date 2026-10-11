use rusqlite::{Connection, params};

use crate::{
    application::variable::VariableRepository,
    domain::variable::{PlatformVariable, Variable, VariableError},
    shared::{
        id::SnowflakeId,
        time::{timestamp_from_millis, timestamp_to_millis},
    },
};

struct SqliteVariableRepositorySql<'a> {
    connection: &'a mut Connection,
}
pub struct SqliteVariableRepository {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}

impl<'a> SqliteVariableRepositorySql<'a> {
    pub fn initialize(
        _database_url: &str,
        connection: &'a mut Connection,
    ) -> Result<Self, VariableError> {
        crate::adapters::outbound::persistence::sqlite::schema::initialize_business(connection)
            .map_err(storage_error)?;
        Ok(Self { connection })
    }
}

impl VariableRepository for SqliteVariableRepositorySql<'_> {
    fn list_variables(&mut self) -> Result<Vec<Variable>, VariableError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, workspace_code, variable_key, variable_value, description,
                        is_sensitive, value_nonce, value_tag, value_algorithm, value_key_version,
                        created_at, updated_at
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
                    is_sensitive: int_to_bool(row.get(5)?),
                    value_nonce: row.get(6)?,
                    value_tag: row.get(7)?,
                    value_algorithm: row.get(8)?,
                    value_key_version: row.get(9)?,
                    created_at: timestamp_from_millis(row.get(10)?),
                    updated_at: timestamp_from_millis(row.get(11)?),
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
                "SELECT id, variable_key, variable_value, description,
                        is_sensitive, value_nonce, value_tag, value_algorithm, value_key_version,
                        created_at, updated_at
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
                    is_sensitive: int_to_bool(row.get(4)?),
                    value_nonce: row.get(5)?,
                    value_tag: row.get(6)?,
                    value_algorithm: row.get(7)?,
                    value_key_version: row.get(8)?,
                    created_at: timestamp_from_millis(row.get(9)?),
                    updated_at: timestamp_from_millis(row.get(10)?),
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
                "INSERT INTO workspace_variables (
                     id, workspace_code, variable_key, variable_value, description,
                     is_sensitive, value_nonce, value_tag, value_algorithm, value_key_version,
                     created_at, updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    variable.id.value(),
                    variable.workspace_code,
                    variable.key,
                    variable.value,
                    variable.description,
                    bool_to_int(variable.is_sensitive),
                    variable.value_nonce,
                    variable.value_tag,
                    variable.value_algorithm,
                    variable.value_key_version,
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
                "INSERT INTO platform_variables (
                     id, variable_key, variable_value, description,
                     is_sensitive, value_nonce, value_tag, value_algorithm, value_key_version,
                     created_at, updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    variable.id.value(),
                    variable.key,
                    variable.value,
                    variable.description,
                    bool_to_int(variable.is_sensitive),
                    variable.value_nonce,
                    variable.value_tag,
                    variable.value_algorithm,
                    variable.value_key_version,
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
                     is_sensitive = ?5,
                     value_nonce = ?6,
                     value_tag = ?7,
                     value_algorithm = ?8,
                     value_key_version = ?9,
                     updated_at = ?10
                 WHERE workspace_code = ?1 AND variable_key = ?2",
                params![
                    variable.workspace_code,
                    variable.key,
                    variable.value,
                    variable.description,
                    bool_to_int(variable.is_sensitive),
                    variable.value_nonce,
                    variable.value_tag,
                    variable.value_algorithm,
                    variable.value_key_version,
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
                     is_sensitive = ?4,
                     value_nonce = ?5,
                     value_tag = ?6,
                     value_algorithm = ?7,
                     value_key_version = ?8,
                     updated_at = ?9
                 WHERE variable_key = ?1",
                params![
                    variable.key,
                    variable.value,
                    variable.description,
                    bool_to_int(variable.is_sensitive),
                    variable.value_nonce,
                    variable.value_tag,
                    variable.value_algorithm,
                    variable.value_key_version,
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

fn storage_error(_error: rusqlite::Error) -> VariableError {
    VariableError::StorageFailure
}

fn bool_to_int(value: bool) -> i64 {
    i64::from(value)
}

fn int_to_bool(value: i64) -> bool {
    value != 0
}

impl SqliteVariableRepository {
    pub fn connect(database_url: &str) -> Result<Self, VariableError> {
        let connection = crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(database_url).map_err(storage_error)?;
        let database_url = database_url.to_owned();
        connection.call(
            move |connection| {
                SqliteVariableRepositorySql::initialize(&database_url, connection).map(|_| ())
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )?;
        Ok(Self { connection })
    }
}
impl VariableRepository for SqliteVariableRepository {
    fn list_variables(&mut self) -> Result<Vec<Variable>, VariableError> {
        self.connection.call(
            move |connection| SqliteVariableRepositorySql { connection }.list_variables(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_platform_variables(&mut self) -> Result<Vec<PlatformVariable>, VariableError> {
        self.connection.call(
            move |connection| SqliteVariableRepositorySql { connection }.list_platform_variables(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn next_id(&mut self) -> Result<i64, VariableError> {
        self.connection.call(
            move |connection| SqliteVariableRepositorySql { connection }.next_id(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn create_variable(&mut self, variable: &Variable) -> Result<(), VariableError> {
        let variable = variable.to_owned();
        self.connection.call(
            move |connection| SqliteVariableRepositorySql { connection }.create_variable(&variable),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn create_platform_variable(
        &mut self,
        variable: &PlatformVariable,
    ) -> Result<(), VariableError> {
        let variable = variable.to_owned();
        self.connection.call(
            move |connection| {
                SqliteVariableRepositorySql { connection }.create_platform_variable(&variable)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn update_variable(&mut self, variable: &Variable) -> Result<(), VariableError> {
        let variable = variable.to_owned();
        self.connection.call(
            move |connection| SqliteVariableRepositorySql { connection }.update_variable(&variable),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn update_platform_variable(
        &mut self,
        variable: &PlatformVariable,
    ) -> Result<(), VariableError> {
        let variable = variable.to_owned();
        self.connection.call(
            move |connection| {
                SqliteVariableRepositorySql { connection }.update_platform_variable(&variable)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_variable(&mut self, workspace_code: &str, key: &str) -> Result<bool, VariableError> {
        let workspace_code = workspace_code.to_owned();
        let key = key.to_owned();
        self.connection.call(
            move |connection| {
                SqliteVariableRepositorySql { connection }.delete_variable(&workspace_code, &key)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_platform_variable(&mut self, key: &str) -> Result<bool, VariableError> {
        let key = key.to_owned();
        self.connection.call(
            move |connection| {
                SqliteVariableRepositorySql { connection }.delete_platform_variable(&key)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
