use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

use rand::{Rng, RngCore};

use crate::{
    application::resource::{
        ResourceActivationOutput, ResourceBinaryAction, ResourceBinaryCommand,
        ResourceBinaryResult, ResourceDraftAction, ResourceDraftCommand, ResourceDraftResult,
        ResourcePage, ResourcePublishCommand, ResourceQuery, ResourceValidationIssue,
    },
    domain::resource::{
        Resource, ResourceActivationResult, ResourceBinaryMapping, ResourceDefinition,
        ResourceError, ResourceKey, ResourceLifecycleStatus, ResourceScopeKind, ResourceSourceKind,
        ResourceValidationStatus, validate_binary_name, validate_definition_code,
        validate_resource_code, validate_version_code,
    },
    domain::variable::VariableError,
    shared::{
        event_bus::{BusActorRef, BusEventDraft},
        time::current_timestamp,
    },
};

use super::{
    ResourceService,
    validation::{declared_variable_references, validate_binary_upload, validate_resource_draft},
};

impl ResourceService {
    pub fn query_resources(
        &self,
        actor_user_code: &str,
        query: ResourceQuery,
    ) -> Result<ResourcePage, ResourceError> {
        validate_resource_query(&query)?;
        let workspace_code = query
            .workspace_code
            .as_deref()
            .ok_or(ResourceError::WorkspaceRequired)?;
        self.authorize_workspace_read(actor_user_code, workspace_code)?;
        let workspace_code = self.active_workspace_code(workspace_code)?;
        let include_detail = query.resource_code.is_some();
        let all_resources = self.repository(|repository| repository.list_resources())?;
        let mappings = self.repository(|repository| repository.list_binary_mappings())?;
        let mut records = if query.include_history || query.resource_version.is_some() {
            all_resources
                .iter()
                .filter(|resource| {
                    resource.key.scope_kind == ResourceScopeKind::Workspace
                        && resource.key.scope_code == workspace_code
                })
                .map(|resource| {
                    self.resource_record(
                        resource,
                        ResourceSourceKind::Local,
                        &mappings,
                        None,
                        include_detail,
                    )
                })
                .collect::<Vec<_>>()
        } else {
            let drafts = local_drafts(&all_resources, &workspace_code);
            let effective = self.effective_resources(&workspace_code)?;
            let effective_codes = include_detail.then(|| {
                effective
                    .keys()
                    .cloned()
                    .collect::<std::collections::BTreeSet<_>>()
            });
            let mut records = effective
                .into_values()
                .map(|effective| {
                    self.resource_record(
                        &effective.resource,
                        effective.source_kind,
                        &mappings,
                        drafts.get(&effective.resource.key.resource_code).copied(),
                        include_detail,
                    )
                })
                .collect::<Vec<_>>();
            if include_detail {
                records.extend(
                    drafts
                        .into_iter()
                        .filter(|(resource_code, _)| {
                            !effective_codes
                                .as_ref()
                                .expect("exact resource query has effective codes")
                                .contains(resource_code)
                        })
                        .map(|(_, draft)| {
                            self.resource_record(
                                draft,
                                ResourceSourceKind::Local,
                                &mappings,
                                None,
                                true,
                            )
                        }),
                );
            }
            records
        };
        records.retain(|record| resource_matches(record, &query));
        records.sort_by(|left, right| left.resource_code.cmp(&right.resource_code));
        let total = records.len();
        let (current, size, start, end) = resource_page_bounds(query.current, query.size, total);
        Ok(ResourcePage {
            current,
            size,
            total,
            records: records[start..end].to_vec(),
        })
    }

    pub fn draft_resource(
        &self,
        actor_user_code: &str,
        command: ResourceDraftCommand,
    ) -> Result<ResourceDraftResult, ResourceError> {
        validate_resource_code(&command.resource_code)?;
        let workspace_code = command
            .workspace_code
            .as_deref()
            .ok_or(ResourceError::WorkspaceRequired)?;
        self.authorize_workspace_write(actor_user_code, workspace_code)?;
        let workspace_code = self.active_workspace_code(workspace_code)?;
        match command.action {
            ResourceDraftAction::Create => {
                let definition_code = command
                    .definition_code
                    .as_deref()
                    .ok_or(ResourceError::InvalidInput)?;
                validate_definition_code(definition_code)?;
                require_non_blank_resource_name(command.name.as_deref())?;
                let definition =
                    self.effective_definition_for_resource(&workspace_code, definition_code)?;
                self.ensure_resource_create_is_available(&workspace_code, &command.resource_code)?;
                let resource = self.create_resource_draft(
                    ResourceScopeKind::Workspace,
                    &workspace_code,
                    &command,
                    &definition,
                    None,
                    Vec::new(),
                )?;
                let issues = self.resource_validation_issues(&resource, &definition, &[])?;
                self.repository(|repository| repository.insert_resource(&resource, &[]))?;
                Ok(resource_draft_result(&resource, issues, "CREATED", false))
            }
            ResourceDraftAction::Update => {
                if command.definition_code.is_some() {
                    return Err(ResourceError::InvalidInput);
                }
                let resource_version = command
                    .resource_version
                    .as_deref()
                    .ok_or(ResourceError::InvalidInput)?;
                validate_version_code(resource_version)?;
                let key = ResourceKey {
                    scope_kind: ResourceScopeKind::Workspace,
                    scope_code: workspace_code.clone(),
                    resource_code: command.resource_code.clone(),
                    resource_version: resource_version.to_string(),
                };
                match self.resource_by_key(&key)? {
                    Some(existing)
                        if existing.lifecycle_status == ResourceLifecycleStatus::Draft =>
                    {
                        let expected_revision = command
                            .expected_draft_revision
                            .ok_or(ResourceError::DraftRevisionConflict)?;
                        let definition = self.definition_by_resource(&existing)?;
                        let mappings = self.mappings_for_resource(&existing.key)?;
                        let updated = self.apply_resource_changes(
                            existing,
                            &command,
                            &definition,
                            &mappings,
                        )?;
                        self.repository(|repository| {
                            repository.update_resource_draft(&updated, &mappings, expected_revision)
                        })?;
                        let issues =
                            self.resource_validation_issues(&updated, &definition, &mappings)?;
                        Ok(resource_draft_result(
                            &updated,
                            issues,
                            "DRAFT_UPDATED",
                            false,
                        ))
                    }
                    Some(_) | None if command.expected_draft_revision.is_some() => {
                        Err(ResourceError::DraftRevisionConflict)
                    }
                    _ => {
                        let source =
                            self.resource_source_for_override(&workspace_code, &command)?;
                        let is_override = source.key.scope_kind != ResourceScopeKind::Workspace
                            || source.key.scope_code != workspace_code;
                        let definition = self.effective_definition_for_resource(
                            &workspace_code,
                            &source.definition_key.definition_code,
                        )?;
                        let source_mappings = self.mappings_for_resource(&source.key)?;
                        let resource = self.create_resource_draft(
                            ResourceScopeKind::Workspace,
                            &workspace_code,
                            &command,
                            &definition,
                            Some(source),
                            source_mappings.clone(),
                        )?;
                        let mappings = copied_mappings_for_draft(&source_mappings, &resource.key);
                        let issues =
                            self.resource_validation_issues(&resource, &definition, &mappings)?;
                        self.repository(|repository| {
                            repository.insert_resource(&resource, &mappings)
                        })?;
                        Ok(resource_draft_result(
                            &resource,
                            issues,
                            if is_override {
                                "OVERRIDE_DRAFT_CREATED"
                            } else {
                                "DRAFT_CREATED"
                            },
                            false,
                        ))
                    }
                }
            }
            ResourceDraftAction::Discard => {
                let resource_version = command
                    .resource_version
                    .as_deref()
                    .ok_or(ResourceError::InvalidInput)?;
                validate_version_code(resource_version)?;
                let expected_revision = command
                    .expected_draft_revision
                    .ok_or(ResourceError::DraftRevisionConflict)?;
                let key = ResourceKey {
                    scope_kind: ResourceScopeKind::Workspace,
                    scope_code: workspace_code.clone(),
                    resource_code: command.resource_code.clone(),
                    resource_version: resource_version.to_string(),
                };
                let (discarded, locators) = self.repository(|repository| {
                    repository.discard_resource_draft(&key, expected_revision)
                })?;
                self.delete_unreferenced_locators(locators);
                Ok(ResourceDraftResult {
                    draft_result: "DISCARDED".to_string(),
                    resource_code: command.resource_code,
                    resource_version: resource_version.to_string(),
                    definition_scope_kind: ResourceScopeKind::Workspace,
                    definition_scope_code: workspace_code,
                    definition_code: String::new(),
                    definition_version: String::new(),
                    draft_revision: expected_revision,
                    validation_status: ResourceValidationStatus::Valid,
                    validation_errors: Vec::new(),
                    discarded: Some(discarded),
                    resource_removed: Some(false),
                })
            }
        }
    }

    pub fn publish_resource(
        &self,
        actor_user_code: &str,
        command: ResourcePublishCommand,
    ) -> Result<ResourceActivationOutput, ResourceError> {
        validate_resource_code(&command.resource_code)?;
        validate_version_code(&command.resource_version)?;
        let workspace_code = command
            .workspace_code
            .as_deref()
            .ok_or(ResourceError::WorkspaceRequired)?;
        self.authorize_workspace_write(actor_user_code, workspace_code)?;
        let workspace_code = self.active_workspace_code(workspace_code)?;
        let key = ResourceKey {
            scope_kind: ResourceScopeKind::Workspace,
            scope_code: workspace_code.clone(),
            resource_code: command.resource_code.clone(),
            resource_version: command.resource_version.clone(),
        };
        let resource = self.resource_by_key(&key)?.ok_or(ResourceError::NotFound)?;
        let definition = self.definition_by_resource(&resource)?;
        let mappings = self.mappings_for_resource(&resource.key)?;
        if resource.lifecycle_status != ResourceLifecycleStatus::Active {
            if !self
                .resource_validation_issues(&resource, &definition, &mappings)?
                .is_empty()
            {
                return Err(ResourceError::ValidationFailed);
            }
            for mapping in &mappings {
                if !self.storage.exists(&mapping.storage_locator)? {
                    return Err(ResourceError::MissingBinaryObject);
                }
            }
            let keys =
                declared_variable_references(&definition.attributes_schema, &resource.attributes)?;
            self.variables
                .ensure_runtime_variable_keys(&workspace_code, &keys)
                .map_err(|_| ResourceError::ValidationFailed)?;
            let runtime_values = self
                .variables
                .resolve_runtime_variable_values(&workspace_code, &keys)
                .map_err(|_| ResourceError::ValidationFailed)?;
            drop(runtime_values);
        }
        let activation = self.repository(|repository| {
            repository.activate_resource(
                &key,
                command.expected_draft_revision,
                command.expected_active_resource_version.as_deref(),
                &current_timestamp(),
            )
        })?;
        if activation.activation_result != ResourceActivationResult::AlreadyActive {
            self.publish_resource_audit(
                actor_user_code,
                &resource,
                &workspace_code,
                activation.activation_result,
                activation.previous_active_version.as_deref(),
            );
        }
        Ok(ResourceActivationOutput {
            resource_code: resource.key.resource_code,
            resource_version: resource.key.resource_version,
            definition_scope_kind: resource.definition_key.scope_kind,
            definition_scope_code: resource.definition_key.scope_code,
            definition_code: resource.definition_key.definition_code,
            definition_version: resource.definition_key.definition_version,
            lifecycle_status: ResourceLifecycleStatus::Active,
            activation_result: activation.activation_result,
            previous_active_resource_version: activation.previous_active_version,
        })
    }

    pub fn handle_resource_binary(
        &self,
        actor_user_code: &str,
        command: ResourceBinaryCommand,
    ) -> Result<ResourceBinaryResult, ResourceError> {
        validate_resource_code(&command.resource_code)?;
        validate_binary_name(
            &command.binary_name,
            self.config.limits.max_binary_name_length,
        )?;
        let workspace_code = command
            .workspace_code
            .as_deref()
            .ok_or(ResourceError::WorkspaceRequired)?;
        match command.action {
            ResourceBinaryAction::Download => {
                self.authorize_workspace_read(actor_user_code, workspace_code)?;
                self.download_resource_binary(&self.active_workspace_code(workspace_code)?, command)
            }
            ResourceBinaryAction::Upload | ResourceBinaryAction::Remove => {
                self.authorize_workspace_write(actor_user_code, workspace_code)?;
                self.mutate_resource_binary(&self.active_workspace_code(workspace_code)?, command)
            }
        }
    }

    pub(super) fn create_resource_draft(
        &self,
        scope_kind: ResourceScopeKind,
        scope_code: &str,
        command: &ResourceDraftCommand,
        definition: &ResourceDefinition,
        source: Option<Resource>,
        mappings: Vec<ResourceBinaryMapping>,
    ) -> Result<Resource, ResourceError> {
        // Disabling a definition prevents creating a new resource, but it must
        // not strand an existing resource that a user explicitly chose to keep
        // as incomplete work.  Existing sources retain their versioned
        // definition contract while the draft is completed.
        if source.is_none()
            && definition.definition_status
                != crate::domain::resource::ResourceDefinitionStatus::Enabled
        {
            return Err(ResourceError::DefinitionDisabled);
        }
        let all_resources = self.repository(|repository| repository.list_resources())?;
        if all_resources.iter().any(|resource| {
            resource.key.scope_kind == scope_kind
                && resource.key.scope_code == scope_code
                && resource.key.resource_code == command.resource_code
                && resource.lifecycle_status == ResourceLifecycleStatus::Draft
        }) {
            return Err(ResourceError::Conflict);
        }
        let now = current_timestamp();
        let resource_version = next_resource_version(
            &all_resources,
            scope_kind,
            scope_code,
            &command.resource_code,
        )?;
        let based_on_resource_version = source.as_ref().and_then(|source| {
            (source.key.scope_kind == scope_kind
                && source.key.scope_code == scope_code
                && source.lifecycle_status != ResourceLifecycleStatus::Draft)
                .then(|| source.key.resource_version.clone())
        });
        let base = source.unwrap_or_else(|| Resource {
            key: ResourceKey {
                scope_kind,
                scope_code: scope_code.to_string(),
                resource_code: command.resource_code.clone(),
                resource_version: resource_version.clone(),
            },
            definition_key: definition.key.clone(),
            based_on_resource_version: None,
            name: command.resource_code.clone(),
            description: String::new(),
            attributes: serde_json::json!({}),
            allow_inheritance: true,
            draft_revision: 0,
            validation_status: ResourceValidationStatus::Valid,
            lifecycle_status: ResourceLifecycleStatus::Draft,
            created_at: now.clone(),
            updated_at: now,
            published_at: None,
        });
        let mut resource = Resource {
            key: ResourceKey {
                scope_kind,
                scope_code: scope_code.to_string(),
                resource_code: command.resource_code.clone(),
                resource_version,
            },
            definition_key: definition.key.clone(),
            based_on_resource_version,
            lifecycle_status: ResourceLifecycleStatus::Draft,
            draft_revision: 0,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            published_at: None,
            ..base
        };
        resource = self.apply_resource_changes(resource, command, definition, &mappings)?;
        Ok(resource)
    }

    pub(super) fn apply_resource_changes(
        &self,
        mut resource: Resource,
        command: &ResourceDraftCommand,
        definition: &ResourceDefinition,
        mappings: &[ResourceBinaryMapping],
    ) -> Result<Resource, ResourceError> {
        if let Some(name) = &command.name {
            let name = name.trim();
            if name.is_empty() {
                return Err(ResourceError::InvalidInput);
            }
            resource.name = name.to_string();
        }
        if let Some(description) = &command.description {
            resource.description = description.trim().to_string();
        }
        if let Some(attributes) = &command.attributes {
            resource.attributes = attributes.clone();
        }
        if let Some(allow_inheritance) = command.allow_inheritance {
            resource.allow_inheritance = allow_inheritance;
        }
        if resource.name.trim().is_empty() {
            return Err(ResourceError::InvalidInput);
        }
        resource.lifecycle_status = ResourceLifecycleStatus::Draft;
        resource.draft_revision = resource.draft_revision.saturating_add(1);
        resource.updated_at = current_timestamp();
        let issues = self.resource_validation_issues(&resource, definition, mappings)?;
        resource.validation_status = if issues.is_empty() {
            ResourceValidationStatus::Valid
        } else {
            ResourceValidationStatus::Invalid
        };
        Ok(resource)
    }

    pub(super) fn resource_validation_issues(
        &self,
        resource: &Resource,
        definition: &ResourceDefinition,
        mappings: &[ResourceBinaryMapping],
    ) -> Result<Vec<ResourceValidationIssue>, ResourceError> {
        let mut issues = validate_resource_draft(
            definition,
            &resource.attributes,
            mappings,
            &self.config.limits,
        )?;
        let keys =
            declared_variable_references(&definition.attributes_schema, &resource.attributes)?;
        let variable_check = match resource.key.scope_kind {
            ResourceScopeKind::Platform => {
                self.variables.ensure_platform_runtime_variable_keys(&keys)
            }
            ResourceScopeKind::Workspace => self
                .variables
                .ensure_runtime_variable_keys(&resource.key.scope_code, &keys),
        };
        match variable_check {
            Ok(()) => {}
            Err(VariableError::NotFound) => issues.push(ResourceValidationIssue {
                path: "/".to_string(),
                message: "one or more declared Variable references do not resolve".to_string(),
            }),
            Err(VariableError::InvalidKey) => return Err(ResourceError::InvalidVariableReference),
            Err(VariableError::InvalidWorkspace | VariableError::WorkspaceRequired) => {
                return Err(ResourceError::InvalidWorkspace);
            }
            Err(VariableError::StorageFailure) => return Err(ResourceError::StorageFailure),
            Err(_) => return Err(ResourceError::ValidationFailed),
        }
        issues.truncate(self.config.limits.max_validation_errors);
        Ok(issues)
    }

    fn ensure_resource_create_is_available(
        &self,
        workspace_code: &str,
        resource_code: &str,
    ) -> Result<(), ResourceError> {
        self.ensure_resource_create_is_available_for_scope(
            ResourceScopeKind::Workspace,
            workspace_code,
            resource_code,
        )
    }

    pub(super) fn ensure_resource_create_is_available_for_scope(
        &self,
        scope_kind: ResourceScopeKind,
        scope_code: &str,
        resource_code: &str,
    ) -> Result<(), ResourceError> {
        let locally_owned = self.repository(|repository| repository.list_resources())?;
        if locally_owned.iter().any(|resource| {
            resource.key.scope_kind == scope_kind
                && resource.key.scope_code == scope_code
                && resource.key.resource_code == resource_code
        }) {
            return Err(ResourceError::Conflict);
        }
        if scope_kind == ResourceScopeKind::Workspace
            && self
                .effective_resources(scope_code)?
                .contains_key(resource_code)
        {
            return Err(ResourceError::Conflict);
        }
        Ok(())
    }

    fn effective_definition_for_resource(
        &self,
        workspace_code: &str,
        definition_code: &str,
    ) -> Result<ResourceDefinition, ResourceError> {
        self.definition_for_resource_scope(
            ResourceScopeKind::Workspace,
            workspace_code,
            definition_code,
        )
    }

    pub(super) fn definition_for_resource_scope(
        &self,
        scope_kind: ResourceScopeKind,
        scope_code: &str,
        definition_code: &str,
    ) -> Result<ResourceDefinition, ResourceError> {
        let definition = match scope_kind {
            ResourceScopeKind::Platform => self
                .repository(|repository| repository.list_definitions())?
                .into_iter()
                .find(|definition| {
                    definition.key.scope_kind == ResourceScopeKind::Platform
                        && definition.key.scope_code == scope_code
                        && definition.key.definition_code == definition_code
                        && definition.lifecycle_status == ResourceLifecycleStatus::Active
                }),
            ResourceScopeKind::Workspace => self
                .effective_definitions(scope_code)?
                .remove(definition_code)
                .map(|effective| effective.definition),
        }
        .ok_or(ResourceError::NotFound)?;
        if definition.definition_status
            != crate::domain::resource::ResourceDefinitionStatus::Enabled
        {
            return Err(ResourceError::DefinitionDisabled);
        }
        Ok(definition)
    }

    fn resource_source_for_override(
        &self,
        workspace_code: &str,
        command: &ResourceDraftCommand,
    ) -> Result<Resource, ResourceError> {
        let all_resources = self.repository(|repository| repository.list_resources())?;
        if let Some(version) = command.resource_version.as_deref() {
            if let Some(resource) = all_resources.into_iter().find(|resource| {
                resource.key.scope_kind == ResourceScopeKind::Workspace
                    && resource.key.scope_code == workspace_code
                    && resource.key.resource_code == command.resource_code
                    && resource.key.resource_version == version
                    && resource.lifecycle_status != ResourceLifecycleStatus::Draft
            }) {
                return Ok(resource);
            }
        }
        let source = self
            .effective_resources(workspace_code)?
            .remove(&command.resource_code)
            .map(|effective| effective.resource)
            .ok_or(ResourceError::NotFound)?;
        if command
            .resource_version
            .as_deref()
            .is_some_and(|version| version != source.key.resource_version)
        {
            return Err(ResourceError::NotFound);
        }
        Ok(source)
    }

    pub(super) fn resource_by_key(
        &self,
        key: &ResourceKey,
    ) -> Result<Option<Resource>, ResourceError> {
        self.repository(|repository| {
            Ok(repository
                .list_resources()?
                .into_iter()
                .find(|resource| resource.key == *key))
        })
    }

    pub(super) fn definition_by_resource(
        &self,
        resource: &Resource,
    ) -> Result<ResourceDefinition, ResourceError> {
        self.repository(|repository| {
            repository
                .list_definitions()?
                .into_iter()
                .find(|definition| definition.key == resource.definition_key)
                .ok_or(ResourceError::NotFound)
        })
    }

    pub(super) fn mappings_for_resource(
        &self,
        key: &ResourceKey,
    ) -> Result<Vec<ResourceBinaryMapping>, ResourceError> {
        self.repository(|repository| {
            Ok(repository
                .list_binary_mappings()?
                .into_iter()
                .filter(|mapping| mapping.resource_key == *key)
                .collect())
        })
    }

    fn mutate_resource_binary(
        &self,
        workspace_code: &str,
        command: ResourceBinaryCommand,
    ) -> Result<ResourceBinaryResult, ResourceError> {
        let resource_version = command
            .resource_version
            .as_deref()
            .ok_or(ResourceError::InvalidInput)?;
        validate_version_code(resource_version)?;
        let expected_revision = command
            .expected_draft_revision
            .ok_or(ResourceError::DraftRevisionConflict)?;
        let key = ResourceKey {
            scope_kind: ResourceScopeKind::Workspace,
            scope_code: workspace_code.to_string(),
            resource_code: command.resource_code.clone(),
            resource_version: resource_version.to_string(),
        };
        let resource = self.resource_by_key(&key)?.ok_or(ResourceError::NotFound)?;
        if resource.lifecycle_status != ResourceLifecycleStatus::Draft {
            return Err(ResourceError::LifecycleDenied);
        }
        let definition = self.definition_by_resource(&resource)?;
        let mut mappings = self.mappings_for_resource(&resource.key)?;
        match command.action {
            ResourceBinaryAction::Upload => {
                let source_path = command
                    .source_path
                    .as_deref()
                    .ok_or(ResourceError::InvalidInput)?;
                let source = self.resolve_agent_file_path(
                    source_path,
                    command.agent_local_directory.as_deref(),
                )?;
                let byte_size = fs::metadata(&source)
                    .map_err(|_| ResourceError::PathDenied)?
                    .len();
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
                let object_key = self.next_object_key(&resource);
                let (locator, stored_size) = self.storage.put_file(
                    &definition.binary_storage_profile,
                    &object_key,
                    &source,
                )?;
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
                let updated = self.update_resource_with_mappings(
                    resource,
                    &definition,
                    mappings.clone(),
                    expected_revision,
                );
                let updated = match updated {
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
                    resource_version: Some(resource_version.to_string()),
                    binary_name: command.binary_name,
                    byte_size: Some(stored_size),
                    target_path: None,
                    draft_revision: Some(updated.draft_revision),
                    validation_status: Some(updated.validation_status),
                    validation_errors: self.resource_validation_issues(
                        &updated,
                        &definition,
                        &mappings,
                    )?,
                })
            }
            ResourceBinaryAction::Remove => {
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
                    expected_revision,
                )?;
                self.delete_unreferenced_locators(vec![removed.storage_locator]);
                Ok(ResourceBinaryResult {
                    action: "REMOVED".to_string(),
                    resource_code: command.resource_code,
                    resource_version: Some(resource_version.to_string()),
                    binary_name: command.binary_name,
                    byte_size: None,
                    target_path: None,
                    draft_revision: Some(updated.draft_revision),
                    validation_status: Some(updated.validation_status),
                    validation_errors: self.resource_validation_issues(
                        &updated,
                        &definition,
                        &mappings,
                    )?,
                })
            }
            ResourceBinaryAction::Download => Err(ResourceError::InvalidInput),
        }
    }

    pub(super) fn update_resource_with_mappings(
        &self,
        resource: Resource,
        definition: &ResourceDefinition,
        mappings: Vec<ResourceBinaryMapping>,
        expected_revision: i64,
    ) -> Result<Resource, ResourceError> {
        let mut updated = resource;
        updated.draft_revision = updated.draft_revision.saturating_add(1);
        updated.updated_at = current_timestamp();
        let issues = self.resource_validation_issues(&updated, definition, &mappings)?;
        updated.validation_status = if issues.is_empty() {
            ResourceValidationStatus::Valid
        } else {
            ResourceValidationStatus::Invalid
        };
        self.repository(|repository| {
            repository.update_resource_draft(&updated, &mappings, expected_revision)
        })?;
        Ok(updated)
    }

    fn download_resource_binary(
        &self,
        workspace_code: &str,
        command: ResourceBinaryCommand,
    ) -> Result<ResourceBinaryResult, ResourceError> {
        let resource = self.resource_for_binary_download(workspace_code, &command)?;
        let mapping = self
            .mappings_for_resource(&resource.key)?
            .into_iter()
            .find(|mapping| mapping.binary_name == command.binary_name)
            .ok_or(ResourceError::NotFound)?;
        let target_path = command
            .target_path
            .as_deref()
            .ok_or(ResourceError::InvalidInput)?;
        let target = self.resolve_agent_target_path(
            target_path,
            command.agent_local_directory.as_deref(),
            command.overwrite,
        )?;
        let temporary = sibling_temporary_path(&target)?;
        let copied = match self.storage.get_file(&mapping.storage_locator, &temporary) {
            Ok(copied) => copied,
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                return Err(error);
            }
        };
        if fs::rename(&temporary, &target).is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(ResourceError::StorageFailure);
        }
        Ok(ResourceBinaryResult {
            action: "DOWNLOADED".to_string(),
            resource_code: command.resource_code,
            resource_version: Some(resource.key.resource_version),
            binary_name: command.binary_name,
            byte_size: Some(copied),
            target_path: Some(target.display().to_string()),
            draft_revision: None,
            validation_status: None,
            validation_errors: Vec::new(),
        })
    }

    pub(super) fn resource_for_binary_download(
        &self,
        workspace_code: &str,
        command: &ResourceBinaryCommand,
    ) -> Result<Resource, ResourceError> {
        if let Some(version) = command.resource_version.as_deref() {
            validate_version_code(version)?;
            return self
                .repository(|repository| repository.list_resources())?
                .into_iter()
                .find(|resource| {
                    resource.key.scope_kind == ResourceScopeKind::Workspace
                        && resource.key.scope_code == workspace_code
                        && resource.key.resource_code == command.resource_code
                        && resource.key.resource_version == version
                })
                .ok_or(ResourceError::NotFound);
        }
        self.effective_resources(workspace_code)?
            .remove(&command.resource_code)
            .map(|effective| effective.resource)
            .ok_or(ResourceError::NotFound)
    }

    fn resolve_agent_file_path(
        &self,
        raw_path: &str,
        agent_local_directory: Option<&str>,
    ) -> Result<PathBuf, ResourceError> {
        let path = self.resolve_agent_path(raw_path, agent_local_directory, true)?;
        let metadata = fs::symlink_metadata(&path).map_err(|_| ResourceError::PathDenied)?;
        if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
            return Err(ResourceError::PathDenied);
        }
        Ok(path)
    }

    fn resolve_agent_target_path(
        &self,
        raw_path: &str,
        agent_local_directory: Option<&str>,
        overwrite: bool,
    ) -> Result<PathBuf, ResourceError> {
        let path = self.resolve_agent_path(raw_path, agent_local_directory, false)?;
        if path.exists() {
            let metadata = fs::symlink_metadata(&path).map_err(|_| ResourceError::PathDenied)?;
            if metadata.file_type().is_symlink() || !metadata.file_type().is_file() || !overwrite {
                return Err(ResourceError::PathDenied);
            }
        }
        Ok(path)
    }

    fn resolve_agent_path(
        &self,
        raw_path: &str,
        agent_local_directory: Option<&str>,
        must_exist: bool,
    ) -> Result<PathBuf, ResourceError> {
        let raw_path = raw_path.trim();
        if raw_path.is_empty()
            || raw_path.starts_with('~')
            || raw_path.contains('$')
            || raw_path.contains('\0')
        {
            return Err(ResourceError::PathDenied);
        }
        let input = Path::new(raw_path);
        if input
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(ResourceError::PathDenied);
        }
        let temp_root = std::env::temp_dir()
            .canonicalize()
            .map_err(|_| ResourceError::PathDenied)?;
        let local_root = agent_local_directory
            .filter(|path| !path.trim().is_empty())
            .map(PathBuf::from)
            .map(|path| path.canonicalize().map_err(|_| ResourceError::PathDenied))
            .transpose()?;
        let base = local_root.clone().unwrap_or_else(|| temp_root.clone());
        let candidate = if input.is_absolute() {
            input.to_path_buf()
        } else {
            base.join(input)
        };
        let canonical = if must_exist {
            if fs::symlink_metadata(&candidate)
                .map_err(|_| ResourceError::PathDenied)?
                .file_type()
                .is_symlink()
            {
                return Err(ResourceError::PathDenied);
            }
            candidate
                .canonicalize()
                .map_err(|_| ResourceError::PathDenied)?
        } else {
            let parent = candidate.parent().ok_or(ResourceError::PathDenied)?;
            let parent = parent
                .canonicalize()
                .map_err(|_| ResourceError::PathDenied)?;
            let file_name = candidate.file_name().ok_or(ResourceError::PathDenied)?;
            parent.join(file_name)
        };
        let allowed = canonical.starts_with(&temp_root)
            || local_root
                .as_ref()
                .is_some_and(|root| canonical.starts_with(root));
        if !allowed
            || self
                .storage_roots()
                .iter()
                .any(|root| canonical.starts_with(root))
        {
            return Err(ResourceError::PathDenied);
        }
        Ok(canonical)
    }

    pub(super) fn next_object_key(&self, resource: &Resource) -> String {
        format!(
            "resources/{}/{}/{}/{:016X}",
            resource.key.scope_code,
            resource.key.resource_code,
            resource.key.resource_version,
            rand::rng().next_u64(),
        )
    }

    pub(super) fn delete_unreferenced_locators(&self, locators: Vec<String>) {
        for locator in locators {
            let referenced = self
                .repository(|repository| repository.locator_reference_count(&locator))
                .unwrap_or(1);
            if referenced == 0 {
                let _ = self.storage.delete(&locator);
            }
        }
    }

    pub(super) fn publish_resource_audit(
        &self,
        actor_user_code: &str,
        resource: &Resource,
        workspace_code: &str,
        activation_result: ResourceActivationResult,
        previous_active_resource_version: Option<&str>,
    ) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit("resource.published", "resource_service")
                .subject("resource", resource.key.resource_code.clone())
                .actor(BusActorRef {
                    user_code: actor_user_code.to_string(),
                    username: actor_user_code.to_string(),
                })
                .summary("Resource published")
                .attr("workspaceCode", workspace_code.to_string())
                .attr("version", resource.key.resource_version.clone())
                .attr(
                    "definitionCode",
                    resource.definition_key.definition_code.clone(),
                )
                .attr(
                    "activationResult",
                    activation_result_code(activation_result).to_string(),
                )
                .attr(
                    "previousActiveResourceVersion",
                    previous_active_resource_version
                        .unwrap_or_default()
                        .to_string(),
                ),
        );
    }
}

fn local_drafts<'a>(
    resources: &'a [Resource],
    workspace_code: &str,
) -> BTreeMap<String, &'a Resource> {
    resources
        .iter()
        .filter(|resource| {
            resource.key.scope_kind == ResourceScopeKind::Workspace
                && resource.key.scope_code == workspace_code
                && resource.lifecycle_status == ResourceLifecycleStatus::Draft
        })
        .map(|resource| (resource.key.resource_code.clone(), resource))
        .collect()
}

fn copied_mappings_for_draft(
    mappings: &[ResourceBinaryMapping],
    resource_key: &ResourceKey,
) -> Vec<ResourceBinaryMapping> {
    mappings
        .iter()
        .map(|mapping| ResourceBinaryMapping {
            resource_key: resource_key.clone(),
            binary_name: mapping.binary_name.clone(),
            storage_locator: mapping.storage_locator.clone(),
            byte_size: mapping.byte_size,
            created_at: current_timestamp(),
        })
        .collect()
}

fn resource_matches(
    record: &crate::application::resource::ResourceRecord,
    query: &ResourceQuery,
) -> bool {
    if let Some(resource_code) = query.resource_code.as_deref() {
        if record.resource_code != resource_code {
            return false;
        }
    }
    if let Some(resource_version) = query.resource_version.as_deref() {
        if record.resource_version.as_deref() != Some(resource_version) {
            return false;
        }
    }
    if let Some(definition_code) = query.definition_code.as_deref() {
        if record.definition_code != definition_code {
            return false;
        }
    }
    query.search.as_deref().is_none_or(|search| {
        let search = search.trim().to_ascii_lowercase();
        search.is_empty()
            || record.resource_code.to_ascii_lowercase().contains(&search)
            || record.name.to_ascii_lowercase().contains(&search)
            || record.description.to_ascii_lowercase().contains(&search)
    })
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

pub(super) fn next_resource_version(
    resources: &[Resource],
    scope_kind: ResourceScopeKind,
    scope_code: &str,
    resource_code: &str,
) -> Result<String, ResourceError> {
    let sequence = resources
        .iter()
        .filter(|resource| {
            resource.key.scope_kind == scope_kind
                && resource.key.scope_code == scope_code
                && resource.key.resource_code == resource_code
                && resource.lifecycle_status != ResourceLifecycleStatus::Draft
        })
        .count()
        + 1;
    let existing = resources
        .iter()
        .filter(|resource| {
            resource.key.scope_kind == scope_kind
                && resource.key.scope_code == scope_code
                && resource.key.resource_code == resource_code
        })
        .map(|resource| resource.key.resource_version.as_str())
        .collect::<BTreeSet<_>>();
    let mut rng = rand::rng();
    for _ in 0..128 {
        let candidate = format!("v{sequence}-{}", random_resource_version_suffix(&mut rng));
        if !existing.contains(candidate.as_str()) {
            return Ok(candidate);
        }
    }
    Err(ResourceError::Conflict)
}

const RESOURCE_VERSION_SUFFIX_ALPHABET: &[u8] =
    b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn random_resource_version_suffix(rng: &mut impl RngCore) -> String {
    (0..3)
        .map(|_| {
            let index = rng.random_range(0..RESOURCE_VERSION_SUFFIX_ALPHABET.len());
            RESOURCE_VERSION_SUFFIX_ALPHABET[index] as char
        })
        .collect()
}

fn require_non_blank_resource_name(value: Option<&str>) -> Result<(), ResourceError> {
    if value.is_none_or(|value| value.trim().is_empty()) {
        return Err(ResourceError::InvalidInput);
    }
    Ok(())
}

fn validate_resource_query(query: &ResourceQuery) -> Result<(), ResourceError> {
    if query.resource_version.is_some() && query.resource_code.is_none()
        || query.resource_version.is_some() && query.include_history
    {
        return Err(ResourceError::InvalidInput);
    }
    Ok(())
}

fn activation_result_code(value: ResourceActivationResult) -> &'static str {
    match value {
        ResourceActivationResult::DirectCommitted => "COMMITTED",
        ResourceActivationResult::DraftPublished => "DRAFT_PUBLISHED",
        ResourceActivationResult::HistoricalReactivated => "HISTORICAL_REACTIVATED",
        ResourceActivationResult::AlreadyActive => "ALREADY_ACTIVE",
    }
}

fn resource_page_bounds(current: usize, size: usize, total: usize) -> (usize, usize, usize, usize) {
    let current = current.max(1);
    let size = size.clamp(1, 50);
    let start = current.saturating_sub(1).saturating_mul(size).min(total);
    let end = start.saturating_add(size).min(total);
    (current, size, start, end)
}

fn sibling_temporary_path(target: &Path) -> Result<PathBuf, ResourceError> {
    let parent = target.parent().ok_or(ResourceError::PathDenied)?;
    let file_name = target.file_name().ok_or(ResourceError::PathDenied)?;
    Ok(parent.join(format!(
        ".{}.resource-{:016X}.tmp",
        file_name.to_string_lossy(),
        rand::rng().next_u64(),
    )))
}
