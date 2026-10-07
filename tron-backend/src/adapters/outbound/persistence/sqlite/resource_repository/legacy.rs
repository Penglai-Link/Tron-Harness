use std::path::Path;

use rusqlite::{OptionalExtension, params};
use serde_json::Value;

use crate::{
    domain::resource::{
        DataSourceConnectionRecord, ResourceError, ResourceFileRecord, ResourceKind,
        ServerAccountRecord, ServerRecord,
    },
    shared::time::timestamp_from_millis,
};

use super::{
    rows::{
        authentication_type_from_storage, insert_data_source_record, insert_file_record,
        insert_server_account_record, insert_server_record,
    },
    schema::claim_resource_code,
    storage_error,
};

pub(super) fn migrate_legacy_active_resources(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
) -> Result<(), ResourceError> {
    if !table_exists(transaction, "resources")? || simplified_resources_exist(transaction)? {
        return Ok(());
    }

    let mut statement = transaction
        .prepare(
            "SELECT scope_kind, scope_code, resource_code, resource_version, definition_code,
                    name, description, attributes_json, allow_inheritance, created_at, updated_at
             FROM resources
             WHERE lifecycle_status = 'ACTIVE'
             ORDER BY scope_kind, scope_code, resource_code, created_at, resource_version",
        )
        .map_err(storage_error)?;
    let mut legacy_records = statement
        .query_map([], |row| {
            Ok(LegacyResourceRecord {
                scope_kind: row.get(0)?,
                scope_code: row.get(1)?,
                resource_code: row.get(2)?,
                resource_version: row.get(3)?,
                definition_code: row.get(4)?,
                name: row.get(5)?,
                description: row.get(6)?,
                attributes_json: row.get(7)?,
                allow_inheritance: row.get::<_, i64>(8)? != 0,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;

    // The simplified account table has a real parent constraint. Migrate the
    // parent server rows first even when the legacy resource codes sort after
    // their accounts.
    legacy_records.sort_by_key(|record| match record.definition_code.as_str() {
        "SERVER_CONNECTION" => 0,
        "SERVER_ACCOUNT" => 2,
        _ => 1,
    });

    for legacy in legacy_records {
        let workspace_code = match (legacy.scope_kind.as_str(), legacy.scope_code.as_str()) {
            ("WORKSPACE", workspace_code) if !workspace_code.trim().is_empty() => {
                workspace_code.to_string()
            }
            ("PLATFORM", "PLATFORM") => "PLATFORM".to_string(),
            _ => {
                record_legacy_migration_issue(
                    transaction,
                    &legacy,
                    "the legacy scope cannot be represented by the simplified workspace model",
                )?;
                continue;
            }
        };
        let attributes: Value = match serde_json::from_str(&legacy.attributes_json) {
            Ok(value) => value,
            Err(_) => {
                record_legacy_migration_issue(
                    transaction,
                    &legacy,
                    "attributes_json is not valid JSON",
                )?;
                continue;
            }
        };
        match legacy.definition_code.as_str() {
            "FILE" | "IMAGE" | "PDF" => {
                let Some(kind) = ResourceKind::parse(&legacy.definition_code) else {
                    unreachable!("known legacy file kinds must parse");
                };
                let Some((file_name, file_extension)) = legacy_file_metadata(kind, &attributes)
                else {
                    record_legacy_migration_issue(
                        transaction,
                        &legacy,
                        "file attributes cannot be represented by one simplified binary",
                    )?;
                    continue;
                };
                let mappings = legacy_binary_mappings(transaction, &legacy)?;
                let Some((storage_locator, byte_size)) = mappings.first().cloned() else {
                    record_legacy_migration_issue(
                        transaction,
                        &legacy,
                        "active file resource does not have a binary mapping",
                    )?;
                    continue;
                };
                if mappings.len() != 1 {
                    record_legacy_migration_issue(
                        transaction,
                        &legacy,
                        "multiple legacy binary mappings cannot be represented by one simplified binary",
                    )?;
                    continue;
                }
                let record = ResourceFileRecord {
                    workspace_code: workspace_code.clone(),
                    resource_code: legacy.resource_code.clone(),
                    kind,
                    name: legacy.name.clone(),
                    description: legacy.description.clone(),
                    file_name,
                    file_extension,
                    byte_size,
                    storage_locator,
                    allow_inheritance: legacy.allow_inheritance,
                    revision: 0,
                    created_at: timestamp_from_millis(legacy.created_at),
                    updated_at: timestamp_from_millis(legacy.updated_at),
                };
                claim_resource_code(
                    transaction,
                    &record.workspace_code,
                    &record.resource_code,
                    record.kind.as_str(),
                )?;
                insert_file_record(transaction, &record)?;
            }
            "MYSQL_CONNECTION"
            | "POSTGRESQL_CONNECTION"
            | "REDIS_CONNECTION"
            | "KAFKA_CONNECTION" => {
                let Some(kind) = ResourceKind::parse(&legacy.definition_code) else {
                    unreachable!("known legacy data source kinds must parse");
                };
                let record = DataSourceConnectionRecord {
                    workspace_code: workspace_code.clone(),
                    resource_code: legacy.resource_code.clone(),
                    kind,
                    name: legacy.name.clone(),
                    description: legacy.description.clone(),
                    attributes,
                    allow_inheritance: legacy.allow_inheritance,
                    revision: 0,
                    created_at: timestamp_from_millis(legacy.created_at),
                    updated_at: timestamp_from_millis(legacy.updated_at),
                };
                claim_resource_code(
                    transaction,
                    &record.workspace_code,
                    &record.resource_code,
                    record.kind.as_str(),
                )?;
                insert_data_source_record(transaction, &record)?;
            }
            "SERVER_CONNECTION" => {
                let Some(host) = legacy_required_attribute(&attributes, "host") else {
                    record_legacy_migration_issue(transaction, &legacy, "server host is missing")?;
                    continue;
                };
                let Some(port) = attributes
                    .get("port")
                    .and_then(Value::as_u64)
                    .filter(|port| (1..=u16::MAX as u64).contains(port))
                    .map(|port| port as u16)
                else {
                    record_legacy_migration_issue(transaction, &legacy, "server port is invalid")?;
                    continue;
                };
                let record = ServerRecord {
                    workspace_code: workspace_code.clone(),
                    resource_code: legacy.resource_code.clone(),
                    name: legacy.name.clone(),
                    description: legacy.description.clone(),
                    host,
                    port,
                    allow_inheritance: legacy.allow_inheritance,
                    revision: 0,
                    created_at: timestamp_from_millis(legacy.created_at),
                    updated_at: timestamp_from_millis(legacy.updated_at),
                };
                claim_resource_code(
                    transaction,
                    &record.workspace_code,
                    &record.resource_code,
                    ResourceKind::ServerConnection.as_str(),
                )?;
                insert_server_record(transaction, &record)?;
            }
            "SERVER_ACCOUNT" => {
                let Some(server_code) = legacy_required_attribute(&attributes, "serverCode") else {
                    record_legacy_migration_issue(transaction, &legacy, "serverCode is missing")?;
                    continue;
                };
                let Some(username) = legacy_required_attribute(&attributes, "username") else {
                    record_legacy_migration_issue(transaction, &legacy, "username is missing")?;
                    continue;
                };
                let Some(authentication_type) = attributes
                    .get("authenticationType")
                    .and_then(Value::as_str)
                    .and_then(|value| authentication_type_from_storage(value).ok())
                else {
                    record_legacy_migration_issue(
                        transaction,
                        &legacy,
                        "authenticationType is invalid",
                    )?;
                    continue;
                };
                let record = ServerAccountRecord {
                    workspace_code: workspace_code.clone(),
                    resource_code: legacy.resource_code.clone(),
                    server_workspace_code: workspace_code.clone(),
                    server_code,
                    name: legacy.name.clone(),
                    description: legacy.description.clone(),
                    username,
                    authentication_type,
                    password_variable_key: legacy_optional_attribute(
                        &attributes,
                        "passwordVariableKey",
                    ),
                    private_key_variable_key: legacy_optional_attribute(
                        &attributes,
                        "privateKeyVariableKey",
                    ),
                    private_key_passphrase_variable_key: legacy_optional_attribute(
                        &attributes,
                        "privateKeyPassphraseVariableKey",
                    ),
                    allow_inheritance: legacy.allow_inheritance,
                    revision: 0,
                    created_at: timestamp_from_millis(legacy.created_at),
                    updated_at: timestamp_from_millis(legacy.updated_at),
                };
                claim_resource_code(
                    transaction,
                    &record.workspace_code,
                    &record.resource_code,
                    ResourceKind::ServerAccount.as_str(),
                )?;
                insert_server_account_record(transaction, &record)?;
            }
            _ => record_legacy_migration_issue(
                transaction,
                &legacy,
                "the legacy definition is not supported by the simplified resource model",
            )?,
        }
    }
    Ok(())
}

#[derive(Debug)]
struct LegacyResourceRecord {
    scope_kind: String,
    scope_code: String,
    resource_code: String,
    resource_version: String,
    definition_code: String,
    name: String,
    description: String,
    attributes_json: String,
    allow_inheritance: bool,
    created_at: i64,
    updated_at: i64,
}

fn table_exists(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    table: &str,
) -> Result<bool, ResourceError> {
    transaction
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |_| Ok(()),
        )
        .optional()
        .map(|value| value.is_some())
        .map_err(storage_error)
}

fn simplified_resources_exist(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
) -> Result<bool, ResourceError> {
    for table in [
        "resource_files",
        "resource_data_source_connections",
        "resource_servers",
        "resource_server_accounts",
    ] {
        let query = format!("SELECT EXISTS(SELECT 1 FROM {table} LIMIT 1)");
        let exists = transaction
            .query_row(&query, [], |row| row.get::<_, i64>(0))
            .map_err(storage_error)?;
        if exists != 0 {
            return Ok(true);
        }
    }
    Ok(false)
}

fn legacy_binary_mappings(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    legacy: &LegacyResourceRecord,
) -> Result<Vec<(String, u64)>, ResourceError> {
    if !table_exists(transaction, "resource_binary_mappings")? {
        return Ok(Vec::new());
    }
    let mut statement = transaction
        .prepare(
            "SELECT storage_locator, byte_size
             FROM resource_binary_mappings
             WHERE scope_kind = ?1 AND scope_code = ?2 AND resource_code = ?3
               AND resource_version = ?4
             ORDER BY binary_name",
        )
        .map_err(storage_error)?;
    statement
        .query_map(
            params![
                legacy.scope_kind,
                legacy.scope_code,
                legacy.resource_code,
                legacy.resource_version,
            ],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64)),
        )
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)
}

fn legacy_file_metadata(kind: ResourceKind, attributes: &Value) -> Option<(String, String)> {
    let source = match kind {
        ResourceKind::Image => {
            let images = attributes.get("images")?.as_array()?;
            if images.len() != 1 {
                return None;
            }
            images.first()?
        }
        ResourceKind::File | ResourceKind::Pdf => attributes,
        _ => return None,
    };
    let file_name = source.get("fileName")?.as_str()?.trim();
    if file_name.is_empty() || file_name.contains('/') || file_name.contains('\\') {
        return None;
    }
    let file_extension = source
        .get("fileExtension")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            Path::new(file_name)
                .extension()
                .and_then(|value| value.to_str())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_default();
    Some((file_name.to_string(), file_extension))
}

fn legacy_required_attribute(attributes: &Value, key: &str) -> Option<String> {
    attributes
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn legacy_optional_attribute(attributes: &Value, key: &str) -> Option<String> {
    attributes
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn record_legacy_migration_issue(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    legacy: &LegacyResourceRecord,
    reason: &str,
) -> Result<(), ResourceError> {
    transaction
        .execute(
            "INSERT OR REPLACE INTO resource_management_migration_issues (
                scope_kind, scope_code, resource_code, resource_version, reason
            ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                legacy.scope_kind,
                legacy.scope_code,
                legacy.resource_code,
                legacy.resource_version,
                reason,
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}
