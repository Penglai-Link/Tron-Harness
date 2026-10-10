use rusqlite::{OptionalExtension, params};

use crate::domain::resource::ResourceError;

use super::storage_error;

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
