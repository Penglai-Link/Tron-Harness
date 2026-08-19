use std::{fs, path::Path};

use rusqlite::{Connection, Error as SqliteError, params};
use serde_json::Value;

use crate::{
    application::resource::ResourceRepository,
    domain::resource::{
        DataSourceConnectionRecord, ResourceError, ResourceFileRecord, ResourceKind,
        ServerAccountRecord, ServerAuthenticationType, ServerRecord,
    },
    shared::time::{timestamp_from_millis, timestamp_to_millis},
};

use super::migrations::{
    RESOURCE_MANAGEMENT_MIGRATION, RESOURCE_MANAGEMENT_SIMPLIFY_MIGRATION,
};

pub struct SqliteResourceRepository {
    connection: Connection,
}

impl SqliteResourceRepository {
    pub fn connect(database_url: &str) -> Result<Self, ResourceError> {
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

    fn apply_migrations(&mut self) -> Result<(), ResourceError> {
        self.connection
            .execute_batch(RESOURCE_MANAGEMENT_MIGRATION)
            .map_err(storage_error)?;
        self.connection
            .execute_batch(RESOURCE_MANAGEMENT_SIMPLIFY_MIGRATION)
            .map_err(storage_error)
    }
}

impl ResourceRepository for SqliteResourceRepository {
    fn list_files(&mut self) -> Result<Vec<ResourceFileRecord>, ResourceError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT workspace_code, resource_code, kind, name, description,
                        file_name, file_extension, byte_size, storage_locator, allow_inheritance,
                        created_at, updated_at
                 FROM resource_files
                 ORDER BY workspace_code, resource_code",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], file_from_row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn insert_file(&mut self, record: &ResourceFileRecord) -> Result<(), ResourceError> {
        self.connection
            .execute(
                "INSERT INTO resource_files (
                    workspace_code, resource_code, kind, name, description,
                    file_name, file_extension, byte_size, storage_locator, allow_inheritance,
                    created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    record.workspace_code,
                    record.resource_code,
                    record.kind.as_str(),
                    record.name,
                    record.description,
                    record.file_name,
                    record.file_extension,
                    record.byte_size as i64,
                    record.storage_locator,
                    bool_to_int(record.allow_inheritance),
                    timestamp_to_millis(&record.created_at),
                    timestamp_to_millis(&record.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn update_file(&mut self, record: &ResourceFileRecord) -> Result<bool, ResourceError> {
        let changed = self
            .connection
            .execute(
                "UPDATE resource_files
                 SET kind = ?3, name = ?4, description = ?5, file_name = ?6,
                     file_extension = ?7, byte_size = ?8, storage_locator = ?9,
                     allow_inheritance = ?10, updated_at = ?11
                 WHERE workspace_code = ?1 AND resource_code = ?2",
                params![
                    record.workspace_code,
                    record.resource_code,
                    record.kind.as_str(),
                    record.name,
                    record.description,
                    record.file_name,
                    record.file_extension,
                    record.byte_size as i64,
                    record.storage_locator,
                    bool_to_int(record.allow_inheritance),
                    timestamp_to_millis(&record.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(changed > 0)
    }

    fn delete_file(
        &mut self,
        workspace_code: &str,
        resource_code: &str,
    ) -> Result<Option<ResourceFileRecord>, ResourceError> {
        let record = self
            .list_files()?
            .into_iter()
            .find(|record| {
                record.workspace_code == workspace_code
                    && record.resource_code == resource_code
            });
        if record.is_some() {
            self.connection
                .execute(
                    "DELETE FROM resource_files
                     WHERE workspace_code = ?1 AND resource_code = ?2",
                    params![workspace_code, resource_code],
                )
                .map_err(storage_error)?;
        }
        Ok(record)
    }

    fn list_data_source_connections(
        &mut self,
    ) -> Result<Vec<DataSourceConnectionRecord>, ResourceError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT workspace_code, resource_code, kind, name, description,
                        attributes_json, allow_inheritance, created_at, updated_at
                 FROM resource_data_source_connections
                 ORDER BY workspace_code, resource_code",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], data_source_from_row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn insert_data_source_connection(
        &mut self,
        record: &DataSourceConnectionRecord,
    ) -> Result<(), ResourceError> {
        self.connection
            .execute(
                "INSERT INTO resource_data_source_connections (
                    workspace_code, resource_code, kind, name, description,
                    attributes_json, allow_inheritance, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.workspace_code,
                    record.resource_code,
                    record.kind.as_str(),
                    record.name,
                    record.description,
                    value_to_storage(&record.attributes)?,
                    bool_to_int(record.allow_inheritance),
                    timestamp_to_millis(&record.created_at),
                    timestamp_to_millis(&record.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn update_data_source_connection(
        &mut self,
        record: &DataSourceConnectionRecord,
    ) -> Result<bool, ResourceError> {
        let changed = self
            .connection
            .execute(
                "UPDATE resource_data_source_connections
                 SET name = ?3, description = ?4, attributes_json = ?5,
                     allow_inheritance = ?6, updated_at = ?7
                 WHERE workspace_code = ?1 AND resource_code = ?2",
                params![
                    record.workspace_code,
                    record.resource_code,
                    record.name,
                    record.description,
                    value_to_storage(&record.attributes)?,
                    bool_to_int(record.allow_inheritance),
                    timestamp_to_millis(&record.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(changed > 0)
    }

    fn delete_data_source_connection(
        &mut self,
        workspace_code: &str,
        resource_code: &str,
    ) -> Result<Option<DataSourceConnectionRecord>, ResourceError> {
        let record = self
            .list_data_source_connections()?
            .into_iter()
            .find(|record| {
                record.workspace_code == workspace_code
                    && record.resource_code == resource_code
            });
        if record.is_some() {
            self.connection
                .execute(
                    "DELETE FROM resource_data_source_connections
                     WHERE workspace_code = ?1 AND resource_code = ?2",
                    params![workspace_code, resource_code],
                )
                .map_err(storage_error)?;
        }
        Ok(record)
    }

    fn list_servers(&mut self) -> Result<Vec<ServerRecord>, ResourceError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT workspace_code, resource_code, name, description, host, port,
                        allow_inheritance, created_at, updated_at
                 FROM resource_servers
                 ORDER BY workspace_code, resource_code",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], server_from_row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn insert_server(&mut self, record: &ServerRecord) -> Result<(), ResourceError> {
        self.connection
            .execute(
                "INSERT INTO resource_servers (
                    workspace_code, resource_code, name, description, host, port,
                    allow_inheritance, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.workspace_code,
                    record.resource_code,
                    record.name,
                    record.description,
                    record.host,
                    record.port as i64,
                    bool_to_int(record.allow_inheritance),
                    timestamp_to_millis(&record.created_at),
                    timestamp_to_millis(&record.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn update_server(&mut self, record: &ServerRecord) -> Result<bool, ResourceError> {
        let changed = self
            .connection
            .execute(
                "UPDATE resource_servers
                 SET name = ?3, description = ?4, host = ?5, port = ?6,
                     allow_inheritance = ?7, updated_at = ?8
                 WHERE workspace_code = ?1 AND resource_code = ?2",
                params![
                    record.workspace_code,
                    record.resource_code,
                    record.name,
                    record.description,
                    record.host,
                    record.port as i64,
                    bool_to_int(record.allow_inheritance),
                    timestamp_to_millis(&record.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(changed > 0)
    }

    fn delete_server(
        &mut self,
        workspace_code: &str,
        resource_code: &str,
    ) -> Result<Option<ServerRecord>, ResourceError> {
        let record = self
            .list_servers()?
            .into_iter()
            .find(|record| {
                record.workspace_code == workspace_code
                    && record.resource_code == resource_code
            });
        if record.is_some() {
            self.connection
                .execute(
                    "DELETE FROM resource_servers
                     WHERE workspace_code = ?1 AND resource_code = ?2",
                    params![workspace_code, resource_code],
                )
                .map_err(storage_error)?;
        }
        Ok(record)
    }

    fn list_server_accounts(&mut self) -> Result<Vec<ServerAccountRecord>, ResourceError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT workspace_code, resource_code, server_code, name, description,
                        username, authentication_type, password_variable_key,
                        private_key_variable_key, private_key_passphrase_variable_key,
                        allow_inheritance, created_at, updated_at
                 FROM resource_server_accounts
                 ORDER BY workspace_code, resource_code",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], server_account_from_row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn insert_server_account(&mut self, record: &ServerAccountRecord) -> Result<(), ResourceError> {
        self.connection
            .execute(
                "INSERT INTO resource_server_accounts (
                    workspace_code, resource_code, server_code, name, description,
                    username, authentication_type, password_variable_key,
                    private_key_variable_key, private_key_passphrase_variable_key,
                    allow_inheritance, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    record.workspace_code,
                    record.resource_code,
                    record.server_code,
                    record.name,
                    record.description,
                    record.username,
                    authentication_type_to_storage(record.authentication_type),
                    record.password_variable_key,
                    record.private_key_variable_key,
                    record.private_key_passphrase_variable_key,
                    bool_to_int(record.allow_inheritance),
                    timestamp_to_millis(&record.created_at),
                    timestamp_to_millis(&record.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn update_server_account(&mut self, record: &ServerAccountRecord) -> Result<bool, ResourceError> {
        let changed = self
            .connection
            .execute(
                "UPDATE resource_server_accounts
                 SET name = ?3, description = ?4, username = ?5, authentication_type = ?6,
                     password_variable_key = ?7, private_key_variable_key = ?8,
                     private_key_passphrase_variable_key = ?9, allow_inheritance = ?10,
                     updated_at = ?11
                 WHERE workspace_code = ?1 AND resource_code = ?2",
                params![
                    record.workspace_code,
                    record.resource_code,
                    record.name,
                    record.description,
                    record.username,
                    authentication_type_to_storage(record.authentication_type),
                    record.password_variable_key,
                    record.private_key_variable_key,
                    record.private_key_passphrase_variable_key,
                    bool_to_int(record.allow_inheritance),
                    timestamp_to_millis(&record.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(changed > 0)
    }

    fn delete_server_account(
        &mut self,
        workspace_code: &str,
        resource_code: &str,
    ) -> Result<Option<ServerAccountRecord>, ResourceError> {
        let record = self
            .list_server_accounts()?
            .into_iter()
            .find(|record| {
                record.workspace_code == workspace_code
                    && record.resource_code == resource_code
            });
        if record.is_some() {
            self.connection
                .execute(
                    "DELETE FROM resource_server_accounts
                     WHERE workspace_code = ?1 AND resource_code = ?2",
                    params![workspace_code, resource_code],
                )
                .map_err(storage_error)?;
        }
        Ok(record)
    }
}

fn file_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ResourceFileRecord> {
    let kind_text: String = row.get(2)?;
    Ok(ResourceFileRecord {
        workspace_code: row.get(0)?,
        resource_code: row.get(1)?,
        kind: ResourceKind::parse(&kind_text)
            .ok_or(ResourceError::StorageFailure)
            .map_err(resource_to_sqlite_error)?,
        name: row.get(3)?,
        description: row.get(4)?,
        file_name: row.get(5)?,
        file_extension: row.get(6)?,
        byte_size: row.get::<_, i64>(7)? as u64,
        storage_locator: row.get(8)?,
        allow_inheritance: row.get::<_, i64>(9)? != 0,
        created_at: timestamp_from_millis(row.get(10)?),
        updated_at: timestamp_from_millis(row.get(11)?),
    })
}

fn data_source_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DataSourceConnectionRecord> {
    let kind_text: String = row.get(2)?;
    let attributes_text: String = row.get(5)?;
    Ok(DataSourceConnectionRecord {
        workspace_code: row.get(0)?,
        resource_code: row.get(1)?,
        kind: ResourceKind::parse(&kind_text)
            .ok_or(ResourceError::StorageFailure)
            .map_err(resource_to_sqlite_error)?,
        name: row.get(3)?,
        description: row.get(4)?,
        attributes: serde_json::from_str(&attributes_text)
            .map_err(|_| resource_to_sqlite_error(ResourceError::StorageFailure))?,
        allow_inheritance: row.get::<_, i64>(6)? != 0,
        created_at: timestamp_from_millis(row.get(7)?),
        updated_at: timestamp_from_millis(row.get(8)?),
    })
}

fn server_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ServerRecord> {
    Ok(ServerRecord {
        workspace_code: row.get(0)?,
        resource_code: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        host: row.get(4)?,
        port: row.get::<_, i64>(5)? as u16,
        allow_inheritance: row.get::<_, i64>(6)? != 0,
        created_at: timestamp_from_millis(row.get(7)?),
        updated_at: timestamp_from_millis(row.get(8)?),
    })
}

fn server_account_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ServerAccountRecord> {
    let authentication_text: String = row.get(6)?;
    Ok(ServerAccountRecord {
        workspace_code: row.get(0)?,
        resource_code: row.get(1)?,
        server_code: row.get(2)?,
        name: row.get(3)?,
        description: row.get(4)?,
        username: row.get(5)?,
        authentication_type: authentication_type_from_storage(&authentication_text)
            .map_err(resource_to_sqlite_error)?,
        password_variable_key: row.get(7)?,
        private_key_variable_key: row.get(8)?,
        private_key_passphrase_variable_key: row.get(9)?,
        allow_inheritance: row.get::<_, i64>(10)? != 0,
        created_at: timestamp_from_millis(row.get(11)?),
        updated_at: timestamp_from_millis(row.get(12)?),
    })
}

fn authentication_type_to_storage(value: ServerAuthenticationType) -> &'static str {
    match value {
        ServerAuthenticationType::Password => "Password",
        ServerAuthenticationType::PrivateKey => "PrivateKey",
    }
}

fn authentication_type_from_storage(value: &str) -> Result<ServerAuthenticationType, ResourceError> {
    match value {
        "Password" => Ok(ServerAuthenticationType::Password),
        "PrivateKey" => Ok(ServerAuthenticationType::PrivateKey),
        _ => Err(ResourceError::StorageFailure),
    }
}

fn bool_to_int(value: bool) -> i64 {
    i64::from(value)
}

fn value_to_storage(value: &Value) -> Result<String, ResourceError> {
    serde_json::to_string(value).map_err(|_| ResourceError::StorageFailure)
}

fn resource_to_sqlite_error(_error: ResourceError) -> SqliteError {
    SqliteError::InvalidQuery
}

fn ensure_database_parent(database_url: &str) -> Result<(), ResourceError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| ResourceError::StorageFailure)?;
    }
    Ok(())
}

fn storage_error(error: SqliteError) -> ResourceError {
    match error {
        SqliteError::SqliteFailure(code, _)
            if code.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            ResourceError::Conflict
        }
        SqliteError::QueryReturnedNoRows => ResourceError::NotFound,
        _ => ResourceError::StorageFailure,
    }
}
