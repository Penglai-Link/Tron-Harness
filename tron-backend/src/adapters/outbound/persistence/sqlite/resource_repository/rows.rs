use rusqlite::{Error as SqliteError, params};
use serde_json::Value;

use crate::{
    domain::resource::{
        DataSourceConnectionRecord, ResourceError, ResourceFileRecord, ResourceKind,
        ServerAccountRecord, ServerAuthenticationType, ServerRecord,
    },
    shared::time::{timestamp_from_millis, timestamp_to_millis},
};

use super::storage_error;

pub(super) fn insert_server_record(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    record: &ServerRecord,
) -> Result<(), ResourceError> {
    transaction
        .execute(
            "INSERT INTO resource_servers (
                workspace_code, resource_code, name, description, host, port,
                allow_inheritance, revision, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                record.workspace_code,
                record.resource_code,
                record.name,
                record.description,
                record.host,
                record.port as i64,
                bool_to_int(record.allow_inheritance),
                record.revision as i64,
                timestamp_to_millis(&record.created_at),
                timestamp_to_millis(&record.updated_at),
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(super) fn insert_server_account_record(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    record: &ServerAccountRecord,
) -> Result<(), ResourceError> {
    transaction
        .execute(
            "INSERT INTO resource_server_accounts (
                workspace_code, resource_code, server_workspace_code, server_code, name, description,
                username, authentication_type, password_variable_key,
                private_key_variable_key, private_key_passphrase_variable_key,
                allow_inheritance, revision, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                record.workspace_code,
                record.resource_code,
                record.server_workspace_code,
                record.server_code,
                record.name,
                record.description,
                record.username,
                authentication_type_to_storage(record.authentication_type),
                record.password_variable_key,
                record.private_key_variable_key,
                record.private_key_passphrase_variable_key,
                bool_to_int(record.allow_inheritance),
                record.revision as i64,
                timestamp_to_millis(&record.created_at),
                timestamp_to_millis(&record.updated_at),
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(super) fn insert_file_record(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    record: &ResourceFileRecord,
) -> Result<(), ResourceError> {
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
    Ok(())
}

pub(super) fn insert_data_source_record(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    record: &DataSourceConnectionRecord,
) -> Result<(), ResourceError> {
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
    Ok(())
}

pub(super) fn file_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ResourceFileRecord> {
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
        revision: row.get::<_, i64>(10)? as u64,
        created_at: timestamp_from_millis(row.get(11)?),
        updated_at: timestamp_from_millis(row.get(12)?),
    })
}

pub(super) fn data_source_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<DataSourceConnectionRecord> {
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
        revision: row.get::<_, i64>(7)? as u64,
        created_at: timestamp_from_millis(row.get(8)?),
        updated_at: timestamp_from_millis(row.get(9)?),
    })
}

pub(super) fn server_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ServerRecord> {
    Ok(ServerRecord {
        workspace_code: row.get(0)?,
        resource_code: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        host: row.get(4)?,
        port: row.get::<_, i64>(5)? as u16,
        allow_inheritance: row.get::<_, i64>(6)? != 0,
        revision: row.get::<_, i64>(7)? as u64,
        created_at: timestamp_from_millis(row.get(8)?),
        updated_at: timestamp_from_millis(row.get(9)?),
    })
}

pub(super) fn server_account_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ServerAccountRecord> {
    let authentication_text: String = row.get(7)?;
    Ok(ServerAccountRecord {
        workspace_code: row.get(0)?,
        resource_code: row.get(1)?,
        server_workspace_code: row.get(2)?,
        server_code: row.get(3)?,
        name: row.get(4)?,
        description: row.get(5)?,
        username: row.get(6)?,
        authentication_type: authentication_type_from_storage(&authentication_text)
            .map_err(resource_to_sqlite_error)?,
        password_variable_key: row.get(8)?,
        private_key_variable_key: row.get(9)?,
        private_key_passphrase_variable_key: row.get(10)?,
        allow_inheritance: row.get::<_, i64>(11)? != 0,
        revision: row.get::<_, i64>(12)? as u64,
        created_at: timestamp_from_millis(row.get(13)?),
        updated_at: timestamp_from_millis(row.get(14)?),
    })
}

pub(super) fn authentication_type_to_storage(value: ServerAuthenticationType) -> &'static str {
    match value {
        ServerAuthenticationType::Password => "Password",
        ServerAuthenticationType::PrivateKey => "PrivateKey",
    }
}

pub(super) fn authentication_type_from_storage(
    value: &str,
) -> Result<ServerAuthenticationType, ResourceError> {
    match value {
        "Password" => Ok(ServerAuthenticationType::Password),
        "PrivateKey" => Ok(ServerAuthenticationType::PrivateKey),
        _ => Err(ResourceError::StorageFailure),
    }
}

pub(super) fn bool_to_int(value: bool) -> i64 {
    i64::from(value)
}

pub(super) fn value_to_storage(value: &Value) -> Result<String, ResourceError> {
    serde_json::to_string(value).map_err(|_| ResourceError::StorageFailure)
}

fn resource_to_sqlite_error(_error: ResourceError) -> SqliteError {
    SqliteError::InvalidQuery
}
