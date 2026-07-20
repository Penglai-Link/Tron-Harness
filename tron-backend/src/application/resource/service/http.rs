use std::{
    collections::BTreeSet,
    io::{Seek, SeekFrom, Write},
};

use crate::{
    application::resource::{
        ResourceActivateVersionCommand, ResourceActivationOutput, ResourceBinaryResult,
        ResourceBinarySummary, ResourceCommitBinary, ResourceCommitCommand,
        ResourceCreateDraftCommand, ResourceDefinitionActivateVersionCommand,
        ResourceDefinitionActivationResult, ResourceDefinitionCreateDraftCommand,
        ResourceDefinitionDeleteCommand, ResourceDefinitionDeleteResult,
        ResourceDefinitionDiscardDraftCommand, ResourceDefinitionDraftAction,
        ResourceDefinitionDraftCommand, ResourceDefinitionDraftResult,
        ResourceDefinitionEffectiveBinaryPolicy, ResourceDefinitionPublishDraftCommand,
        ResourceDefinitionSetEnabledCommand, ResourceDefinitionUpdateDraftCommand,
        ResourceDefinitionVersionGetQuery, ResourceDefinitionVersionPage,
        ResourceDefinitionVersionPageQuery, ResourceDefinitionVersionView, ResourceDeleteCommand,
        ResourceDeleteResult, ResourceDiscardDraftCommand, ResourceDraftAction,
        ResourceDraftCommand, ResourceDraftResult, ResourceHttpBinaryDownload,
        ResourceHttpBinaryDownloadCommand, ResourceHttpBinaryRemoveCommand,
        ResourceHttpBinaryUploadCommand, ResourcePublishDraftCommand, ResourceUpdateDraftCommand,
        ResourceValidationIssue, ResourceVersionGetQuery, ResourceVersionPage,
        ResourceVersionPageQuery, ResourceVersionView,
    },
    config::ResourceLimits,
    domain::{
        resource::{
            PLATFORM_SCOPE_CODE, Resource, ResourceActivationResult, ResourceBinaryMapping,
            ResourceDefinition, ResourceDefinitionKey, ResourceDefinitionStatus, ResourceError,
            ResourceKey, ResourceLifecycleStatus, ResourceScopeKind, ResourceSourceKind,
            ResourceValidationStatus, validate_binary_name, validate_definition_code,
            validate_resource_code, validate_version_code,
        },
        variable::VariableError,
    },
    shared::time::current_timestamp,
};

use super::{
    ResourceService,
    definitions::{definition_validation_issues, next_definition_version},
    resources::next_resource_version,
    seeds::is_builtin_definition_code,
    validation::{declared_variable_references, parse_binary_policy, validate_binary_upload},
};

impl ResourceService {
    pub fn page_definition_versions(
        &self,
        actor_user_code: &str,
        query: ResourceDefinitionVersionPageQuery,
    ) -> Result<ResourceDefinitionVersionPage, ResourceError> {
        let scope_code = self.management_scope(
            actor_user_code,
            query.scope_kind,
            query.workspace_code.as_deref(),
            false,
        )?;
        let definitions = self.repository(|repository| repository.list_definitions())?;
        let mut views = match query.scope_kind {
            ResourceScopeKind::Platform => definitions
                .iter()
                .filter(|definition| {
                    definition.key.scope_kind == ResourceScopeKind::Platform
                        && definition.key.scope_code == scope_code
                })
                .map(|definition| {
                    definition_version_view(
                        definition,
                        ResourceSourceKind::Platform,
                        false,
                        &self.config.limits,
                    )
                })
                .collect(),
            ResourceScopeKind::Workspace => self.workspace_definition_version_views(
                &scope_code,
                &definitions,
                query.lifecycle_status,
            )?,
        };
        views.retain(|view| definition_page_matches(view, &query));
        sort_definition_views(&mut views);
        let total = views.len();
        let (current, size, start, end) = page_bounds(query.current, query.size, total);
        Ok(ResourceDefinitionVersionPage {
            current,
            size,
            total,
            records: views[start..end].to_vec(),
        })
    }

    pub fn get_definition_version(
        &self,
        actor_user_code: &str,
        query: ResourceDefinitionVersionGetQuery,
    ) -> Result<ResourceDefinitionVersionView, ResourceError> {
        validate_definition_code(&query.definition_code)?;
        validate_version_code(&query.definition_version)?;
        let scope_code = self.management_scope(
            actor_user_code,
            query.scope_kind,
            query.workspace_code.as_deref(),
            false,
        )?;
        let (definition, source_kind) = self.visible_definition_version(
            query.scope_kind,
            &scope_code,
            &query.definition_code,
            &query.definition_version,
        )?;
        Ok(definition_version_view(
            &definition,
            source_kind,
            true,
            &self.config.limits,
        ))
    }

    pub fn create_definition_draft_for_http(
        &self,
        actor_user_code: &str,
        command: ResourceDefinitionCreateDraftCommand,
    ) -> Result<ResourceDefinitionDraftResult, ResourceError> {
        validate_definition_code(&command.definition_code)?;
        require_non_blank(&command.name)?;
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        let source = command
            .source_definition_version
            .as_deref()
            .map(|version| {
                validate_version_code(version)?;
                let (definition, _) = self.visible_definition_version(
                    command.scope_kind,
                    &scope_code,
                    &command.definition_code,
                    version,
                )?;
                if definition.lifecycle_status == ResourceLifecycleStatus::Draft {
                    return Err(ResourceError::LifecycleDenied);
                }
                Ok(definition)
            })
            .transpose()?;
        let draft_command = ResourceDefinitionDraftCommand {
            scope_kind: command.scope_kind,
            workspace_code: command.workspace_code,
            action: ResourceDefinitionDraftAction::Create,
            definition_code: command.definition_code,
            definition_version: None,
            expected_draft_revision: None,
            name: Some(command.name),
            description: command.description,
            definition_status: command.definition_status,
            allow_inheritance: command.allow_inheritance,
            attributes_schema: command.attributes_schema,
            binary_schema: command.binary_schema,
            binary_storage_profile: command.binary_storage_profile,
        };
        let definition = self.create_definition_draft(&scope_code, &draft_command, source)?;
        let validation_errors = definition_validation_issues(&definition, self)?;
        self.repository(|repository| repository.insert_definition(&definition))?;
        Ok(definition_draft_result(
            &definition,
            validation_errors,
            false,
        ))
    }

    pub fn update_definition_draft_for_http(
        &self,
        actor_user_code: &str,
        command: ResourceDefinitionUpdateDraftCommand,
    ) -> Result<ResourceDefinitionDraftResult, ResourceError> {
        validate_definition_code(&command.definition_code)?;
        validate_version_code(&command.definition_version)?;
        if !definition_update_has_content(&command) {
            return Err(ResourceError::InvalidInput);
        }
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        let key = ResourceDefinitionKey {
            scope_kind: command.scope_kind,
            scope_code,
            definition_code: command.definition_code.clone(),
            definition_version: command.definition_version.clone(),
        };
        let existing = self
            .definition_by_key(&key)?
            .ok_or(ResourceError::NotFound)?;
        if existing.lifecycle_status != ResourceLifecycleStatus::Draft {
            return Err(ResourceError::LifecycleDenied);
        }
        let draft_command = ResourceDefinitionDraftCommand {
            scope_kind: command.scope_kind,
            workspace_code: command.workspace_code,
            action: ResourceDefinitionDraftAction::Update,
            definition_code: command.definition_code,
            definition_version: Some(command.definition_version),
            expected_draft_revision: Some(command.expected_draft_revision),
            name: command.name,
            description: command.description,
            definition_status: command.definition_status,
            allow_inheritance: command.allow_inheritance,
            attributes_schema: command.attributes_schema,
            binary_schema: command.binary_schema,
            binary_storage_profile: command.binary_storage_profile,
        };
        let updated = self.apply_definition_changes(existing, &draft_command)?;
        self.repository(|repository| {
            repository.update_definition_draft(&updated, command.expected_draft_revision)
        })?;
        let validation_errors = definition_validation_issues(&updated, self)?;
        Ok(definition_draft_result(&updated, validation_errors, false))
    }

    pub fn discard_definition_draft_for_http(
        &self,
        actor_user_code: &str,
        command: ResourceDefinitionDiscardDraftCommand,
    ) -> Result<ResourceDefinitionDraftResult, ResourceError> {
        validate_definition_code(&command.definition_code)?;
        validate_version_code(&command.definition_version)?;
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        let key = ResourceDefinitionKey {
            scope_kind: command.scope_kind,
            scope_code: scope_code.clone(),
            definition_code: command.definition_code.clone(),
            definition_version: command.definition_version.clone(),
        };
        let discarded = self.repository(|repository| {
            repository.discard_definition_draft(&key, command.expected_draft_revision)
        })?;
        Ok(ResourceDefinitionDraftResult {
            scope_kind: command.scope_kind,
            scope_code,
            definition_code: command.definition_code,
            definition_version: command.definition_version,
            draft_revision: command.expected_draft_revision,
            lifecycle_status: ResourceLifecycleStatus::Draft,
            validation_status: ResourceValidationStatus::Valid,
            validation_errors: Vec::new(),
            discarded: Some(discarded),
        })
    }

    pub fn publish_definition_draft_for_http(
        &self,
        actor_user_code: &str,
        command: ResourceDefinitionPublishDraftCommand,
    ) -> Result<ResourceDefinitionActivationResult, ResourceError> {
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        let key = definition_key(
            command.scope_kind,
            &scope_code,
            &command.definition_code,
            &command.definition_version,
        )?;
        let definition = self
            .definition_by_key(&key)?
            .ok_or(ResourceError::NotFound)?;
        if definition.lifecycle_status != ResourceLifecycleStatus::Draft {
            return Err(ResourceError::LifecycleDenied);
        }
        if !definition_validation_issues(&definition, self)?.is_empty() {
            return Err(ResourceError::ValidationFailed);
        }
        let activation = self.repository(|repository| {
            repository.activate_definition(
                &key,
                Some(command.expected_draft_revision),
                None,
                &current_timestamp(),
            )
        })?;
        self.publish_audit(
            "resource_definition.published",
            actor_user_code,
            &command.definition_code,
            &scope_code,
            &command.definition_version,
        );
        Ok(definition_activation_result(
            command.definition_code,
            command.definition_version,
            activation.activation_result,
            activation.previous_active_version,
        ))
    }

    pub fn activate_definition_version_for_http(
        &self,
        actor_user_code: &str,
        command: ResourceDefinitionActivateVersionCommand,
    ) -> Result<ResourceDefinitionActivationResult, ResourceError> {
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        let key = definition_key(
            command.scope_kind,
            &scope_code,
            &command.definition_code,
            &command.definition_version,
        )?;
        let definition = self
            .definition_by_key(&key)?
            .ok_or(ResourceError::NotFound)?;
        if definition.lifecycle_status == ResourceLifecycleStatus::Draft {
            return Err(ResourceError::LifecycleDenied);
        }
        if definition.lifecycle_status == ResourceLifecycleStatus::Historical
            && !definition_validation_issues(&definition, self)?.is_empty()
        {
            return Err(ResourceError::ValidationFailed);
        }
        let activation = self.repository(|repository| {
            repository.activate_definition(
                &key,
                None,
                command.expected_active_definition_version.as_deref(),
                &current_timestamp(),
            )
        })?;
        if activation.activation_result != ResourceActivationResult::AlreadyActive {
            self.publish_audit(
                "resource_definition.activated",
                actor_user_code,
                &command.definition_code,
                &scope_code,
                &command.definition_version,
            );
        }
        Ok(definition_activation_result(
            command.definition_code,
            command.definition_version,
            activation.activation_result,
            activation.previous_active_version,
        ))
    }

    pub fn set_definition_enabled_for_http(
        &self,
        actor_user_code: &str,
        command: ResourceDefinitionSetEnabledCommand,
    ) -> Result<ResourceDefinitionActivationResult, ResourceError> {
        validate_definition_code(&command.definition_code)?;
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        let source = self.effective_active_definition_for_http(
            command.scope_kind,
            &scope_code,
            &command.definition_code,
        )?;
        if command.expected_active_definition_version.as_str()
            != source.key.definition_version.as_str()
        {
            return Err(ResourceError::ActiveVersionConflict);
        }
        let definitions = self.repository(|repository| repository.list_definitions())?;
        if definitions.iter().any(|definition| {
            definition.key.scope_kind == command.scope_kind
                && definition.key.scope_code == scope_code
                && definition.key.definition_code == command.definition_code
                && definition.lifecycle_status == ResourceLifecycleStatus::Draft
        }) {
            return Err(ResourceError::Conflict);
        }
        if source.definition_status
            == if command.enabled {
                ResourceDefinitionStatus::Enabled
            } else {
                ResourceDefinitionStatus::Disabled
            }
        {
            return Ok(definition_activation_result(
                source.key.definition_code,
                source.key.definition_version,
                ResourceActivationResult::AlreadyActive,
                None,
            ));
        }
        let target_active = definitions
            .iter()
            .find(|definition| {
                definition.key.scope_kind == command.scope_kind
                    && definition.key.scope_code == scope_code
                    && definition.key.definition_code == command.definition_code
                    && definition.lifecycle_status == ResourceLifecycleStatus::Active
            })
            .map(|definition| definition.key.definition_version.clone());
        let expected_effective_source = source.key.clone();
        let now = current_timestamp();
        let definition = ResourceDefinition {
            key: ResourceDefinitionKey {
                scope_kind: command.scope_kind,
                scope_code: scope_code.clone(),
                definition_code: command.definition_code.clone(),
                definition_version: next_definition_version(
                    &definitions,
                    command.scope_kind,
                    &scope_code,
                    &command.definition_code,
                )?,
            },
            name: source.name,
            description: source.description,
            definition_status: if command.enabled {
                ResourceDefinitionStatus::Enabled
            } else {
                ResourceDefinitionStatus::Disabled
            },
            allow_inheritance: source.allow_inheritance,
            attributes_schema: source.attributes_schema,
            binary_schema: source.binary_schema,
            binary_storage_profile: source.binary_storage_profile,
            draft_revision: 0,
            validation_status: ResourceValidationStatus::Valid,
            lifecycle_status: ResourceLifecycleStatus::Active,
            created_at: now.clone(),
            updated_at: now.clone(),
            published_at: Some(now),
        };
        if !definition_validation_issues(&definition, self)?.is_empty() {
            return Err(ResourceError::ValidationFailed);
        }
        let activation = self.repository(|repository| {
            repository.commit_active_definition(
                &definition,
                target_active.as_deref(),
                &expected_effective_source,
            )
        })?;
        self.publish_audit(
            "resource_definition.enabled_changed",
            actor_user_code,
            &definition.key.definition_code,
            &scope_code,
            &definition.key.definition_version,
        );
        Ok(definition_activation_result(
            definition.key.definition_code,
            definition.key.definition_version,
            activation.activation_result,
            activation.previous_active_version,
        ))
    }

    pub fn delete_definition_for_http(
        &self,
        actor_user_code: &str,
        command: ResourceDefinitionDeleteCommand,
    ) -> Result<ResourceDefinitionDeleteResult, ResourceError> {
        validate_definition_code(&command.definition_code)?;
        validate_version_code(&command.expected_active_definition_version)?;
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        if command.scope_kind == ResourceScopeKind::Platform
            && scope_code == PLATFORM_SCOPE_CODE
            && is_builtin_definition_code(&command.definition_code)
        {
            return Err(ResourceError::BuiltinDefinition);
        }
        let owned_active = self.repository(|repository| {
            Ok(repository
                .list_definitions()?
                .into_iter()
                .find(|definition| {
                    definition.key.scope_kind == command.scope_kind
                        && definition.key.scope_code == scope_code
                        && definition.key.definition_code == command.definition_code
                        && definition.key.definition_version
                            == command.expected_active_definition_version
                        && definition.lifecycle_status == ResourceLifecycleStatus::Active
                }))
        })?;
        if owned_active.is_none() {
            return Err(ResourceError::LifecycleDenied);
        }
        let deleted = self.repository(|repository| {
            repository.delete_definition_group(
                command.scope_kind,
                &scope_code,
                &command.definition_code,
                &command.expected_active_definition_version,
            )
        })?;
        if deleted {
            self.publish_audit(
                "resource_definition.deleted",
                actor_user_code,
                &command.definition_code,
                &scope_code,
                &command.expected_active_definition_version,
            );
        }
        Ok(ResourceDefinitionDeleteResult {
            definition_code: command.definition_code,
            deleted,
        })
    }

    fn effective_active_definition_for_http(
        &self,
        scope_kind: ResourceScopeKind,
        scope_code: &str,
        definition_code: &str,
    ) -> Result<ResourceDefinition, ResourceError> {
        match scope_kind {
            ResourceScopeKind::Platform => self
                .repository(|repository| repository.list_definitions())?
                .into_iter()
                .find(|definition| {
                    definition.key.scope_kind == ResourceScopeKind::Platform
                        && definition.key.scope_code == scope_code
                        && definition.key.definition_code == definition_code
                        && definition.lifecycle_status == ResourceLifecycleStatus::Active
                })
                .ok_or(ResourceError::NotFound),
            ResourceScopeKind::Workspace => self
                .effective_definitions(scope_code)?
                .remove(definition_code)
                .map(|effective| effective.definition)
                .ok_or(ResourceError::NotFound),
        }
    }

    fn workspace_definition_version_views(
        &self,
        workspace_code: &str,
        definitions: &[ResourceDefinition],
        lifecycle: Option<ResourceLifecycleStatus>,
    ) -> Result<Vec<ResourceDefinitionVersionView>, ResourceError> {
        let mut views = Vec::new();
        if lifecycle.is_none() || lifecycle == Some(ResourceLifecycleStatus::Active) {
            views.extend(
                self.effective_definitions(workspace_code)?
                    .into_values()
                    .map(|effective| {
                        definition_version_view(
                            &effective.definition,
                            effective.source_kind,
                            false,
                            &self.config.limits,
                        )
                    }),
            );
        }
        if lifecycle.is_none()
            || lifecycle == Some(ResourceLifecycleStatus::Draft)
            || lifecycle == Some(ResourceLifecycleStatus::Historical)
        {
            views.extend(
                definitions
                    .iter()
                    .filter(|definition| {
                        definition.key.scope_kind == ResourceScopeKind::Workspace
                            && definition.key.scope_code == workspace_code
                            && definition.lifecycle_status != ResourceLifecycleStatus::Active
                            && lifecycle
                                .is_none_or(|expected| definition.lifecycle_status == expected)
                    })
                    .map(|definition| {
                        definition_version_view(
                            definition,
                            ResourceSourceKind::Local,
                            false,
                            &self.config.limits,
                        )
                    }),
            );
        }
        Ok(views)
    }

    fn visible_definition_version(
        &self,
        scope_kind: ResourceScopeKind,
        scope_code: &str,
        definition_code: &str,
        definition_version: &str,
    ) -> Result<(ResourceDefinition, ResourceSourceKind), ResourceError> {
        let definitions = self.repository(|repository| repository.list_definitions())?;
        if scope_kind == ResourceScopeKind::Platform {
            return definitions
                .into_iter()
                .find(|definition| {
                    definition.key.scope_kind == ResourceScopeKind::Platform
                        && definition.key.scope_code == scope_code
                        && definition.key.definition_code == definition_code
                        && definition.key.definition_version == definition_version
                })
                .map(|definition| (definition, ResourceSourceKind::Platform))
                .ok_or(ResourceError::NotFound);
        }
        if let Some(definition) = definitions.iter().find(|definition| {
            definition.key.scope_kind == ResourceScopeKind::Workspace
                && definition.key.scope_code == scope_code
                && definition.key.definition_code == definition_code
                && definition.key.definition_version == definition_version
                && definition.lifecycle_status != ResourceLifecycleStatus::Active
        }) {
            return Ok((definition.clone(), ResourceSourceKind::Local));
        }
        self.effective_definitions(scope_code)?
            .remove(definition_code)
            .filter(|effective| effective.definition.key.definition_version == definition_version)
            .map(|effective| (effective.definition, effective.source_kind))
            .ok_or(ResourceError::NotFound)
    }
}

fn definition_version_view(
    definition: &ResourceDefinition,
    source_kind: ResourceSourceKind,
    include_detail: bool,
    limits: &ResourceLimits,
) -> ResourceDefinitionVersionView {
    ResourceDefinitionVersionView {
        scope_kind: definition.key.scope_kind,
        scope_code: definition.key.scope_code.clone(),
        source_kind,
        built_in: definition.key.scope_kind == ResourceScopeKind::Platform
            && definition.key.scope_code == PLATFORM_SCOPE_CODE
            && is_builtin_definition_code(&definition.key.definition_code),
        definition_code: definition.key.definition_code.clone(),
        definition_version: definition.key.definition_version.clone(),
        name: definition.name.clone(),
        description: definition.description.clone(),
        definition_status: definition.definition_status,
        allow_inheritance: definition.allow_inheritance,
        lifecycle_status: definition.lifecycle_status,
        validation_status: definition.validation_status,
        draft_revision: (definition.lifecycle_status == ResourceLifecycleStatus::Draft)
            .then_some(definition.draft_revision),
        attributes_schema: include_detail.then(|| definition.attributes_schema.clone()),
        binary_schema: include_detail.then(|| definition.binary_schema.clone()),
        effective_binary_policy: include_detail
            .then(|| effective_binary_policy(definition, limits))
            .flatten(),
        binary_storage_profile: include_detail.then(|| definition.binary_storage_profile.clone()),
        created_at: definition.created_at.clone(),
        updated_at: definition.updated_at.clone(),
        published_at: definition.published_at.clone(),
    }
}

/// The raw binary schema may intentionally omit limits and inherit the service
/// defaults.  Detail consumers need the resolved values for client-side
/// guidance, while the persisted schema must remain unchanged for editing.
fn effective_binary_policy(
    definition: &ResourceDefinition,
    limits: &ResourceLimits,
) -> Option<ResourceDefinitionEffectiveBinaryPolicy> {
    let policy = parse_binary_policy(&definition.binary_schema, limits).ok()?;
    Some(ResourceDefinitionEffectiveBinaryPolicy {
        min_count: policy.min_count,
        max_count: policy.max_count,
        max_size_bytes: policy.max_size_bytes,
        max_name_length: policy.max_name_length,
        name_pattern: policy.name_pattern,
    })
}

fn definition_page_matches(
    view: &ResourceDefinitionVersionView,
    query: &ResourceDefinitionVersionPageQuery,
) -> bool {
    query
        .definition_code
        .as_deref()
        .is_none_or(|value| view.definition_code == value)
        && query
            .definition_status
            .is_none_or(|value| view.definition_status == value)
        && query
            .lifecycle_status
            .is_none_or(|value| view.lifecycle_status == value)
        && query
            .validation_status
            .is_none_or(|value| view.validation_status == value)
        && query
            .source_kind
            .is_none_or(|value| view.source_kind == value)
        && query.search.as_deref().is_none_or(|search| {
            let search = search.trim().to_ascii_lowercase();
            search.is_empty()
                || view.definition_code.to_ascii_lowercase().contains(&search)
                || view.name.to_ascii_lowercase().contains(&search)
                || view.description.to_ascii_lowercase().contains(&search)
        })
}

fn definition_update_has_content(command: &ResourceDefinitionUpdateDraftCommand) -> bool {
    command.name.is_some()
        || command.description.is_some()
        || command.definition_status.is_some()
        || command.allow_inheritance.is_some()
        || command.attributes_schema.is_some()
        || command.binary_schema.is_some()
        || command.binary_storage_profile.is_some()
}

fn definition_draft_result(
    definition: &ResourceDefinition,
    validation_errors: Vec<ResourceValidationIssue>,
    discarded: bool,
) -> ResourceDefinitionDraftResult {
    ResourceDefinitionDraftResult {
        scope_kind: definition.key.scope_kind,
        scope_code: definition.key.scope_code.clone(),
        definition_code: definition.key.definition_code.clone(),
        definition_version: definition.key.definition_version.clone(),
        draft_revision: definition.draft_revision,
        lifecycle_status: definition.lifecycle_status,
        validation_status: definition.validation_status,
        validation_errors,
        discarded: discarded.then_some(true),
    }
}

fn definition_activation_result(
    definition_code: String,
    definition_version: String,
    activation_result: ResourceActivationResult,
    previous_active_definition_version: Option<String>,
) -> ResourceDefinitionActivationResult {
    ResourceDefinitionActivationResult {
        definition_code,
        definition_version,
        lifecycle_status: ResourceLifecycleStatus::Active,
        activation_result,
        previous_active_definition_version,
    }
}

fn definition_key(
    scope_kind: ResourceScopeKind,
    scope_code: &str,
    definition_code: &str,
    definition_version: &str,
) -> Result<ResourceDefinitionKey, ResourceError> {
    validate_definition_code(definition_code)?;
    validate_version_code(definition_version)?;
    Ok(ResourceDefinitionKey {
        scope_kind,
        scope_code: scope_code.to_string(),
        definition_code: definition_code.to_string(),
        definition_version: definition_version.to_string(),
    })
}

fn sort_definition_views(views: &mut [ResourceDefinitionVersionView]) {
    views.sort_by(|left, right| {
        left.definition_code
            .cmp(&right.definition_code)
            .then_with(|| right.definition_version.cmp(&left.definition_version))
    });
}

fn page_bounds(current: usize, size: usize, total: usize) -> (usize, usize, usize, usize) {
    let current = current.max(1);
    let size = size.clamp(1, 50);
    let start = current.saturating_sub(1).saturating_mul(size).min(total);
    let end = start.saturating_add(size).min(total);
    (current, size, start, end)
}

fn require_non_blank(value: &str) -> Result<(), ResourceError> {
    if value.trim().is_empty() {
        Err(ResourceError::InvalidInput)
    } else {
        Ok(())
    }
}

impl ResourceService {
    pub fn page_resource_versions(
        &self,
        actor_user_code: &str,
        query: ResourceVersionPageQuery,
    ) -> Result<ResourceVersionPage, ResourceError> {
        let scope_code = self.management_scope(
            actor_user_code,
            query.scope_kind,
            query.workspace_code.as_deref(),
            false,
        )?;
        let resources = self.repository(|repository| repository.list_resources())?;
        let definitions = self.repository(|repository| repository.list_definitions())?;
        let mappings = self.repository(|repository| repository.list_binary_mappings())?;
        let mut views = match query.scope_kind {
            ResourceScopeKind::Platform => resources
                .iter()
                .filter(|resource| {
                    resource.key.scope_kind == ResourceScopeKind::Platform
                        && resource.key.scope_code == scope_code
                })
                .map(|resource| {
                    resource_version_view(
                        resource,
                        ResourceSourceKind::Platform,
                        &definitions,
                        &mappings,
                        false,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
            ResourceScopeKind::Workspace => self.workspace_resource_version_views(
                &scope_code,
                &resources,
                &definitions,
                &mappings,
                query.lifecycle_status,
            )?,
        };
        views.retain(|view| resource_page_matches(view, &query));
        sort_resource_views(&mut views);
        let total = views.len();
        let (current, size, start, end) = page_bounds(query.current, query.size, total);
        Ok(ResourceVersionPage {
            current,
            size,
            total,
            records: views[start..end].to_vec(),
        })
    }

    pub fn get_resource_version(
        &self,
        actor_user_code: &str,
        query: ResourceVersionGetQuery,
    ) -> Result<ResourceVersionView, ResourceError> {
        validate_resource_code(&query.resource_code)?;
        validate_version_code(&query.resource_version)?;
        let scope_code = self.management_scope(
            actor_user_code,
            query.scope_kind,
            query.workspace_code.as_deref(),
            false,
        )?;
        let (resource, source_kind) = self.visible_resource_version(
            query.scope_kind,
            &scope_code,
            &query.resource_code,
            &query.resource_version,
        )?;
        let definitions = self.repository(|repository| repository.list_definitions())?;
        let mappings = self.repository(|repository| repository.list_binary_mappings())?;
        resource_version_view(&resource, source_kind, &definitions, &mappings, true)
    }

    pub fn create_resource_draft_for_http(
        &self,
        actor_user_code: &str,
        command: ResourceCreateDraftCommand,
    ) -> Result<ResourceDraftResult, ResourceError> {
        validate_resource_code(&command.resource_code)?;
        require_non_blank(&command.name)?;
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        let source = command
            .source_resource_version
            .as_deref()
            .map(|version| {
                validate_version_code(version)?;
                let (resource, _) = self.visible_resource_version(
                    command.scope_kind,
                    &scope_code,
                    &command.resource_code,
                    version,
                )?;
                if resource.lifecycle_status == ResourceLifecycleStatus::Draft {
                    return Err(ResourceError::LifecycleDenied);
                }
                Ok(resource)
            })
            .transpose()?;
        let definition = match source.as_ref() {
            Some(source) => {
                if command
                    .definition_code
                    .as_deref()
                    .is_some_and(|definition_code| {
                        definition_code != source.definition_key.definition_code
                    })
                {
                    return Err(ResourceError::InvalidInput);
                }
                self.definition_by_resource(source)?
            }
            None => {
                let definition_code = command
                    .definition_code
                    .as_deref()
                    .ok_or(ResourceError::InvalidInput)?;
                self.ensure_resource_create_is_available_for_scope(
                    command.scope_kind,
                    &scope_code,
                    &command.resource_code,
                )?;
                self.definition_for_resource_scope(
                    command.scope_kind,
                    &scope_code,
                    definition_code,
                )?
            }
        };
        let source_mappings = source
            .as_ref()
            .map(|resource| self.mappings_for_resource(&resource.key))
            .transpose()?
            .unwrap_or_default();
        let draft_command = ResourceDraftCommand {
            action: ResourceDraftAction::Create,
            workspace_code: command.workspace_code,
            resource_code: command.resource_code,
            resource_version: None,
            expected_draft_revision: None,
            definition_code: Some(definition.key.definition_code.clone()),
            name: Some(command.name),
            description: command.description,
            attributes: command.attributes,
            allow_inheritance: command.allow_inheritance,
        };
        let resource = self.create_resource_draft(
            command.scope_kind,
            &scope_code,
            &draft_command,
            &definition,
            source,
            source_mappings.clone(),
        )?;
        let mappings = copied_mappings_for_key(&source_mappings, &resource.key);
        let validation_errors =
            self.resource_validation_issues(&resource, &definition, &mappings)?;
        self.repository(|repository| repository.insert_resource(&resource, &mappings))?;
        Ok(resource_draft_result(
            &resource,
            validation_errors,
            "CREATED",
            false,
        ))
    }

    pub fn update_resource_draft_for_http(
        &self,
        actor_user_code: &str,
        command: ResourceUpdateDraftCommand,
    ) -> Result<ResourceDraftResult, ResourceError> {
        validate_resource_code(&command.resource_code)?;
        validate_version_code(&command.resource_version)?;
        if !resource_update_has_content(&command) {
            return Err(ResourceError::InvalidInput);
        }
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        let key = resource_key(
            command.scope_kind,
            &scope_code,
            &command.resource_code,
            &command.resource_version,
        )?;
        let existing = self.resource_by_key(&key)?.ok_or(ResourceError::NotFound)?;
        if existing.lifecycle_status != ResourceLifecycleStatus::Draft {
            return Err(ResourceError::LifecycleDenied);
        }
        let definition = self.definition_by_resource(&existing)?;
        let mappings = self.mappings_for_resource(&existing.key)?;
        let draft_command = ResourceDraftCommand {
            action: ResourceDraftAction::Update,
            workspace_code: command.workspace_code,
            resource_code: command.resource_code,
            resource_version: Some(command.resource_version),
            expected_draft_revision: Some(command.expected_draft_revision),
            definition_code: None,
            name: command.name,
            description: command.description,
            attributes: command.attributes,
            allow_inheritance: command.allow_inheritance,
        };
        let updated =
            self.apply_resource_changes(existing, &draft_command, &definition, &mappings)?;
        self.repository(|repository| {
            repository.update_resource_draft(&updated, &mappings, command.expected_draft_revision)
        })?;
        let validation_errors =
            self.resource_validation_issues(&updated, &definition, &mappings)?;
        Ok(resource_draft_result(
            &updated,
            validation_errors,
            "DRAFT_UPDATED",
            false,
        ))
    }

    pub fn discard_resource_draft_for_http(
        &self,
        actor_user_code: &str,
        command: ResourceDiscardDraftCommand,
    ) -> Result<ResourceDraftResult, ResourceError> {
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        let key = resource_key(
            command.scope_kind,
            &scope_code,
            &command.resource_code,
            &command.resource_version,
        )?;
        let resource = self.resource_by_key(&key)?.ok_or(ResourceError::NotFound)?;
        let (discarded, locators) = self.repository(|repository| {
            repository.discard_resource_draft(&key, command.expected_draft_revision)
        })?;
        self.delete_unreferenced_locators(locators);
        Ok(ResourceDraftResult {
            draft_result: "DISCARDED".to_string(),
            resource_code: command.resource_code,
            resource_version: command.resource_version,
            definition_scope_kind: resource.definition_key.scope_kind,
            definition_scope_code: resource.definition_key.scope_code,
            definition_code: resource.definition_key.definition_code,
            definition_version: resource.definition_key.definition_version,
            draft_revision: command.expected_draft_revision,
            validation_status: ResourceValidationStatus::Valid,
            validation_errors: Vec::new(),
            discarded: Some(discarded),
            resource_removed: Some(false),
        })
    }

    pub fn publish_resource_draft_for_http(
        &self,
        actor_user_code: &str,
        command: ResourcePublishDraftCommand,
    ) -> Result<ResourceActivationOutput, ResourceError> {
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        let key = resource_key(
            command.scope_kind,
            &scope_code,
            &command.resource_code,
            &command.resource_version,
        )?;
        let resource = self.resource_by_key(&key)?.ok_or(ResourceError::NotFound)?;
        if resource.lifecycle_status != ResourceLifecycleStatus::Draft {
            return Err(ResourceError::LifecycleDenied);
        }
        let definition = self.definition_by_resource(&resource)?;
        let mappings = self.mappings_for_resource(&key)?;
        self.ensure_resource_publishable(&resource, &definition, &mappings)?;
        let activation = self.repository(|repository| {
            repository.activate_resource(
                &key,
                Some(command.expected_draft_revision),
                None,
                &current_timestamp(),
            )
        })?;
        self.publish_resource_audit(
            actor_user_code,
            &resource,
            &scope_code,
            activation.activation_result,
            activation.previous_active_version.as_deref(),
        );
        Ok(resource_activation_output(
            &resource,
            activation.activation_result,
            activation.previous_active_version,
        ))
    }

    pub fn activate_resource_version_for_http(
        &self,
        actor_user_code: &str,
        command: ResourceActivateVersionCommand,
    ) -> Result<ResourceActivationOutput, ResourceError> {
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        let key = resource_key(
            command.scope_kind,
            &scope_code,
            &command.resource_code,
            &command.resource_version,
        )?;
        let resource = self.resource_by_key(&key)?.ok_or(ResourceError::NotFound)?;
        if resource.lifecycle_status == ResourceLifecycleStatus::Draft {
            return Err(ResourceError::LifecycleDenied);
        }
        if resource.lifecycle_status == ResourceLifecycleStatus::Active {
            return Ok(resource_activation_output(
                &resource,
                ResourceActivationResult::AlreadyActive,
                None,
            ));
        }
        let definition = self.definition_by_resource(&resource)?;
        let mappings = self.mappings_for_resource(&key)?;
        self.ensure_resource_publishable(&resource, &definition, &mappings)?;
        let activation = self.repository(|repository| {
            repository.activate_resource(
                &key,
                None,
                command.expected_active_resource_version.as_deref(),
                &current_timestamp(),
            )
        })?;
        self.publish_resource_audit(
            actor_user_code,
            &resource,
            &scope_code,
            activation.activation_result,
            activation.previous_active_version.as_deref(),
        );
        Ok(resource_activation_output(
            &resource,
            activation.activation_result,
            activation.previous_active_version,
        ))
    }

    pub fn create_resource_for_http(
        &self,
        actor_user_code: &str,
        command: ResourceCommitCommand,
        binaries: Vec<ResourceCommitBinary>,
    ) -> Result<ResourceActivationOutput, ResourceError> {
        self.commit_resource_for_http(actor_user_code, command, binaries, true)
    }

    pub fn update_resource_for_http(
        &self,
        actor_user_code: &str,
        command: ResourceCommitCommand,
        binaries: Vec<ResourceCommitBinary>,
    ) -> Result<ResourceActivationOutput, ResourceError> {
        self.commit_resource_for_http(actor_user_code, command, binaries, false)
    }

    pub fn preflight_resource_commit(
        &self,
        actor_user_code: &str,
        command: &ResourceCommitCommand,
        create: bool,
    ) -> Result<(), ResourceError> {
        validate_resource_code(&command.resource_code)?;
        require_non_blank(&command.name)?;
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        if create {
            let definition_code = command
                .definition_code
                .as_deref()
                .ok_or(ResourceError::InvalidInput)?;
            validate_definition_code(definition_code)?;
            if command.expected_active_resource_version.is_some() {
                return Err(ResourceError::InvalidInput);
            }
            self.ensure_resource_create_is_available_for_scope(
                command.scope_kind,
                &scope_code,
                &command.resource_code,
            )?;
            let _ = self.definition_for_resource_scope(
                command.scope_kind,
                &scope_code,
                definition_code,
            )?;
        } else {
            if command.definition_code.is_some() {
                return Err(ResourceError::InvalidInput);
            }
            let expected = command
                .expected_active_resource_version
                .as_deref()
                .ok_or(ResourceError::ActiveVersionConflict)?;
            validate_version_code(expected)?;
            let source = self.effective_active_resource_for_http(
                command.scope_kind,
                &scope_code,
                &command.resource_code,
            )?;
            if source.key.resource_version != expected {
                return Err(ResourceError::ActiveVersionConflict);
            }
        }
        Ok(())
    }

    pub fn delete_resource_for_http(
        &self,
        actor_user_code: &str,
        command: ResourceDeleteCommand,
    ) -> Result<ResourceDeleteResult, ResourceError> {
        validate_resource_code(&command.resource_code)?;
        if let Some(version) = command.expected_active_resource_version.as_deref() {
            validate_version_code(version)?;
        }
        if let Some(version) = command.expected_draft_resource_version.as_deref() {
            validate_version_code(version)?;
        }
        if command.expected_draft_resource_version.is_some()
            != command.expected_draft_revision.is_some()
        {
            return Err(ResourceError::InvalidInput);
        }
        if command.expected_active_resource_version.is_none()
            && command.expected_draft_resource_version.is_none()
        {
            return Err(ResourceError::InvalidInput);
        }
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        let locally_owned = self.repository(|repository| {
            Ok(repository.list_resources()?.into_iter().any(|resource| {
                resource.key.scope_kind == command.scope_kind
                    && resource.key.scope_code == scope_code
                    && resource.key.resource_code == command.resource_code
            }))
        })?;
        if !locally_owned {
            return Err(ResourceError::LifecycleDenied);
        }
        let (deleted, locators) = self.repository(|repository| {
            repository.delete_resource_group(
                command.scope_kind,
                &scope_code,
                &command.resource_code,
                command.expected_active_resource_version.as_deref(),
                command.expected_draft_resource_version.as_deref(),
                command.expected_draft_revision,
            )
        })?;
        self.delete_unreferenced_locators(locators);
        Ok(ResourceDeleteResult {
            resource_code: command.resource_code,
            deleted,
        })
    }

    fn commit_resource_for_http(
        &self,
        actor_user_code: &str,
        command: ResourceCommitCommand,
        binaries: Vec<ResourceCommitBinary>,
        create: bool,
    ) -> Result<ResourceActivationOutput, ResourceError> {
        self.preflight_resource_commit(actor_user_code, &command, create)?;
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        let all_resources = self.repository(|repository| repository.list_resources())?;
        let (source, definition, expected_local_active) = if create {
            let definition_code = command
                .definition_code
                .as_deref()
                .ok_or(ResourceError::InvalidInput)?;
            (
                None,
                self.definition_for_resource_scope(
                    command.scope_kind,
                    &scope_code,
                    definition_code,
                )?,
                None,
            )
        } else {
            let source = self.effective_active_resource_for_http(
                command.scope_kind,
                &scope_code,
                &command.resource_code,
            )?;
            if command.expected_active_resource_version.as_deref()
                != Some(source.key.resource_version.as_str())
            {
                return Err(ResourceError::ActiveVersionConflict);
            }
            let local_active = all_resources
                .iter()
                .find(|resource| {
                    resource.key.scope_kind == command.scope_kind
                        && resource.key.scope_code == scope_code
                        && resource.key.resource_code == command.resource_code
                        && resource.lifecycle_status == ResourceLifecycleStatus::Active
                })
                .map(|resource| resource.key.resource_version.clone());
            let definition = self.definition_by_resource(&source)?;
            (Some(source), definition, local_active)
        };
        let expected_effective_source = source.as_ref().map(|source| source.key.clone());
        let expected_enabled_definition = create.then(|| definition.key.clone());
        let source_mappings = source
            .as_ref()
            .map(|resource| self.mappings_for_resource(&resource.key))
            .transpose()?
            .unwrap_or_default();
        let now = current_timestamp();
        let resource_version = next_resource_version(
            &all_resources,
            command.scope_kind,
            &scope_code,
            &command.resource_code,
        )?;
        let resource = Resource {
            key: ResourceKey {
                scope_kind: command.scope_kind,
                scope_code: scope_code.clone(),
                resource_code: command.resource_code.clone(),
                resource_version,
            },
            definition_key: definition.key.clone(),
            based_on_resource_version: source.as_ref().and_then(|source| {
                (source.key.scope_kind == command.scope_kind && source.key.scope_code == scope_code)
                    .then(|| source.key.resource_version.clone())
            }),
            name: command.name.trim().to_string(),
            description: command.description.trim().to_string(),
            attributes: command.attributes.clone(),
            allow_inheritance: command.allow_inheritance,
            draft_revision: 0,
            validation_status: ResourceValidationStatus::Valid,
            lifecycle_status: ResourceLifecycleStatus::Active,
            created_at: now.clone(),
            updated_at: now.clone(),
            published_at: Some(now),
        };
        let mut mappings = copied_mappings_for_key(&source_mappings, &resource.key);
        let removed_names = command
            .removed_binary_names
            .iter()
            .map(|name| {
                validate_binary_name(name, self.config.limits.max_binary_name_length)?;
                Ok(name.clone())
            })
            .collect::<Result<BTreeSet<_>, ResourceError>>()?;
        if removed_names.len() != command.removed_binary_names.len() {
            return Err(ResourceError::InvalidInput);
        }
        mappings.retain(|mapping| !removed_names.contains(&mapping.binary_name));
        let mut uploaded_names = BTreeSet::new();
        let mut stored_locators = Vec::new();
        let commit = (|| {
            for mut binary in binaries {
                validate_binary_name(
                    &binary.binary_name,
                    self.config.limits.max_binary_name_length,
                )?;
                if !uploaded_names.insert(binary.binary_name.clone())
                    || removed_names.contains(&binary.binary_name)
                {
                    return Err(ResourceError::InvalidInput);
                }
                let byte_size = binary
                    .source
                    .metadata()
                    .map_err(|_| ResourceError::StorageFailure)?
                    .len();
                if byte_size > self.config.limits.max_binary_bytes {
                    return Err(ResourceError::ValidationFailed);
                }
                let replacing = mappings
                    .iter()
                    .any(|mapping| mapping.binary_name == binary.binary_name);
                validate_binary_upload(
                    &definition,
                    &binary.binary_name,
                    byte_size,
                    &mappings,
                    replacing,
                    &self.config.limits,
                )?;
                binary
                    .source
                    .seek(SeekFrom::Start(0))
                    .map_err(|_| ResourceError::StorageFailure)?;
                let (locator, stored_size) = self.storage.put_reader(
                    &definition.binary_storage_profile,
                    &self.next_object_key(&resource),
                    &mut binary.source,
                )?;
                if let Err(error) = validate_binary_upload(
                    &definition,
                    &binary.binary_name,
                    stored_size,
                    &mappings,
                    replacing,
                    &self.config.limits,
                ) {
                    let _ = self.storage.delete(&locator);
                    return Err(error);
                }
                stored_locators.push(locator.clone());
                mappings.retain(|mapping| mapping.binary_name != binary.binary_name);
                mappings.push(ResourceBinaryMapping {
                    resource_key: resource.key.clone(),
                    binary_name: binary.binary_name,
                    storage_locator: locator,
                    byte_size: stored_size,
                    created_at: current_timestamp(),
                });
            }
            self.ensure_resource_publishable(&resource, &definition, &mappings)?;
            self.repository(|repository| {
                repository.commit_active_resource(
                    &resource,
                    &mappings,
                    expected_local_active.as_deref(),
                    expected_effective_source.as_ref(),
                    expected_enabled_definition.as_ref(),
                )
            })
        })();
        let activation = match commit {
            Ok(activation) => activation,
            Err(error) => {
                for locator in stored_locators {
                    let _ = self.storage.delete(&locator);
                }
                return Err(error);
            }
        };
        self.publish_resource_audit(
            actor_user_code,
            &resource,
            &scope_code,
            activation.activation_result,
            activation.previous_active_version.as_deref(),
        );
        Ok(resource_activation_output(
            &resource,
            activation.activation_result,
            activation.previous_active_version,
        ))
    }

    fn effective_active_resource_for_http(
        &self,
        scope_kind: ResourceScopeKind,
        scope_code: &str,
        resource_code: &str,
    ) -> Result<Resource, ResourceError> {
        match scope_kind {
            ResourceScopeKind::Platform => self
                .repository(|repository| repository.list_resources())?
                .into_iter()
                .find(|resource| {
                    resource.key.scope_kind == ResourceScopeKind::Platform
                        && resource.key.scope_code == scope_code
                        && resource.key.resource_code == resource_code
                        && resource.lifecycle_status == ResourceLifecycleStatus::Active
                })
                .ok_or(ResourceError::NotFound),
            ResourceScopeKind::Workspace => self
                .effective_resources(scope_code)?
                .remove(resource_code)
                .map(|effective| effective.resource)
                .ok_or(ResourceError::NotFound),
        }
    }

    /// Checks every fact that is available before a browser starts streaming a
    /// binary body.  The HTTP adapter calls this before allocating its
    /// anonymous temporary file, so an unauthorised or stale request cannot
    /// consume server disk space.
    pub fn preflight_http_binary_upload(
        &self,
        actor_user_code: &str,
        command: &ResourceHttpBinaryUploadCommand,
    ) -> Result<(), ResourceError> {
        let (_resource, definition, mappings) =
            self.http_binary_upload_target(actor_user_code, command)?;
        let replacing = mappings
            .iter()
            .any(|mapping| mapping.binary_name == command.binary_name);
        validate_binary_upload(
            &definition,
            &command.binary_name,
            0,
            &mappings,
            replacing,
            &self.config.limits,
        )?;
        Ok(())
    }

    pub fn upload_http_binary_from_file(
        &self,
        actor_user_code: &str,
        command: ResourceHttpBinaryUploadCommand,
        source: &mut std::fs::File,
    ) -> Result<ResourceBinaryResult, ResourceError> {
        let (resource, definition, mut mappings) =
            self.http_binary_upload_target(actor_user_code, &command)?;
        let byte_size = source
            .metadata()
            .map_err(|_| ResourceError::StorageFailure)?
            .len();
        if byte_size > self.config.limits.max_binary_bytes {
            return Err(ResourceError::ValidationFailed);
        }
        let replacing = mappings
            .iter()
            .any(|mapping| mapping.binary_name == command.binary_name);
        validate_binary_upload(
            &definition,
            &command.binary_name,
            byte_size,
            &mappings,
            replacing,
            &self.config.limits,
        )?;
        source
            .seek(SeekFrom::Start(0))
            .map_err(|_| ResourceError::StorageFailure)?;
        let object_key = self.next_object_key(&resource);
        let (locator, stored_size) =
            self.storage
                .put_reader(&definition.binary_storage_profile, &object_key, source)?;
        if let Err(error) = validate_binary_upload(
            &definition,
            &command.binary_name,
            stored_size,
            &mappings,
            replacing,
            &self.config.limits,
        ) {
            let _ = self.storage.delete(&locator);
            return Err(error);
        }
        let replaced_locator = mappings
            .iter()
            .find(|mapping| mapping.binary_name == command.binary_name)
            .map(|mapping| mapping.storage_locator.clone());
        mappings.retain(|mapping| mapping.binary_name != command.binary_name);
        mappings.push(ResourceBinaryMapping {
            resource_key: resource.key.clone(),
            binary_name: command.binary_name.clone(),
            storage_locator: locator.clone(),
            byte_size: stored_size,
            created_at: current_timestamp(),
        });
        let updated = match self.update_resource_with_mappings(
            resource,
            &definition,
            mappings.clone(),
            command.expected_draft_revision,
        ) {
            Ok(updated) => updated,
            Err(error) => {
                let _ = self.storage.delete(&locator);
                return Err(error);
            }
        };
        if let Some(locator) = replaced_locator {
            self.delete_unreferenced_locators(vec![locator]);
        }
        Ok(ResourceBinaryResult {
            action: "UPLOADED".to_string(),
            resource_code: command.resource_code,
            resource_version: Some(command.resource_version),
            binary_name: command.binary_name,
            byte_size: Some(stored_size),
            target_path: None,
            draft_revision: Some(updated.draft_revision),
            validation_status: Some(updated.validation_status),
            validation_errors: self.resource_validation_issues(&updated, &definition, &mappings)?,
        })
    }

    pub fn remove_http_binary(
        &self,
        actor_user_code: &str,
        command: ResourceHttpBinaryRemoveCommand,
    ) -> Result<ResourceBinaryResult, ResourceError> {
        validate_resource_code(&command.resource_code)?;
        validate_version_code(&command.resource_version)?;
        validate_binary_name(
            &command.binary_name,
            self.config.limits.max_binary_name_length,
        )?;
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        let key = resource_key(
            command.scope_kind,
            &scope_code,
            &command.resource_code,
            &command.resource_version,
        )?;
        let resource = self.resource_by_key(&key)?.ok_or(ResourceError::NotFound)?;
        if resource.lifecycle_status != ResourceLifecycleStatus::Draft {
            return Err(ResourceError::LifecycleDenied);
        }
        let definition = self.definition_by_resource(&resource)?;
        let mut mappings = self.mappings_for_resource(&key)?;
        let removed = mappings
            .iter()
            .find(|mapping| mapping.binary_name == command.binary_name)
            .cloned()
            .ok_or(ResourceError::NotFound)?;
        mappings.retain(|mapping| mapping.binary_name != command.binary_name);
        let updated = self.update_resource_with_mappings(
            resource,
            &definition,
            mappings.clone(),
            command.expected_draft_revision,
        )?;
        self.delete_unreferenced_locators(vec![removed.storage_locator]);
        Ok(ResourceBinaryResult {
            action: "REMOVED".to_string(),
            resource_code: command.resource_code,
            resource_version: Some(command.resource_version),
            binary_name: command.binary_name,
            byte_size: None,
            target_path: None,
            draft_revision: Some(updated.draft_revision),
            validation_status: Some(updated.validation_status),
            validation_errors: self.resource_validation_issues(&updated, &definition, &mappings)?,
        })
    }

    pub fn download_http_binary_to_file(
        &self,
        actor_user_code: &str,
        command: ResourceHttpBinaryDownloadCommand,
        target: &mut std::fs::File,
    ) -> Result<ResourceHttpBinaryDownload, ResourceError> {
        validate_resource_code(&command.resource_code)?;
        validate_version_code(&command.resource_version)?;
        validate_binary_name(
            &command.binary_name,
            self.config.limits.max_binary_name_length,
        )?;
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            false,
        )?;
        let (resource, _) = self.visible_resource_version(
            command.scope_kind,
            &scope_code,
            &command.resource_code,
            &command.resource_version,
        )?;
        let mapping = self
            .mappings_for_resource(&resource.key)?
            .into_iter()
            .find(|mapping| mapping.binary_name == command.binary_name)
            .ok_or(ResourceError::NotFound)?;
        target
            .set_len(0)
            .and_then(|()| target.seek(SeekFrom::Start(0)).map(|_| ()))
            .map_err(|_| ResourceError::StorageFailure)?;
        let copied = self.storage.get_writer(&mapping.storage_locator, target)?;
        target.flush().map_err(|_| ResourceError::StorageFailure)?;
        target
            .seek(SeekFrom::Start(0))
            .map_err(|_| ResourceError::StorageFailure)?;
        Ok(ResourceHttpBinaryDownload {
            resource_version: resource.key.resource_version,
            binary_name: command.binary_name,
            byte_size: copied,
        })
    }

    pub fn max_binary_bytes(&self) -> u64 {
        self.config.limits.max_binary_bytes
    }

    pub fn max_commit_bytes(&self) -> u64 {
        self.config.limits.max_commit_bytes
    }

    pub fn max_commit_binaries(&self) -> usize {
        self.config.limits.max_binaries
    }

    pub fn max_commit_control_bytes(&self) -> usize {
        self.config
            .limits
            .attributes_max_bytes
            .saturating_add(16 * 1024)
    }

    fn workspace_resource_version_views(
        &self,
        workspace_code: &str,
        resources: &[Resource],
        definitions: &[ResourceDefinition],
        mappings: &[ResourceBinaryMapping],
        lifecycle: Option<ResourceLifecycleStatus>,
    ) -> Result<Vec<ResourceVersionView>, ResourceError> {
        let mut views = Vec::new();
        if lifecycle.is_none() || lifecycle == Some(ResourceLifecycleStatus::Active) {
            for effective in self.effective_resources(workspace_code)?.into_values() {
                views.push(resource_version_view(
                    &effective.resource,
                    effective.source_kind,
                    definitions,
                    mappings,
                    false,
                )?);
            }
        }
        if lifecycle.is_none()
            || lifecycle == Some(ResourceLifecycleStatus::Draft)
            || lifecycle == Some(ResourceLifecycleStatus::Historical)
        {
            for resource in resources.iter().filter(|resource| {
                resource.key.scope_kind == ResourceScopeKind::Workspace
                    && resource.key.scope_code == workspace_code
                    && resource.lifecycle_status != ResourceLifecycleStatus::Active
                    && lifecycle.is_none_or(|expected| resource.lifecycle_status == expected)
            }) {
                views.push(resource_version_view(
                    resource,
                    ResourceSourceKind::Local,
                    definitions,
                    mappings,
                    false,
                )?);
            }
        }
        Ok(views)
    }

    fn visible_resource_version(
        &self,
        scope_kind: ResourceScopeKind,
        scope_code: &str,
        resource_code: &str,
        resource_version: &str,
    ) -> Result<(Resource, ResourceSourceKind), ResourceError> {
        let resources = self.repository(|repository| repository.list_resources())?;
        if scope_kind == ResourceScopeKind::Platform {
            return resources
                .into_iter()
                .find(|resource| {
                    resource.key.scope_kind == ResourceScopeKind::Platform
                        && resource.key.scope_code == scope_code
                        && resource.key.resource_code == resource_code
                        && resource.key.resource_version == resource_version
                })
                .map(|resource| (resource, ResourceSourceKind::Platform))
                .ok_or(ResourceError::NotFound);
        }
        if let Some(resource) = resources.iter().find(|resource| {
            resource.key.scope_kind == ResourceScopeKind::Workspace
                && resource.key.scope_code == scope_code
                && resource.key.resource_code == resource_code
                && resource.key.resource_version == resource_version
                && resource.lifecycle_status != ResourceLifecycleStatus::Active
        }) {
            return Ok((resource.clone(), ResourceSourceKind::Local));
        }
        self.effective_resources(scope_code)?
            .remove(resource_code)
            .filter(|effective| effective.resource.key.resource_version == resource_version)
            .map(|effective| (effective.resource, effective.source_kind))
            .ok_or(ResourceError::NotFound)
    }

    fn ensure_resource_publishable(
        &self,
        resource: &Resource,
        definition: &ResourceDefinition,
        mappings: &[ResourceBinaryMapping],
    ) -> Result<(), ResourceError> {
        if !self
            .resource_validation_issues(resource, definition, mappings)?
            .is_empty()
        {
            return Err(ResourceError::ValidationFailed);
        }
        for mapping in mappings {
            if !self.storage.exists(&mapping.storage_locator)? {
                return Err(ResourceError::MissingBinaryObject);
            }
        }
        let keys =
            declared_variable_references(&definition.attributes_schema, &resource.attributes)?;
        let variable_result = match resource.key.scope_kind {
            ResourceScopeKind::Platform => self
                .variables
                .ensure_platform_runtime_variable_keys(&keys)
                .and_then(|()| {
                    self.variables
                        .resolve_platform_runtime_variable_values(&keys)
                        .map(|_| ())
                }),
            ResourceScopeKind::Workspace => self
                .variables
                .ensure_runtime_variable_keys(&resource.key.scope_code, &keys)
                .and_then(|()| {
                    self.variables
                        .resolve_runtime_variable_values(&resource.key.scope_code, &keys)
                        .map(|_| ())
                }),
        };
        map_variable_result(variable_result)
    }

    fn http_binary_upload_target(
        &self,
        actor_user_code: &str,
        command: &ResourceHttpBinaryUploadCommand,
    ) -> Result<(Resource, ResourceDefinition, Vec<ResourceBinaryMapping>), ResourceError> {
        validate_resource_code(&command.resource_code)?;
        validate_version_code(&command.resource_version)?;
        validate_binary_name(
            &command.binary_name,
            self.config.limits.max_binary_name_length,
        )?;
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        let key = resource_key(
            command.scope_kind,
            &scope_code,
            &command.resource_code,
            &command.resource_version,
        )?;
        let resource = self.resource_by_key(&key)?.ok_or(ResourceError::NotFound)?;
        if resource.lifecycle_status != ResourceLifecycleStatus::Draft {
            return Err(ResourceError::LifecycleDenied);
        }
        if resource.draft_revision != command.expected_draft_revision {
            return Err(ResourceError::DraftRevisionConflict);
        }
        let definition = self.definition_by_resource(&resource)?;
        if !self.storage.has_profile(&definition.binary_storage_profile) {
            return Err(ResourceError::StorageProfileNotFound);
        }
        let mappings = self.mappings_for_resource(&key)?;
        Ok((resource, definition, mappings))
    }
}

fn resource_version_view(
    resource: &Resource,
    source_kind: ResourceSourceKind,
    definitions: &[ResourceDefinition],
    mappings: &[ResourceBinaryMapping],
    include_detail: bool,
) -> Result<ResourceVersionView, ResourceError> {
    let definition = definitions
        .iter()
        .find(|definition| definition.key == resource.definition_key)
        .ok_or(ResourceError::NotFound)?;
    Ok(ResourceVersionView {
        scope_kind: resource.key.scope_kind,
        scope_code: resource.key.scope_code.clone(),
        source_kind,
        resource_code: resource.key.resource_code.clone(),
        resource_version: resource.key.resource_version.clone(),
        definition_scope_kind: resource.definition_key.scope_kind,
        definition_scope_code: resource.definition_key.scope_code.clone(),
        definition_code: resource.definition_key.definition_code.clone(),
        definition_version: resource.definition_key.definition_version.clone(),
        definition_name: definition.name.clone(),
        definition_status: definition.definition_status,
        name: resource.name.clone(),
        description: resource.description.clone(),
        attributes: include_detail.then(|| resource.attributes.clone()),
        attributes_schema: include_detail.then(|| definition.attributes_schema.clone()),
        binary_schema: include_detail.then(|| definition.binary_schema.clone()),
        binary_storage_profile: include_detail.then(|| definition.binary_storage_profile.clone()),
        allow_inheritance: resource.allow_inheritance,
        lifecycle_status: resource.lifecycle_status,
        validation_status: resource.validation_status,
        draft_revision: (resource.lifecycle_status == ResourceLifecycleStatus::Draft)
            .then_some(resource.draft_revision),
        binaries: include_detail.then(|| binary_summaries(resource, mappings)),
        created_at: resource.created_at.clone(),
        updated_at: resource.updated_at.clone(),
        published_at: resource.published_at.clone(),
    })
}

fn binary_summaries(
    resource: &Resource,
    mappings: &[ResourceBinaryMapping],
) -> Vec<ResourceBinarySummary> {
    mappings
        .iter()
        .filter(|mapping| mapping.resource_key == resource.key)
        .map(|mapping| ResourceBinarySummary {
            binary_name: mapping.binary_name.clone(),
            byte_size: mapping.byte_size,
        })
        .collect()
}

fn resource_page_matches(view: &ResourceVersionView, query: &ResourceVersionPageQuery) -> bool {
    query
        .resource_code
        .as_deref()
        .is_none_or(|value| view.resource_code == value)
        && query
            .definition_code
            .as_deref()
            .is_none_or(|value| view.definition_code == value)
        && query
            .definition_status
            .is_none_or(|value| view.definition_status == value)
        && query
            .lifecycle_status
            .is_none_or(|value| view.lifecycle_status == value)
        && query
            .validation_status
            .is_none_or(|value| view.validation_status == value)
        && query
            .source_kind
            .is_none_or(|value| view.source_kind == value)
        && query.search.as_deref().is_none_or(|search| {
            let search = search.trim().to_ascii_lowercase();
            search.is_empty()
                || view.resource_code.to_ascii_lowercase().contains(&search)
                || view.name.to_ascii_lowercase().contains(&search)
                || view.description.to_ascii_lowercase().contains(&search)
        })
}

fn resource_update_has_content(command: &ResourceUpdateDraftCommand) -> bool {
    command.name.is_some()
        || command.description.is_some()
        || command.attributes.is_some()
        || command.allow_inheritance.is_some()
}

fn resource_draft_result(
    resource: &Resource,
    validation_errors: Vec<ResourceValidationIssue>,
    draft_result: &str,
    discarded: bool,
) -> ResourceDraftResult {
    ResourceDraftResult {
        draft_result: draft_result.to_string(),
        resource_code: resource.key.resource_code.clone(),
        resource_version: resource.key.resource_version.clone(),
        definition_scope_kind: resource.definition_key.scope_kind,
        definition_scope_code: resource.definition_key.scope_code.clone(),
        definition_code: resource.definition_key.definition_code.clone(),
        definition_version: resource.definition_key.definition_version.clone(),
        draft_revision: resource.draft_revision,
        validation_status: resource.validation_status,
        validation_errors,
        discarded: discarded.then_some(true),
        resource_removed: Some(false),
    }
}

fn resource_activation_output(
    resource: &Resource,
    activation_result: ResourceActivationResult,
    previous_active_resource_version: Option<String>,
) -> ResourceActivationOutput {
    ResourceActivationOutput {
        resource_code: resource.key.resource_code.clone(),
        resource_version: resource.key.resource_version.clone(),
        definition_scope_kind: resource.definition_key.scope_kind,
        definition_scope_code: resource.definition_key.scope_code.clone(),
        definition_code: resource.definition_key.definition_code.clone(),
        definition_version: resource.definition_key.definition_version.clone(),
        lifecycle_status: ResourceLifecycleStatus::Active,
        activation_result,
        previous_active_resource_version,
    }
}

fn resource_key(
    scope_kind: ResourceScopeKind,
    scope_code: &str,
    resource_code: &str,
    resource_version: &str,
) -> Result<ResourceKey, ResourceError> {
    validate_resource_code(resource_code)?;
    validate_version_code(resource_version)?;
    Ok(ResourceKey {
        scope_kind,
        scope_code: scope_code.to_string(),
        resource_code: resource_code.to_string(),
        resource_version: resource_version.to_string(),
    })
}

fn copied_mappings_for_key(
    mappings: &[ResourceBinaryMapping],
    key: &ResourceKey,
) -> Vec<ResourceBinaryMapping> {
    mappings
        .iter()
        .map(|mapping| ResourceBinaryMapping {
            resource_key: key.clone(),
            binary_name: mapping.binary_name.clone(),
            storage_locator: mapping.storage_locator.clone(),
            byte_size: mapping.byte_size,
            created_at: current_timestamp(),
        })
        .collect()
}

fn map_variable_result(result: Result<(), VariableError>) -> Result<(), ResourceError> {
    match result {
        Ok(()) => Ok(()),
        Err(VariableError::NotFound) => Err(ResourceError::ValidationFailed),
        Err(VariableError::InvalidKey) => Err(ResourceError::InvalidVariableReference),
        Err(VariableError::InvalidWorkspace | VariableError::WorkspaceRequired) => {
            Err(ResourceError::InvalidWorkspace)
        }
        Err(VariableError::StorageFailure) => Err(ResourceError::StorageFailure),
        Err(_) => Err(ResourceError::ValidationFailed),
    }
}

fn sort_resource_views(views: &mut [ResourceVersionView]) {
    views.sort_by(|left, right| {
        left.resource_code
            .cmp(&right.resource_code)
            .then_with(|| right.resource_version.cmp(&left.resource_version))
    });
}
