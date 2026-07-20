use std::{fs, path::Path};

use rusqlite::{Connection, Error as SqliteError, OptionalExtension, Transaction, params};
use serde_json::Value;

use crate::{
    application::resource::{ResourceActivationOutcome, ResourceRepository},
    domain::resource::{
        Resource, ResourceActivationResult, ResourceBinaryMapping, ResourceDefinition,
        ResourceDefinitionKey, ResourceDefinitionStatus, ResourceError, ResourceKey,
        ResourceLifecycleStatus, ResourceScopeKind, ResourceValidationStatus,
    },
    shared::time::{
        option_timestamp_from_millis, option_timestamp_to_millis, timestamp_from_millis,
        timestamp_to_millis,
    },
};

use super::migrations::RESOURCE_MANAGEMENT_MIGRATION;

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
            .map_err(storage_error)
    }
}

impl ResourceRepository for SqliteResourceRepository {
    fn list_definitions(&mut self) -> Result<Vec<ResourceDefinition>, ResourceError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT scope_kind, scope_code, definition_code, definition_version, name, description,
                        definition_status, allow_inheritance, attributes_schema, binary_schema,
                        binary_storage_profile, draft_revision, validation_status, lifecycle_status,
                        created_at, updated_at, published_at
                 FROM resource_definitions
                 ORDER BY scope_kind, scope_code, definition_code, created_at, definition_version",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], definition_from_row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn list_resources(&mut self) -> Result<Vec<Resource>, ResourceError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT scope_kind, scope_code, resource_code, resource_version,
                        definition_scope_kind, definition_scope_code, definition_code, definition_version,
                        based_on_resource_version, name, description, attributes_json, allow_inheritance,
                        draft_revision, validation_status, lifecycle_status, created_at, updated_at, published_at
                 FROM resources
                 ORDER BY scope_kind, scope_code, resource_code, created_at, resource_version",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], resource_from_row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn list_binary_mappings(&mut self) -> Result<Vec<ResourceBinaryMapping>, ResourceError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT scope_kind, scope_code, resource_code, resource_version, binary_name,
                        storage_locator, byte_size, created_at
                 FROM resource_binary_mappings
                 ORDER BY scope_kind, scope_code, resource_code, resource_version, binary_name",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], mapping_from_row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn insert_definition(&mut self, definition: &ResourceDefinition) -> Result<(), ResourceError> {
        self.connection
            .execute(
                "INSERT INTO resource_definitions (
                    scope_kind, scope_code, definition_code, definition_version, name, description,
                    definition_status, allow_inheritance, attributes_schema, binary_schema,
                    binary_storage_profile, draft_revision, validation_status, lifecycle_status,
                    created_at, updated_at, published_at
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17
                )",
                params![
                    scope_kind_to_storage(definition.key.scope_kind),
                    definition.key.scope_code,
                    definition.key.definition_code,
                    definition.key.definition_version,
                    definition.name,
                    definition.description,
                    definition_status_to_storage(definition.definition_status),
                    bool_to_int(definition.allow_inheritance),
                    value_to_storage(&definition.attributes_schema)?,
                    value_to_storage(&definition.binary_schema)?,
                    definition.binary_storage_profile,
                    definition.draft_revision,
                    validation_status_to_storage(definition.validation_status),
                    lifecycle_status_to_storage(definition.lifecycle_status),
                    timestamp_to_millis(&definition.created_at),
                    timestamp_to_millis(&definition.updated_at),
                    option_timestamp_to_millis(definition.published_at.as_deref()),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn upgrade_builtin_definition(
        &mut self,
        expected_legacy: &ResourceDefinition,
        replacement: &ResourceDefinition,
    ) -> Result<bool, ResourceError> {
        if expected_legacy.key.scope_kind != replacement.key.scope_kind
            || expected_legacy.key.scope_code != replacement.key.scope_code
            || expected_legacy.key.definition_code != replacement.key.definition_code
            || expected_legacy.lifecycle_status != ResourceLifecycleStatus::Active
            || replacement.lifecycle_status != ResourceLifecycleStatus::Active
        {
            return Err(ResourceError::InvalidInput);
        }
        let tx = self.connection.transaction().map_err(storage_error)?;
        let current = tx
            .query_row(
                "SELECT scope_kind, scope_code, definition_code, definition_version, name, description,
                        definition_status, allow_inheritance, attributes_schema, binary_schema,
                        binary_storage_profile, draft_revision, validation_status, lifecycle_status,
                        created_at, updated_at, published_at
                 FROM resource_definitions
                 WHERE scope_kind = ?1 AND scope_code = ?2 AND definition_code = ?3 AND definition_version = ?4",
                params![
                    scope_kind_to_storage(expected_legacy.key.scope_kind),
                    expected_legacy.key.scope_code,
                    expected_legacy.key.definition_code,
                    expected_legacy.key.definition_version,
                ],
                definition_from_row,
            )
            .optional()
            .map_err(storage_error)?;
        if current.as_ref() != Some(expected_legacy)
            || definition_draft_exists(&tx, &expected_legacy.key)?
        {
            tx.commit().map_err(storage_error)?;
            return Ok(false);
        }
        demote_active_definition(&tx, &expected_legacy.key, &replacement.updated_at)?;
        tx.execute(
            "INSERT INTO resource_definitions (
                scope_kind, scope_code, definition_code, definition_version, name, description,
                definition_status, allow_inheritance, attributes_schema, binary_schema,
                binary_storage_profile, draft_revision, validation_status, lifecycle_status,
                created_at, updated_at, published_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17
            )",
            params![
                scope_kind_to_storage(replacement.key.scope_kind),
                replacement.key.scope_code,
                replacement.key.definition_code,
                replacement.key.definition_version,
                replacement.name,
                replacement.description,
                definition_status_to_storage(replacement.definition_status),
                bool_to_int(replacement.allow_inheritance),
                value_to_storage(&replacement.attributes_schema)?,
                value_to_storage(&replacement.binary_schema)?,
                replacement.binary_storage_profile,
                replacement.draft_revision,
                validation_status_to_storage(replacement.validation_status),
                lifecycle_status_to_storage(replacement.lifecycle_status),
                timestamp_to_millis(&replacement.created_at),
                timestamp_to_millis(&replacement.updated_at),
                option_timestamp_to_millis(replacement.published_at.as_deref()),
            ],
        )
        .map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        Ok(true)
    }

    fn update_definition_draft(
        &mut self,
        definition: &ResourceDefinition,
        expected_draft_revision: i64,
    ) -> Result<(), ResourceError> {
        let updated = self
            .connection
            .execute(
                "UPDATE resource_definitions
                 SET name = ?5, description = ?6, definition_status = ?7, allow_inheritance = ?8,
                     attributes_schema = ?9, binary_schema = ?10, binary_storage_profile = ?11,
                     draft_revision = ?12, validation_status = ?13, updated_at = ?14
                 WHERE scope_kind = ?1 AND scope_code = ?2 AND definition_code = ?3
                   AND definition_version = ?4 AND lifecycle_status = 'DRAFT'
                   AND draft_revision = ?15",
                params![
                    scope_kind_to_storage(definition.key.scope_kind),
                    definition.key.scope_code,
                    definition.key.definition_code,
                    definition.key.definition_version,
                    definition.name,
                    definition.description,
                    definition_status_to_storage(definition.definition_status),
                    bool_to_int(definition.allow_inheritance),
                    value_to_storage(&definition.attributes_schema)?,
                    value_to_storage(&definition.binary_schema)?,
                    definition.binary_storage_profile,
                    definition.draft_revision,
                    validation_status_to_storage(definition.validation_status),
                    timestamp_to_millis(&definition.updated_at),
                    expected_draft_revision,
                ],
            )
            .map_err(storage_error)?;
        if updated == 0 {
            return Err(ResourceError::DraftRevisionConflict);
        }
        Ok(())
    }

    fn activate_definition(
        &mut self,
        key: &ResourceDefinitionKey,
        expected_draft_revision: Option<i64>,
        expected_active_version: Option<&str>,
        published_at: &str,
    ) -> Result<ResourceActivationOutcome, ResourceError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        let (lifecycle, draft_revision) = definition_lifecycle(&tx, key)?;
        let prior_active = active_definition_version(&tx, key)?;
        match lifecycle {
            ResourceLifecycleStatus::Active => {
                tx.commit().map_err(storage_error)?;
                Ok(ResourceActivationOutcome {
                    activation_result: ResourceActivationResult::AlreadyActive,
                    previous_active_version: None,
                })
            }
            ResourceLifecycleStatus::Draft => {
                if expected_draft_revision != Some(draft_revision)
                    || expected_active_version.is_some()
                {
                    return Err(ResourceError::DraftRevisionConflict);
                }
                demote_active_definition(&tx, key, published_at)?;
                let updated = tx
                    .execute(
                        "UPDATE resource_definitions
                         SET lifecycle_status = 'ACTIVE', validation_status = 'VALID', published_at = ?5, updated_at = ?5
                         WHERE scope_kind = ?1 AND scope_code = ?2 AND definition_code = ?3
                           AND definition_version = ?4 AND lifecycle_status = 'DRAFT'",
                        params![
                            scope_kind_to_storage(key.scope_kind), key.scope_code, key.definition_code,
                            key.definition_version, timestamp_to_millis(published_at)
                        ],
                    )
                    .map_err(storage_error)?;
                if updated != 1 {
                    return Err(ResourceError::Conflict);
                }
                tx.commit().map_err(storage_error)?;
                Ok(ResourceActivationOutcome {
                    activation_result: ResourceActivationResult::DraftPublished,
                    previous_active_version: prior_active,
                })
            }
            ResourceLifecycleStatus::Historical => {
                if expected_draft_revision.is_some() || expected_active_version.is_none() {
                    return Err(ResourceError::ActiveVersionConflict);
                }
                if definition_draft_exists(&tx, key)? {
                    return Err(ResourceError::Conflict);
                }
                if prior_active.as_deref() != expected_active_version {
                    return Err(ResourceError::ActiveVersionConflict);
                }
                demote_active_definition(&tx, key, published_at)?;
                let updated = tx
                    .execute(
                        "UPDATE resource_definitions
                         SET lifecycle_status = 'ACTIVE', validation_status = 'VALID', published_at = ?5, updated_at = ?5
                         WHERE scope_kind = ?1 AND scope_code = ?2 AND definition_code = ?3
                           AND definition_version = ?4 AND lifecycle_status = 'HISTORICAL'",
                        params![
                            scope_kind_to_storage(key.scope_kind), key.scope_code, key.definition_code,
                            key.definition_version, timestamp_to_millis(published_at)
                        ],
                    )
                    .map_err(storage_error)?;
                if updated != 1 {
                    return Err(ResourceError::Conflict);
                }
                tx.commit().map_err(storage_error)?;
                Ok(ResourceActivationOutcome {
                    activation_result: ResourceActivationResult::HistoricalReactivated,
                    previous_active_version: prior_active,
                })
            }
        }
    }

    fn discard_definition_draft(
        &mut self,
        key: &ResourceDefinitionKey,
        expected_draft_revision: i64,
    ) -> Result<bool, ResourceError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        let existing = definition_lifecycle_optional(&tx, key)?;
        let Some((lifecycle, revision)) = existing else {
            tx.commit().map_err(storage_error)?;
            return Ok(false);
        };
        if lifecycle != ResourceLifecycleStatus::Draft {
            return Err(ResourceError::LifecycleDenied);
        }
        if revision != expected_draft_revision {
            return Err(ResourceError::DraftRevisionConflict);
        }
        tx.execute(
            "DELETE FROM resource_definitions
             WHERE scope_kind = ?1 AND scope_code = ?2 AND definition_code = ?3
               AND definition_version = ?4 AND lifecycle_status = 'DRAFT' AND draft_revision = ?5",
            params![
                scope_kind_to_storage(key.scope_kind),
                key.scope_code,
                key.definition_code,
                key.definition_version,
                expected_draft_revision
            ],
        )
        .map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        Ok(true)
    }

    fn commit_active_definition(
        &mut self,
        definition: &ResourceDefinition,
        expected_local_active_version: Option<&str>,
        expected_effective_source: &ResourceDefinitionKey,
    ) -> Result<ResourceActivationOutcome, ResourceError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        ensure_definition_remains_active(&tx, expected_effective_source)?;
        let active = active_definition_version(&tx, &definition.key)?;
        if active.as_deref() != expected_local_active_version {
            return Err(ResourceError::ActiveVersionConflict);
        }
        if definition_draft_exists(&tx, &definition.key)? {
            return Err(ResourceError::Conflict);
        }
        demote_active_definition(&tx, &definition.key, &definition.updated_at)?;
        insert_definition(&tx, definition)?;
        tx.commit().map_err(storage_error)?;
        Ok(ResourceActivationOutcome {
            activation_result: ResourceActivationResult::DirectCommitted,
            previous_active_version: active,
        })
    }

    fn delete_definition_group(
        &mut self,
        scope_kind: ResourceScopeKind,
        scope_code: &str,
        definition_code: &str,
        expected_active_version: &str,
    ) -> Result<bool, ResourceError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        let key = ResourceDefinitionKey {
            scope_kind,
            scope_code: scope_code.to_string(),
            definition_code: definition_code.to_string(),
            definition_version: expected_active_version.to_string(),
        };
        let active = active_definition_version(&tx, &key)?;
        if active.as_deref() != Some(expected_active_version) {
            return Err(ResourceError::ActiveVersionConflict);
        }
        if definition_draft_exists(&tx, &key)? {
            return Err(ResourceError::Conflict);
        }
        let active_status: String = tx
            .query_row(
                "SELECT definition_status FROM resource_definitions
                 WHERE scope_kind = ?1 AND scope_code = ?2 AND definition_code = ?3
                   AND definition_version = ?4 AND lifecycle_status = 'ACTIVE'",
                params![
                    scope_kind_to_storage(scope_kind),
                    scope_code,
                    definition_code,
                    expected_active_version
                ],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if definition_status_from_storage(&active_status)? != ResourceDefinitionStatus::Disabled {
            return Err(ResourceError::LifecycleDenied);
        }
        let references: i64 = tx
            .query_row(
                "SELECT count(*) FROM resources
                 WHERE definition_scope_kind = ?1 AND definition_scope_code = ?2
                   AND definition_code = ?3",
                params![
                    scope_kind_to_storage(scope_kind),
                    scope_code,
                    definition_code
                ],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if references > 0 {
            return Err(ResourceError::DefinitionInUse);
        }
        let deleted = tx
            .execute(
                "DELETE FROM resource_definitions
                 WHERE scope_kind = ?1 AND scope_code = ?2 AND definition_code = ?3",
                params![
                    scope_kind_to_storage(scope_kind),
                    scope_code,
                    definition_code
                ],
            )
            .map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        Ok(deleted > 0)
    }

    fn definition_resource_reference_count(
        &mut self,
        scope_kind: ResourceScopeKind,
        scope_code: &str,
        definition_code: &str,
    ) -> Result<usize, ResourceError> {
        let count: i64 = self
            .connection
            .query_row(
                "SELECT count(*) FROM resources
                 WHERE definition_scope_kind = ?1 AND definition_scope_code = ?2
                   AND definition_code = ?3",
                params![
                    scope_kind_to_storage(scope_kind),
                    scope_code,
                    definition_code
                ],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        Ok(count as usize)
    }

    fn insert_resource(
        &mut self,
        resource: &Resource,
        mappings: &[ResourceBinaryMapping],
    ) -> Result<(), ResourceError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        ensure_resource_definition_code(&tx, resource)?;
        ensure_resource_definition_exists(&tx, resource)?;
        insert_resource(&tx, resource)?;
        replace_mappings(&tx, &resource.key, mappings)?;
        tx.commit().map_err(storage_error)
    }

    fn update_resource_draft(
        &mut self,
        resource: &Resource,
        mappings: &[ResourceBinaryMapping],
        expected_draft_revision: i64,
    ) -> Result<(), ResourceError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        let updated = tx
            .execute(
                "UPDATE resources
                 SET name = ?5, description = ?6, attributes_json = ?7, allow_inheritance = ?8,
                     draft_revision = ?9, validation_status = ?10, updated_at = ?11
                 WHERE scope_kind = ?1 AND scope_code = ?2 AND resource_code = ?3
                   AND resource_version = ?4 AND lifecycle_status = 'DRAFT' AND draft_revision = ?12",
                params![
                    scope_kind_to_storage(resource.key.scope_kind), resource.key.scope_code,
                    resource.key.resource_code, resource.key.resource_version, resource.name,
                    resource.description, value_to_storage(&resource.attributes)?,
                    bool_to_int(resource.allow_inheritance), resource.draft_revision,
                    validation_status_to_storage(resource.validation_status),
                    timestamp_to_millis(&resource.updated_at), expected_draft_revision,
                ],
            )
            .map_err(storage_error)?;
        if updated != 1 {
            return Err(ResourceError::DraftRevisionConflict);
        }
        replace_mappings(&tx, &resource.key, mappings)?;
        tx.commit().map_err(storage_error)
    }

    fn activate_resource(
        &mut self,
        key: &ResourceKey,
        expected_draft_revision: Option<i64>,
        expected_active_version: Option<&str>,
        published_at: &str,
    ) -> Result<ResourceActivationOutcome, ResourceError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        let (lifecycle, draft_revision) = resource_lifecycle(&tx, key)?;
        let prior_active = active_resource_version(&tx, key)?;
        match lifecycle {
            ResourceLifecycleStatus::Active => {
                tx.commit().map_err(storage_error)?;
                Ok(ResourceActivationOutcome {
                    activation_result: ResourceActivationResult::AlreadyActive,
                    previous_active_version: None,
                })
            }
            ResourceLifecycleStatus::Draft => {
                if expected_draft_revision != Some(draft_revision)
                    || expected_active_version.is_some()
                {
                    return Err(ResourceError::DraftRevisionConflict);
                }
                demote_active_resource(&tx, key, published_at)?;
                let updated = tx
                    .execute(
                        "UPDATE resources
                         SET lifecycle_status = 'ACTIVE', validation_status = 'VALID', published_at = ?5, updated_at = ?5
                         WHERE scope_kind = ?1 AND scope_code = ?2 AND resource_code = ?3
                           AND resource_version = ?4 AND lifecycle_status = 'DRAFT'",
                        params![
                            scope_kind_to_storage(key.scope_kind), key.scope_code, key.resource_code,
                            key.resource_version, timestamp_to_millis(published_at)
                        ],
                    )
                    .map_err(storage_error)?;
                if updated != 1 {
                    return Err(ResourceError::Conflict);
                }
                tx.commit().map_err(storage_error)?;
                Ok(ResourceActivationOutcome {
                    activation_result: ResourceActivationResult::DraftPublished,
                    previous_active_version: prior_active,
                })
            }
            ResourceLifecycleStatus::Historical => {
                if expected_draft_revision.is_some() || expected_active_version.is_none() {
                    return Err(ResourceError::ActiveVersionConflict);
                }
                if resource_draft_exists(&tx, key)? {
                    return Err(ResourceError::Conflict);
                }
                if prior_active.as_deref() != expected_active_version {
                    return Err(ResourceError::ActiveVersionConflict);
                }
                demote_active_resource(&tx, key, published_at)?;
                let updated = tx
                    .execute(
                        "UPDATE resources
                         SET lifecycle_status = 'ACTIVE', validation_status = 'VALID', published_at = ?5, updated_at = ?5
                         WHERE scope_kind = ?1 AND scope_code = ?2 AND resource_code = ?3
                           AND resource_version = ?4 AND lifecycle_status = 'HISTORICAL'",
                        params![
                            scope_kind_to_storage(key.scope_kind), key.scope_code, key.resource_code,
                            key.resource_version, timestamp_to_millis(published_at)
                        ],
                    )
                    .map_err(storage_error)?;
                if updated != 1 {
                    return Err(ResourceError::Conflict);
                }
                tx.commit().map_err(storage_error)?;
                Ok(ResourceActivationOutcome {
                    activation_result: ResourceActivationResult::HistoricalReactivated,
                    previous_active_version: prior_active,
                })
            }
        }
    }

    fn discard_resource_draft(
        &mut self,
        key: &ResourceKey,
        expected_draft_revision: i64,
    ) -> Result<(bool, Vec<String>), ResourceError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        let existing = resource_lifecycle_optional(&tx, key)?;
        let Some((lifecycle, revision)) = existing else {
            tx.commit().map_err(storage_error)?;
            return Ok((false, Vec::new()));
        };
        if lifecycle != ResourceLifecycleStatus::Draft {
            return Err(ResourceError::LifecycleDenied);
        }
        if revision != expected_draft_revision {
            return Err(ResourceError::DraftRevisionConflict);
        }
        let locators = mapping_locators(&tx, key)?;
        delete_mappings(&tx, key)?;
        tx.execute(
            "DELETE FROM resources
             WHERE scope_kind = ?1 AND scope_code = ?2 AND resource_code = ?3
               AND resource_version = ?4 AND lifecycle_status = 'DRAFT' AND draft_revision = ?5",
            params![
                scope_kind_to_storage(key.scope_kind),
                key.scope_code,
                key.resource_code,
                key.resource_version,
                expected_draft_revision
            ],
        )
        .map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        Ok((true, locators))
    }

    fn commit_active_resource(
        &mut self,
        resource: &Resource,
        mappings: &[ResourceBinaryMapping],
        expected_local_active_version: Option<&str>,
        expected_effective_source: Option<&ResourceKey>,
        expected_enabled_definition: Option<&ResourceDefinitionKey>,
    ) -> Result<ResourceActivationOutcome, ResourceError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        ensure_resource_definition_code(&tx, resource)?;
        ensure_resource_definition_exists(&tx, resource)?;
        if let Some(expected_effective_source) = expected_effective_source {
            ensure_resource_remains_active(&tx, expected_effective_source)?;
        }
        if let Some(expected_enabled_definition) = expected_enabled_definition {
            ensure_definition_remains_active_and_enabled(&tx, expected_enabled_definition)?;
        }
        let active = active_resource_version(&tx, &resource.key)?;
        if active.as_deref() != expected_local_active_version {
            return Err(ResourceError::ActiveVersionConflict);
        }
        if resource_draft_exists(&tx, &resource.key)? {
            return Err(ResourceError::Conflict);
        }
        demote_active_resource(&tx, &resource.key, &resource.updated_at)?;
        insert_resource(&tx, resource)?;
        replace_mappings(&tx, &resource.key, mappings)?;
        tx.commit().map_err(storage_error)?;
        Ok(ResourceActivationOutcome {
            activation_result: ResourceActivationResult::DirectCommitted,
            previous_active_version: active,
        })
    }

    fn delete_resource_group(
        &mut self,
        scope_kind: ResourceScopeKind,
        scope_code: &str,
        resource_code: &str,
        expected_active_version: Option<&str>,
        expected_draft_version: Option<&str>,
        expected_draft_revision: Option<i64>,
    ) -> Result<(bool, Vec<String>), ResourceError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        let key = ResourceKey {
            scope_kind,
            scope_code: scope_code.to_string(),
            resource_code: resource_code.to_string(),
            resource_version: String::new(),
        };
        let active = active_resource_version(&tx, &key)?;
        if active.as_deref() != expected_active_version {
            return Err(ResourceError::ActiveVersionConflict);
        }
        let draft: Option<(String, i64)> = tx
            .query_row(
                "SELECT resource_version, draft_revision FROM resources
                 WHERE scope_kind = ?1 AND scope_code = ?2 AND resource_code = ?3
                   AND lifecycle_status = 'DRAFT'",
                params![scope_kind_to_storage(scope_kind), scope_code, resource_code],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage_error)?;
        match draft {
            Some((version, revision))
                if expected_draft_version == Some(version.as_str())
                    && expected_draft_revision == Some(revision) => {}
            Some(_) => return Err(ResourceError::DraftRevisionConflict),
            None if expected_draft_version.is_some() || expected_draft_revision.is_some() => {
                return Err(ResourceError::DraftRevisionConflict);
            }
            None => {}
        }
        let mut statement = tx
            .prepare(
                "SELECT storage_locator FROM resource_binary_mappings
                 WHERE scope_kind = ?1 AND scope_code = ?2 AND resource_code = ?3",
            )
            .map_err(storage_error)?;
        let locators = statement
            .query_map(
                params![scope_kind_to_storage(scope_kind), scope_code, resource_code],
                |row| row.get(0),
            )
            .map_err(storage_error)?
            .collect::<Result<Vec<String>, _>>()
            .map_err(storage_error)?;
        drop(statement);
        tx.execute(
            "DELETE FROM resource_binary_mappings
             WHERE scope_kind = ?1 AND scope_code = ?2 AND resource_code = ?3",
            params![scope_kind_to_storage(scope_kind), scope_code, resource_code],
        )
        .map_err(storage_error)?;
        let deleted = tx
            .execute(
                "DELETE FROM resources
                 WHERE scope_kind = ?1 AND scope_code = ?2 AND resource_code = ?3",
                params![scope_kind_to_storage(scope_kind), scope_code, resource_code],
            )
            .map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        Ok((deleted > 0, locators))
    }

    fn locator_reference_count(&mut self, locator: &str) -> Result<usize, ResourceError> {
        let count = self
            .connection
            .query_row(
                "SELECT count(*) FROM resource_binary_mappings WHERE storage_locator = ?1",
                [locator],
                |row| row.get::<_, i64>(0),
            )
            .map_err(storage_error)?;
        Ok(count as usize)
    }
}

fn ensure_resource_definition_code(
    tx: &Transaction<'_>,
    resource: &Resource,
) -> Result<(), ResourceError> {
    let existing_definition_code = tx
        .query_row(
            "SELECT definition_code FROM resources
             WHERE scope_kind = ?1 AND scope_code = ?2 AND resource_code = ?3
             LIMIT 1",
            params![
                scope_kind_to_storage(resource.key.scope_kind),
                resource.key.scope_code,
                resource.key.resource_code,
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?;
    if existing_definition_code
        .as_deref()
        .is_some_and(|definition_code| definition_code != resource.definition_key.definition_code)
    {
        return Err(ResourceError::Conflict);
    }
    Ok(())
}

fn ensure_resource_definition_exists(
    tx: &Transaction<'_>,
    resource: &Resource,
) -> Result<(), ResourceError> {
    let exists = tx
        .query_row(
            "SELECT 1 FROM resource_definitions
             WHERE scope_kind = ?1 AND scope_code = ?2 AND definition_code = ?3
               AND definition_version = ?4",
            params![
                scope_kind_to_storage(resource.definition_key.scope_kind),
                resource.definition_key.scope_code,
                resource.definition_key.definition_code,
                resource.definition_key.definition_version,
            ],
            |_| Ok(()),
        )
        .optional()
        .map_err(storage_error)?
        .is_some();
    if !exists {
        return Err(ResourceError::NotFound);
    }
    Ok(())
}

fn insert_definition(
    tx: &Transaction<'_>,
    definition: &ResourceDefinition,
) -> Result<(), ResourceError> {
    tx.execute(
        "INSERT INTO resource_definitions (
            scope_kind, scope_code, definition_code, definition_version, name, description,
            definition_status, allow_inheritance, attributes_schema, binary_schema,
            binary_storage_profile, draft_revision, validation_status, lifecycle_status,
            created_at, updated_at, published_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17
        )",
        params![
            scope_kind_to_storage(definition.key.scope_kind),
            definition.key.scope_code,
            definition.key.definition_code,
            definition.key.definition_version,
            definition.name,
            definition.description,
            definition_status_to_storage(definition.definition_status),
            bool_to_int(definition.allow_inheritance),
            value_to_storage(&definition.attributes_schema)?,
            value_to_storage(&definition.binary_schema)?,
            definition.binary_storage_profile,
            definition.draft_revision,
            validation_status_to_storage(definition.validation_status),
            lifecycle_status_to_storage(definition.lifecycle_status),
            timestamp_to_millis(&definition.created_at),
            timestamp_to_millis(&definition.updated_at),
            option_timestamp_to_millis(definition.published_at.as_deref()),
        ],
    )
    .map_err(storage_error)?;
    Ok(())
}

fn insert_resource(tx: &Transaction<'_>, resource: &Resource) -> Result<(), ResourceError> {
    tx.execute(
        "INSERT INTO resources (
            scope_kind, scope_code, resource_code, resource_version,
            definition_scope_kind, definition_scope_code, definition_code, definition_version,
            based_on_resource_version, name, description, attributes_json, allow_inheritance,
            draft_revision, validation_status, lifecycle_status, created_at, updated_at, published_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19
        )",
        params![
            scope_kind_to_storage(resource.key.scope_kind), resource.key.scope_code,
            resource.key.resource_code, resource.key.resource_version,
            scope_kind_to_storage(resource.definition_key.scope_kind), resource.definition_key.scope_code,
            resource.definition_key.definition_code, resource.definition_key.definition_version,
            resource.based_on_resource_version, resource.name, resource.description,
            value_to_storage(&resource.attributes)?, bool_to_int(resource.allow_inheritance),
            resource.draft_revision, validation_status_to_storage(resource.validation_status),
            lifecycle_status_to_storage(resource.lifecycle_status), timestamp_to_millis(&resource.created_at),
            timestamp_to_millis(&resource.updated_at), option_timestamp_to_millis(resource.published_at.as_deref()),
        ],
    )
    .map_err(storage_error)?;
    Ok(())
}

fn replace_mappings(
    tx: &Transaction<'_>,
    key: &ResourceKey,
    mappings: &[ResourceBinaryMapping],
) -> Result<(), ResourceError> {
    delete_mappings(tx, key)?;
    for mapping in mappings {
        tx.execute(
            "INSERT INTO resource_binary_mappings (
                scope_kind, scope_code, resource_code, resource_version, binary_name,
                storage_locator, byte_size, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                scope_kind_to_storage(key.scope_kind),
                key.scope_code,
                key.resource_code,
                key.resource_version,
                mapping.binary_name,
                mapping.storage_locator,
                mapping.byte_size as i64,
                timestamp_to_millis(&mapping.created_at),
            ],
        )
        .map_err(storage_error)?;
    }
    Ok(())
}

fn delete_mappings(tx: &Transaction<'_>, key: &ResourceKey) -> Result<(), ResourceError> {
    tx.execute(
        "DELETE FROM resource_binary_mappings
         WHERE scope_kind = ?1 AND scope_code = ?2 AND resource_code = ?3 AND resource_version = ?4",
        params![
            scope_kind_to_storage(key.scope_kind), key.scope_code, key.resource_code,
            key.resource_version
        ],
    )
    .map_err(storage_error)?;
    Ok(())
}

fn mapping_locators(tx: &Transaction<'_>, key: &ResourceKey) -> Result<Vec<String>, ResourceError> {
    let mut statement = tx
        .prepare(
            "SELECT storage_locator FROM resource_binary_mappings
             WHERE scope_kind = ?1 AND scope_code = ?2 AND resource_code = ?3 AND resource_version = ?4",
        )
        .map_err(storage_error)?;
    statement
        .query_map(
            params![
                scope_kind_to_storage(key.scope_kind),
                key.scope_code,
                key.resource_code,
                key.resource_version
            ],
            |row| row.get(0),
        )
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)
}

fn definition_lifecycle(
    tx: &Transaction<'_>,
    key: &ResourceDefinitionKey,
) -> Result<(ResourceLifecycleStatus, i64), ResourceError> {
    definition_lifecycle_optional(tx, key)?.ok_or(ResourceError::NotFound)
}

fn definition_lifecycle_optional(
    tx: &Transaction<'_>,
    key: &ResourceDefinitionKey,
) -> Result<Option<(ResourceLifecycleStatus, i64)>, ResourceError> {
    tx.query_row(
        "SELECT lifecycle_status, draft_revision FROM resource_definitions
         WHERE scope_kind = ?1 AND scope_code = ?2 AND definition_code = ?3 AND definition_version = ?4",
        params![
            scope_kind_to_storage(key.scope_kind), key.scope_code, key.definition_code, key.definition_version
        ],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
    )
    .optional()
    .map_err(storage_error)?
    .map(|(lifecycle, revision)| Ok((lifecycle_status_from_storage(&lifecycle)?, revision)))
    .transpose()
}

fn active_definition_version(
    tx: &Transaction<'_>,
    key: &ResourceDefinitionKey,
) -> Result<Option<String>, ResourceError> {
    tx.query_row(
        "SELECT definition_version FROM resource_definitions
         WHERE scope_kind = ?1 AND scope_code = ?2 AND definition_code = ?3 AND lifecycle_status = 'ACTIVE'",
        params![scope_kind_to_storage(key.scope_kind), key.scope_code, key.definition_code],
        |row| row.get(0),
    )
    .optional()
    .map_err(storage_error)
}

fn ensure_definition_remains_active(
    tx: &Transaction<'_>,
    expected_key: &ResourceDefinitionKey,
) -> Result<(), ResourceError> {
    if active_definition_version(tx, expected_key)?.as_deref()
        != Some(expected_key.definition_version.as_str())
    {
        return Err(ResourceError::ActiveVersionConflict);
    }
    Ok(())
}

fn ensure_definition_remains_active_and_enabled(
    tx: &Transaction<'_>,
    expected_key: &ResourceDefinitionKey,
) -> Result<(), ResourceError> {
    let active = tx
        .query_row(
            "SELECT definition_version, definition_status FROM resource_definitions
             WHERE scope_kind = ?1 AND scope_code = ?2 AND definition_code = ?3
               AND lifecycle_status = 'ACTIVE'",
            params![
                scope_kind_to_storage(expected_key.scope_kind),
                expected_key.scope_code,
                expected_key.definition_code
            ],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(storage_error)?;
    let Some((active_version, active_status)) = active else {
        return Err(ResourceError::ActiveVersionConflict);
    };
    if active_version != expected_key.definition_version {
        return Err(ResourceError::ActiveVersionConflict);
    }
    if definition_status_from_storage(&active_status)? != ResourceDefinitionStatus::Enabled {
        return Err(ResourceError::DefinitionDisabled);
    }
    Ok(())
}

fn definition_draft_exists(
    tx: &Transaction<'_>,
    key: &ResourceDefinitionKey,
) -> Result<bool, ResourceError> {
    let count: i64 = tx
        .query_row(
            "SELECT count(*) FROM resource_definitions
             WHERE scope_kind = ?1 AND scope_code = ?2 AND definition_code = ?3 AND lifecycle_status = 'DRAFT'",
            params![scope_kind_to_storage(key.scope_kind), key.scope_code, key.definition_code],
            |row| row.get(0),
        )
        .map_err(storage_error)?;
    Ok(count > 0)
}

fn demote_active_definition(
    tx: &Transaction<'_>,
    key: &ResourceDefinitionKey,
    timestamp: &str,
) -> Result<(), ResourceError> {
    tx.execute(
        "UPDATE resource_definitions SET lifecycle_status = 'HISTORICAL', updated_at = ?4
         WHERE scope_kind = ?1 AND scope_code = ?2 AND definition_code = ?3 AND lifecycle_status = 'ACTIVE'",
        params![
            scope_kind_to_storage(key.scope_kind), key.scope_code, key.definition_code,
            timestamp_to_millis(timestamp)
        ],
    )
    .map_err(storage_error)?;
    Ok(())
}

fn resource_lifecycle(
    tx: &Transaction<'_>,
    key: &ResourceKey,
) -> Result<(ResourceLifecycleStatus, i64), ResourceError> {
    resource_lifecycle_optional(tx, key)?.ok_or(ResourceError::NotFound)
}

fn resource_lifecycle_optional(
    tx: &Transaction<'_>,
    key: &ResourceKey,
) -> Result<Option<(ResourceLifecycleStatus, i64)>, ResourceError> {
    tx.query_row(
        "SELECT lifecycle_status, draft_revision FROM resources
         WHERE scope_kind = ?1 AND scope_code = ?2 AND resource_code = ?3 AND resource_version = ?4",
        params![scope_kind_to_storage(key.scope_kind), key.scope_code, key.resource_code, key.resource_version],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
    )
    .optional()
    .map_err(storage_error)?
    .map(|(lifecycle, revision)| Ok((lifecycle_status_from_storage(&lifecycle)?, revision)))
    .transpose()
}

fn active_resource_version(
    tx: &Transaction<'_>,
    key: &ResourceKey,
) -> Result<Option<String>, ResourceError> {
    tx.query_row(
        "SELECT resource_version FROM resources
         WHERE scope_kind = ?1 AND scope_code = ?2 AND resource_code = ?3 AND lifecycle_status = 'ACTIVE'",
        params![scope_kind_to_storage(key.scope_kind), key.scope_code, key.resource_code],
        |row| row.get(0),
    )
    .optional()
    .map_err(storage_error)
}

fn ensure_resource_remains_active(
    tx: &Transaction<'_>,
    expected_key: &ResourceKey,
) -> Result<(), ResourceError> {
    if active_resource_version(tx, expected_key)?.as_deref()
        != Some(expected_key.resource_version.as_str())
    {
        return Err(ResourceError::ActiveVersionConflict);
    }
    Ok(())
}

fn resource_draft_exists(tx: &Transaction<'_>, key: &ResourceKey) -> Result<bool, ResourceError> {
    let count: i64 = tx
        .query_row(
            "SELECT count(*) FROM resources
             WHERE scope_kind = ?1 AND scope_code = ?2 AND resource_code = ?3 AND lifecycle_status = 'DRAFT'",
            params![scope_kind_to_storage(key.scope_kind), key.scope_code, key.resource_code],
            |row| row.get(0),
        )
        .map_err(storage_error)?;
    Ok(count > 0)
}

fn demote_active_resource(
    tx: &Transaction<'_>,
    key: &ResourceKey,
    timestamp: &str,
) -> Result<(), ResourceError> {
    tx.execute(
        "UPDATE resources SET lifecycle_status = 'HISTORICAL', updated_at = ?4
         WHERE scope_kind = ?1 AND scope_code = ?2 AND resource_code = ?3 AND lifecycle_status = 'ACTIVE'",
        params![
            scope_kind_to_storage(key.scope_kind), key.scope_code, key.resource_code,
            timestamp_to_millis(timestamp)
        ],
    )
    .map_err(storage_error)?;
    Ok(())
}

fn definition_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ResourceDefinition> {
    Ok(ResourceDefinition {
        key: ResourceDefinitionKey {
            scope_kind: scope_kind_from_storage(row.get::<_, String>(0)?.as_str())
                .map_err(resource_to_sqlite_error)?,
            scope_code: row.get(1)?,
            definition_code: row.get(2)?,
            definition_version: row.get(3)?,
        },
        name: row.get(4)?,
        description: row.get(5)?,
        definition_status: definition_status_from_storage(row.get::<_, String>(6)?.as_str())
            .map_err(resource_to_sqlite_error)?,
        allow_inheritance: row.get::<_, i64>(7)? != 0,
        attributes_schema: value_from_storage(row.get::<_, String>(8)?.as_str())?,
        binary_schema: value_from_storage(row.get::<_, String>(9)?.as_str())?,
        binary_storage_profile: row.get(10)?,
        draft_revision: row.get(11)?,
        validation_status: validation_status_from_storage(row.get::<_, String>(12)?.as_str())
            .map_err(resource_to_sqlite_error)?,
        lifecycle_status: lifecycle_status_from_storage(row.get::<_, String>(13)?.as_str())
            .map_err(resource_to_sqlite_error)?,
        created_at: timestamp_from_millis(row.get(14)?),
        updated_at: timestamp_from_millis(row.get(15)?),
        published_at: option_timestamp_from_millis(row.get(16)?),
    })
}

fn resource_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Resource> {
    Ok(Resource {
        key: ResourceKey {
            scope_kind: scope_kind_from_storage(row.get::<_, String>(0)?.as_str())
                .map_err(resource_to_sqlite_error)?,
            scope_code: row.get(1)?,
            resource_code: row.get(2)?,
            resource_version: row.get(3)?,
        },
        definition_key: ResourceDefinitionKey {
            scope_kind: scope_kind_from_storage(row.get::<_, String>(4)?.as_str())
                .map_err(resource_to_sqlite_error)?,
            scope_code: row.get(5)?,
            definition_code: row.get(6)?,
            definition_version: row.get(7)?,
        },
        based_on_resource_version: row.get(8)?,
        name: row.get(9)?,
        description: row.get(10)?,
        attributes: value_from_storage(row.get::<_, String>(11)?.as_str())?,
        allow_inheritance: row.get::<_, i64>(12)? != 0,
        draft_revision: row.get(13)?,
        validation_status: validation_status_from_storage(row.get::<_, String>(14)?.as_str())
            .map_err(resource_to_sqlite_error)?,
        lifecycle_status: lifecycle_status_from_storage(row.get::<_, String>(15)?.as_str())
            .map_err(resource_to_sqlite_error)?,
        created_at: timestamp_from_millis(row.get(16)?),
        updated_at: timestamp_from_millis(row.get(17)?),
        published_at: option_timestamp_from_millis(row.get(18)?),
    })
}

fn mapping_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ResourceBinaryMapping> {
    Ok(ResourceBinaryMapping {
        resource_key: ResourceKey {
            scope_kind: scope_kind_from_storage(row.get::<_, String>(0)?.as_str())
                .map_err(resource_to_sqlite_error)?,
            scope_code: row.get(1)?,
            resource_code: row.get(2)?,
            resource_version: row.get(3)?,
        },
        binary_name: row.get(4)?,
        storage_locator: row.get(5)?,
        byte_size: row.get::<_, i64>(6)? as u64,
        created_at: timestamp_from_millis(row.get(7)?),
    })
}

fn scope_kind_to_storage(value: ResourceScopeKind) -> &'static str {
    match value {
        ResourceScopeKind::Platform => "PLATFORM",
        ResourceScopeKind::Workspace => "WORKSPACE",
    }
}

fn scope_kind_from_storage(value: &str) -> Result<ResourceScopeKind, ResourceError> {
    match value {
        "PLATFORM" => Ok(ResourceScopeKind::Platform),
        "WORKSPACE" => Ok(ResourceScopeKind::Workspace),
        _ => Err(ResourceError::StorageFailure),
    }
}

fn lifecycle_status_to_storage(value: ResourceLifecycleStatus) -> &'static str {
    match value {
        ResourceLifecycleStatus::Draft => "DRAFT",
        ResourceLifecycleStatus::Active => "ACTIVE",
        ResourceLifecycleStatus::Historical => "HISTORICAL",
    }
}

fn lifecycle_status_from_storage(value: &str) -> Result<ResourceLifecycleStatus, ResourceError> {
    match value {
        "DRAFT" => Ok(ResourceLifecycleStatus::Draft),
        "ACTIVE" => Ok(ResourceLifecycleStatus::Active),
        "HISTORICAL" => Ok(ResourceLifecycleStatus::Historical),
        _ => Err(ResourceError::StorageFailure),
    }
}

fn definition_status_to_storage(value: ResourceDefinitionStatus) -> &'static str {
    match value {
        ResourceDefinitionStatus::Enabled => "ENABLED",
        ResourceDefinitionStatus::Disabled => "DISABLED",
    }
}

fn definition_status_from_storage(value: &str) -> Result<ResourceDefinitionStatus, ResourceError> {
    match value {
        "ENABLED" => Ok(ResourceDefinitionStatus::Enabled),
        "DISABLED" => Ok(ResourceDefinitionStatus::Disabled),
        _ => Err(ResourceError::StorageFailure),
    }
}

fn validation_status_to_storage(value: ResourceValidationStatus) -> &'static str {
    match value {
        ResourceValidationStatus::Valid => "VALID",
        ResourceValidationStatus::Invalid => "INVALID",
    }
}

fn validation_status_from_storage(value: &str) -> Result<ResourceValidationStatus, ResourceError> {
    match value {
        "VALID" => Ok(ResourceValidationStatus::Valid),
        "INVALID" => Ok(ResourceValidationStatus::Invalid),
        _ => Err(ResourceError::StorageFailure),
    }
}

fn bool_to_int(value: bool) -> i64 {
    i64::from(value)
}

fn value_to_storage(value: &Value) -> Result<String, ResourceError> {
    serde_json::to_string(value).map_err(|_| ResourceError::StorageFailure)
}

fn value_from_storage(value: &str) -> rusqlite::Result<Value> {
    serde_json::from_str(value).map_err(|_| SqliteError::InvalidQuery)
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
