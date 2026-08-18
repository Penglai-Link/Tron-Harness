use std::collections::HashSet;

use rusqlite::Connection;

use crate::domain::resource::{Resource, ResourceError, ResourceLifecycleStatus};

use super::resource_repository::{
    active_resource_version, ensure_definition_remains_active_and_enabled,
    ensure_resource_definition_code, ensure_resource_definition_exists, insert_resource,
    resource_draft_exists, storage_error,
};

pub(super) fn insert_active_resource_bundle(
    connection: &mut Connection,
    resources: &[Resource],
) -> Result<(), ResourceError> {
    if resources.is_empty() {
        return Err(ResourceError::InvalidInput);
    }
    let mut resource_codes = HashSet::new();
    let tx = connection.transaction().map_err(storage_error)?;
    for resource in resources {
        if resource.lifecycle_status != ResourceLifecycleStatus::Active
            || !resource_codes.insert((
                resource.key.scope_kind,
                resource.key.scope_code.clone(),
                resource.key.resource_code.clone(),
            ))
        {
            return Err(ResourceError::InvalidInput);
        }
        ensure_resource_definition_code(&tx, resource)?;
        ensure_resource_definition_exists(&tx, resource)?;
        ensure_definition_remains_active_and_enabled(&tx, &resource.definition_key)?;
        if active_resource_version(&tx, &resource.key)?.is_some()
            || resource_draft_exists(&tx, &resource.key)?
        {
            return Err(ResourceError::Conflict);
        }
        insert_resource(&tx, resource)?;
    }
    tx.commit().map_err(storage_error)
}
