use std::{fs, path::Path};

use rusqlite::{Connection, Error as SqliteError, OptionalExtension, params};

use crate::{
    application::resource::ResourceRepository,
    domain::resource::{
        DataSourceConnectionRecord, ResourceError, ResourceFileRecord, ResourceKind,
        ServerAccountRecord, ServerRecord,
    },
    shared::time::timestamp_to_millis,
};

use super::migrations::RESOURCE_MANAGEMENT_SIMPLIFY_MIGRATION;

mod legacy;
mod rows;
mod schema;

use legacy::migrate_legacy_active_resources;
use rows::{
    authentication_type_to_storage, bool_to_int, data_source_from_row, file_from_row,
    insert_server_account_record, insert_server_record, server_account_from_row, server_from_row,
    value_to_storage,
};
use schema::{
    backfill_resource_code_registry, claim_resource_code, ensure_revision_columns,
    ensure_server_account_origin_column, ensure_server_account_parent_guards,
    release_resource_code,
};

const RESOURCE_MANAGEMENT_SIMPLIFY_VERSION: &str = "20260818_0001";

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
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(storage_error)?;
        let mut repository = Self { connection };
        repository.apply_migrations()?;
        Ok(repository)
    }

    fn apply_migrations(&mut self) -> Result<(), ResourceError> {
        let transaction = self.connection.transaction().map_err(storage_error)?;
        transaction
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS resource_management_migrations (
                    version TEXT PRIMARY KEY,
                    applied_at INTEGER NOT NULL
                );",
            )
            .map_err(storage_error)?;
        let applied = transaction
            .query_row(
                "SELECT 1 FROM resource_management_migrations WHERE version = ?1",
                [RESOURCE_MANAGEMENT_SIMPLIFY_VERSION],
                |_| Ok(()),
            )
            .optional()
            .map_err(storage_error)?
            .is_some();
        if !applied {
            transaction
                .execute_batch(RESOURCE_MANAGEMENT_SIMPLIFY_MIGRATION)
                .map_err(storage_error)?;
        }
        ensure_revision_columns(&transaction)?;
        ensure_server_account_origin_column(&transaction)?;
        if !applied {
            migrate_legacy_active_resources(&transaction)?;
            backfill_resource_code_registry(&transaction)?;
            transaction
                .execute(
                    "INSERT INTO resource_management_migrations (version, applied_at)
                     VALUES (?1, strftime('%s', 'now') * 1000)",
                    [RESOURCE_MANAGEMENT_SIMPLIFY_VERSION],
                )
                .map_err(storage_error)?;
        }
        ensure_server_account_parent_guards(&transaction)?;
        transaction.commit().map_err(storage_error)
    }
}

impl ResourceRepository for SqliteResourceRepository {
    fn list_files(&mut self) -> Result<Vec<ResourceFileRecord>, ResourceError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT workspace_code, resource_code, kind, name, description,
                        file_name, file_extension, byte_size, storage_locator, allow_inheritance,
                        revision, created_at, updated_at
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
        let transaction = self.connection.transaction().map_err(storage_error)?;
        claim_resource_code(
            &transaction,
            &record.workspace_code,
            &record.resource_code,
            record.kind.as_str(),
        )?;
        transaction
            .execute(
                "INSERT INTO resource_files (
                    workspace_code, resource_code, kind, name, description,
                    file_name, file_extension, byte_size, storage_locator, allow_inheritance,
                    revision, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
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
                    record.revision as i64,
                    timestamp_to_millis(&record.created_at),
                    timestamp_to_millis(&record.updated_at),
                ],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)
    }

    fn update_file(&mut self, record: &ResourceFileRecord) -> Result<bool, ResourceError> {
        let changed = self
            .connection
            .execute(
                "UPDATE resource_files
                 SET kind = ?3, name = ?4, description = ?5, file_name = ?6,
                     file_extension = ?7, byte_size = ?8, storage_locator = ?9,
                     allow_inheritance = ?10, updated_at = ?11, revision = revision + 1
                 WHERE workspace_code = ?1 AND resource_code = ?2 AND revision = ?12",
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
                    record.revision as i64,
                ],
            )
            .map_err(storage_error)?;
        Ok(changed > 0)
    }

    fn delete_file(
        &mut self,
        workspace_code: &str,
        resource_code: &str,
        revision: u64,
    ) -> Result<bool, ResourceError> {
        let transaction = self.connection.transaction().map_err(storage_error)?;
        let deleted = transaction
            .execute(
                "DELETE FROM resource_files
                 WHERE workspace_code = ?1 AND resource_code = ?2 AND revision = ?3",
                params![workspace_code, resource_code, revision as i64],
            )
            .map_err(storage_error)?;
        if deleted > 0 {
            release_resource_code(&transaction, workspace_code, resource_code)?;
        }
        transaction.commit().map_err(storage_error)?;
        Ok(deleted > 0)
    }

    fn list_data_source_connections(
        &mut self,
    ) -> Result<Vec<DataSourceConnectionRecord>, ResourceError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT workspace_code, resource_code, kind, name, description,
                        attributes_json, allow_inheritance, revision, created_at, updated_at
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
        let transaction = self.connection.transaction().map_err(storage_error)?;
        claim_resource_code(
            &transaction,
            &record.workspace_code,
            &record.resource_code,
            record.kind.as_str(),
        )?;
        transaction
            .execute(
                "INSERT INTO resource_data_source_connections (
                    workspace_code, resource_code, kind, name, description,
                    attributes_json, allow_inheritance, revision, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    record.workspace_code,
                    record.resource_code,
                    record.kind.as_str(),
                    record.name,
                    record.description,
                    value_to_storage(&record.attributes)?,
                    bool_to_int(record.allow_inheritance),
                    record.revision as i64,
                    timestamp_to_millis(&record.created_at),
                    timestamp_to_millis(&record.updated_at),
                ],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)
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
                     allow_inheritance = ?6, updated_at = ?7, revision = revision + 1
                 WHERE workspace_code = ?1 AND resource_code = ?2 AND revision = ?8",
                params![
                    record.workspace_code,
                    record.resource_code,
                    record.name,
                    record.description,
                    value_to_storage(&record.attributes)?,
                    bool_to_int(record.allow_inheritance),
                    timestamp_to_millis(&record.updated_at),
                    record.revision as i64,
                ],
            )
            .map_err(storage_error)?;
        Ok(changed > 0)
    }

    fn delete_data_source_connection(
        &mut self,
        workspace_code: &str,
        resource_code: &str,
        revision: u64,
    ) -> Result<bool, ResourceError> {
        let transaction = self.connection.transaction().map_err(storage_error)?;
        let deleted = transaction
            .execute(
                "DELETE FROM resource_data_source_connections
                 WHERE workspace_code = ?1 AND resource_code = ?2 AND revision = ?3",
                params![workspace_code, resource_code, revision as i64],
            )
            .map_err(storage_error)?;
        if deleted > 0 {
            release_resource_code(&transaction, workspace_code, resource_code)?;
        }
        transaction.commit().map_err(storage_error)?;
        Ok(deleted > 0)
    }

    fn list_servers(&mut self) -> Result<Vec<ServerRecord>, ResourceError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT workspace_code, resource_code, name, description, host, port,
                        allow_inheritance, revision, created_at, updated_at
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
        let transaction = self.connection.transaction().map_err(storage_error)?;
        claim_resource_code(
            &transaction,
            &record.workspace_code,
            &record.resource_code,
            ResourceKind::ServerConnection.as_str(),
        )?;
        insert_server_record(&transaction, record)?;
        transaction.commit().map_err(storage_error)
    }

    fn insert_server_bundle(
        &mut self,
        server: &ServerRecord,
        initial_account: &ServerAccountRecord,
    ) -> Result<(), ResourceError> {
        if server.workspace_code != initial_account.workspace_code
            || server.resource_code == initial_account.resource_code
            || initial_account.server_workspace_code != server.workspace_code
            || initial_account.server_code != server.resource_code
        {
            return Err(ResourceError::InvalidInput);
        }
        let transaction = self.connection.transaction().map_err(storage_error)?;
        claim_resource_code(
            &transaction,
            &server.workspace_code,
            &server.resource_code,
            ResourceKind::ServerConnection.as_str(),
        )?;
        claim_resource_code(
            &transaction,
            &initial_account.workspace_code,
            &initial_account.resource_code,
            ResourceKind::ServerAccount.as_str(),
        )?;
        insert_server_record(&transaction, server)?;
        insert_server_account_record(&transaction, initial_account)?;
        transaction.commit().map_err(storage_error)
    }

    fn update_server(&mut self, record: &ServerRecord) -> Result<bool, ResourceError> {
        let changed = self
            .connection
            .execute(
                "UPDATE resource_servers
                 SET name = ?3, description = ?4, host = ?5, port = ?6,
                     allow_inheritance = ?7, updated_at = ?8, revision = revision + 1
                 WHERE workspace_code = ?1 AND resource_code = ?2 AND revision = ?9",
                params![
                    record.workspace_code,
                    record.resource_code,
                    record.name,
                    record.description,
                    record.host,
                    record.port as i64,
                    bool_to_int(record.allow_inheritance),
                    timestamp_to_millis(&record.updated_at),
                    record.revision as i64,
                ],
            )
            .map_err(storage_error)?;
        Ok(changed > 0)
    }

    fn delete_server_if_no_accounts(
        &mut self,
        workspace_code: &str,
        resource_code: &str,
        revision: u64,
    ) -> Result<bool, ResourceError> {
        let transaction = self.connection.transaction().map_err(storage_error)?;
        let has_accounts = transaction
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM resource_server_accounts
                    WHERE server_workspace_code = ?1 AND server_code = ?2
                )",
                params![workspace_code, resource_code],
                |row| row.get::<_, i64>(0),
            )
            .map_err(storage_error)?
            != 0;
        if has_accounts {
            return Err(ResourceError::ServerHasAccounts);
        }
        let deleted = transaction
            .execute(
                "DELETE FROM resource_servers
                 WHERE workspace_code = ?1 AND resource_code = ?2 AND revision = ?3",
                params![workspace_code, resource_code, revision as i64],
            )
            .map_err(storage_error)?;
        if deleted > 0 {
            release_resource_code(&transaction, workspace_code, resource_code)?;
        }
        transaction.commit().map_err(storage_error)?;
        Ok(deleted > 0)
    }

    fn list_server_accounts(&mut self) -> Result<Vec<ServerAccountRecord>, ResourceError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT workspace_code, resource_code, server_workspace_code, server_code,
                        name, description,
                        username, authentication_type, password_variable_key,
                        private_key_variable_key, private_key_passphrase_variable_key,
                        allow_inheritance, revision, created_at, updated_at
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
        let transaction = self.connection.transaction().map_err(storage_error)?;
        claim_resource_code(
            &transaction,
            &record.workspace_code,
            &record.resource_code,
            ResourceKind::ServerAccount.as_str(),
        )?;
        insert_server_account_record(&transaction, record)?;
        transaction.commit().map_err(storage_error)
    }

    fn update_server_account(
        &mut self,
        record: &ServerAccountRecord,
    ) -> Result<bool, ResourceError> {
        let changed = self
            .connection
            .execute(
                "UPDATE resource_server_accounts
                 SET name = ?3, description = ?4, username = ?5, authentication_type = ?6,
                     password_variable_key = ?7, private_key_variable_key = ?8,
                     private_key_passphrase_variable_key = ?9, allow_inheritance = ?10,
                     updated_at = ?11, revision = revision + 1
                 WHERE workspace_code = ?1 AND resource_code = ?2 AND revision = ?12",
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
                    record.revision as i64,
                ],
            )
            .map_err(storage_error)?;
        Ok(changed > 0)
    }

    fn delete_server_account(
        &mut self,
        workspace_code: &str,
        resource_code: &str,
        revision: u64,
    ) -> Result<bool, ResourceError> {
        let transaction = self.connection.transaction().map_err(storage_error)?;
        let deleted = transaction
            .execute(
                "DELETE FROM resource_server_accounts
                 WHERE workspace_code = ?1 AND resource_code = ?2 AND revision = ?3",
                params![workspace_code, resource_code, revision as i64],
            )
            .map_err(storage_error)?;
        if deleted > 0 {
            release_resource_code(&transaction, workspace_code, resource_code)?;
        }
        transaction.commit().map_err(storage_error)?;
        Ok(deleted > 0)
    }
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
