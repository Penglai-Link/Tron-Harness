use rusqlite::{OptionalExtension, params};

use crate::domain::resource::{ResourceError, ResourceKind};

use super::storage_error;

pub(super) fn ensure_revision_columns(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
) -> Result<(), ResourceError> {
    for table in [
        "resource_files",
        "resource_data_source_connections",
        "resource_servers",
        "resource_server_accounts",
    ] {
        let statement = format!("PRAGMA table_info({table})");
        let mut query = transaction.prepare(&statement).map_err(storage_error)?;
        let columns = query
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        if !columns.iter().any(|column| column == "revision") {
            let statement =
                format!("ALTER TABLE {table} ADD COLUMN revision INTEGER NOT NULL DEFAULT 0");
            transaction
                .execute_batch(&statement)
                .map_err(storage_error)?;
        }
    }
    Ok(())
}

pub(super) fn ensure_server_account_origin_column(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
) -> Result<(), ResourceError> {
    let mut query = transaction
        .prepare("PRAGMA table_info(resource_server_accounts)")
        .map_err(storage_error)?;
    let columns = query
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    if !columns
        .iter()
        .any(|column| column == "server_workspace_code")
    {
        transaction
            .execute_batch(
                "ALTER TABLE resource_server_accounts
                 ADD COLUMN server_workspace_code TEXT COLLATE BINARY;
                 UPDATE resource_server_accounts
                 SET server_workspace_code = workspace_code
                 WHERE server_workspace_code IS NULL OR trim(server_workspace_code) = '';",
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

pub(super) fn ensure_server_account_parent_guards(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
) -> Result<(), ResourceError> {
    transaction
        .execute_batch(
            "CREATE TRIGGER IF NOT EXISTS resource_server_accounts_require_parent_insert
             BEFORE INSERT ON resource_server_accounts
             FOR EACH ROW
             WHEN NEW.server_workspace_code IS NULL
                  OR NOT EXISTS (
                      SELECT 1 FROM resource_servers
                      WHERE workspace_code = NEW.server_workspace_code
                        AND resource_code = NEW.server_code
                  )
             BEGIN
                 SELECT RAISE(ABORT, 'resource server account parent does not exist');
             END;

             CREATE TRIGGER IF NOT EXISTS resource_server_accounts_require_parent_update
             BEFORE UPDATE OF server_workspace_code, server_code ON resource_server_accounts
             FOR EACH ROW
             WHEN NEW.server_workspace_code IS NULL
                  OR NOT EXISTS (
                      SELECT 1 FROM resource_servers
                      WHERE workspace_code = NEW.server_workspace_code
                        AND resource_code = NEW.server_code
                  )
             BEGIN
                 SELECT RAISE(ABORT, 'resource server account parent does not exist');
             END;

             CREATE TRIGGER IF NOT EXISTS resource_servers_prevent_parent_delete
             BEFORE DELETE ON resource_servers
             FOR EACH ROW
             WHEN EXISTS (
                 SELECT 1 FROM resource_server_accounts
                 WHERE server_workspace_code = OLD.workspace_code
                   AND server_code = OLD.resource_code
             )
             BEGIN
                 SELECT RAISE(ABORT, 'resource server has accounts');
             END;",
        )
        .map_err(storage_error)
}

pub(super) fn claim_resource_code(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    workspace_code: &str,
    resource_code: &str,
    kind: &str,
) -> Result<(), ResourceError> {
    let existing = transaction
        .query_row(
            "SELECT kind FROM resource_code_registry
             WHERE workspace_code = ?1 AND resource_code = ?2",
            params![workspace_code, resource_code],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?;
    match existing {
        Some(existing) if existing == kind => Ok(()),
        Some(_) => Err(ResourceError::Conflict),
        None => {
            transaction
                .execute(
                    "INSERT INTO resource_code_registry (workspace_code, resource_code, kind)
                     VALUES (?1, ?2, ?3)",
                    params![workspace_code, resource_code, kind],
                )
                .map_err(storage_error)?;
            Ok(())
        }
    }
}

pub(super) fn release_resource_code(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    workspace_code: &str,
    resource_code: &str,
) -> Result<(), ResourceError> {
    transaction
        .execute(
            "DELETE FROM resource_code_registry
             WHERE workspace_code = ?1 AND resource_code = ?2",
            params![workspace_code, resource_code],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(super) fn backfill_resource_code_registry(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
) -> Result<(), ResourceError> {
    for (query, fallback_kind) in [
        (
            "SELECT workspace_code, resource_code, kind FROM resource_files",
            None,
        ),
        (
            "SELECT workspace_code, resource_code, kind FROM resource_data_source_connections",
            None,
        ),
        (
            "SELECT workspace_code, resource_code, '' FROM resource_servers",
            Some(ResourceKind::ServerConnection.as_str()),
        ),
        (
            "SELECT workspace_code, resource_code, '' FROM resource_server_accounts",
            Some(ResourceKind::ServerAccount.as_str()),
        ),
    ] {
        let mut statement = transaction.prepare(query).map_err(storage_error)?;
        let records = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        for (workspace_code, resource_code, kind) in records {
            let kind = fallback_kind.unwrap_or(&kind);
            match claim_resource_code(transaction, &workspace_code, &resource_code, kind) {
                Ok(()) => {}
                Err(ResourceError::Conflict) => record_simplified_code_collision_issue(
                    transaction,
                    &workspace_code,
                    &resource_code,
                    kind,
                )?,
                Err(error) => return Err(error),
            }
        }
    }
    Ok(())
}

fn record_simplified_code_collision_issue(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    workspace_code: &str,
    resource_code: &str,
    kind: &str,
) -> Result<(), ResourceError> {
    transaction
        .execute(
            "INSERT OR REPLACE INTO resource_management_migration_issues (
                scope_kind, scope_code, resource_code, resource_version, reason
            ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                "SIMPLIFIED",
                workspace_code,
                resource_code,
                "registry",
                format!(
                    "resource code also occurs in another simplified resource kind; {kind} was not registered"
                ),
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}
