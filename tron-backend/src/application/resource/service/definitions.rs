use std::collections::{BTreeMap, BTreeSet};

use rand::{Rng, RngCore};
use serde_json::{Value, json};

use crate::{
    application::resource::{
        ResourceDefinitionActivationResult, ResourceDefinitionDraftAction,
        ResourceDefinitionDraftCommand, ResourceDefinitionDraftResult, ResourceDefinitionPage,
        ResourceDefinitionPublishCommand, ResourceDefinitionQuery, ResourceValidationIssue,
    },
    domain::resource::{
        ResourceActivationResult, ResourceDefinition, ResourceDefinitionKey,
        ResourceDefinitionStatus, ResourceError, ResourceLifecycleStatus, ResourceScopeKind,
        ResourceSourceKind, ResourceValidationStatus, validate_definition_code,
        validate_version_code,
    },
    shared::{
        event_bus::{BusActorRef, BusEventDraft},
        time::current_timestamp,
    },
};

use super::{
    ResourceService,
    validation::{validate_definition_content, validate_definition_persistence},
};

impl ResourceService {
    pub fn query_definitions(
        &self,
        actor_user_code: &str,
        query: ResourceDefinitionQuery,
    ) -> Result<ResourceDefinitionPage, ResourceError> {
        validate_definition_query(&query)?;
        let scope_code = self.workspace_scope(
            actor_user_code,
            query.scope_kind,
            query.workspace_code.as_deref(),
            false,
        )?;
        let definitions = self.repository(|repository| repository.list_definitions())?;
        let exact_version = query.definition_version.is_some();
        let mut records = if exact_version || query.include_history {
            definitions
                .iter()
                .filter(|definition| {
                    definition.key.scope_kind == query.scope_kind
                        && definition.key.scope_code == scope_code
                })
                .map(|definition| {
                    self.definition_record(
                        &definition,
                        if query.scope_kind == ResourceScopeKind::Platform {
                            ResourceSourceKind::Platform
                        } else {
                            ResourceSourceKind::Local
                        },
                        exact_version,
                    )
                })
                .collect::<Vec<_>>()
        } else {
            self.logical_definition_records(
                query.scope_kind,
                &scope_code,
                query.definition_code.as_deref(),
                &definitions,
            )?
        };
        records.retain(|record| definition_matches(record, &query));
        records.sort_by(|left, right| {
            left.definition_code
                .cmp(&right.definition_code)
                .then_with(|| right.definition_version.cmp(&left.definition_version))
        });
        let total = records.len();
        let (current, size, start, end) = page_bounds(query.current, query.size, total);
        Ok(ResourceDefinitionPage {
            result_kind: if query.definition_version.is_some() || query.include_history {
                "VERSION".to_string()
            } else {
                "DEFINITION".to_string()
            },
            current,
            size,
            total,
            records: records[start..end].to_vec(),
        })
    }

    pub fn draft_definition(
        &self,
        actor_user_code: &str,
        command: ResourceDefinitionDraftCommand,
    ) -> Result<ResourceDefinitionDraftResult, ResourceError> {
        validate_definition_code(&command.definition_code)?;
        let scope_code = self.workspace_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        match command.action {
            ResourceDefinitionDraftAction::Create => {
                require_non_blank_name(command.name.as_deref())?;
                let definition = self.create_definition_draft(&scope_code, &command, None)?;
                let validation_errors = definition_validation_issues(&definition, self)?;
                self.repository(|repository| repository.insert_definition(&definition))?;
                Ok(definition_draft_result(
                    &definition,
                    validation_errors,
                    false,
                ))
            }
            ResourceDefinitionDraftAction::Update => {
                ensure_definition_update_has_content(&command)?;
                let definition_version = command
                    .definition_version
                    .as_deref()
                    .ok_or(ResourceError::InvalidInput)?;
                validate_version_code(definition_version)?;
                let key = ResourceDefinitionKey {
                    scope_kind: command.scope_kind,
                    scope_code: scope_code.clone(),
                    definition_code: command.definition_code.clone(),
                    definition_version: definition_version.to_string(),
                };
                match self.definition_by_key(&key)? {
                    Some(existing)
                        if existing.lifecycle_status == ResourceLifecycleStatus::Draft =>
                    {
                        let expected_revision = command
                            .expected_draft_revision
                            .ok_or(ResourceError::DraftRevisionConflict)?;
                        let updated = self.apply_definition_changes(existing, &command)?;
                        self.repository(|repository| {
                            repository.update_definition_draft(&updated, expected_revision)
                        })?;
                        let validation_errors = definition_validation_issues(&updated, self)?;
                        Ok(definition_draft_result(&updated, validation_errors, false))
                    }
                    Some(existing)
                        if existing.lifecycle_status == ResourceLifecycleStatus::Active =>
                    {
                        if command.expected_draft_revision.is_some() {
                            return Err(ResourceError::InvalidInput);
                        }
                        let definition =
                            self.create_definition_draft(&scope_code, &command, Some(existing))?;
                        let validation_errors = definition_validation_issues(&definition, self)?;
                        self.repository(|repository| repository.insert_definition(&definition))?;
                        Ok(definition_draft_result(
                            &definition,
                            validation_errors,
                            false,
                        ))
                    }
                    Some(_) => Err(ResourceError::LifecycleDenied),
                    None => {
                        if command.expected_draft_revision.is_some() {
                            return Err(ResourceError::InvalidInput);
                        }
                        let source = self.definition_source_for_override(&scope_code, &command)?;
                        let definition =
                            self.create_definition_draft(&scope_code, &command, Some(source))?;
                        let validation_errors = definition_validation_issues(&definition, self)?;
                        self.repository(|repository| repository.insert_definition(&definition))?;
                        Ok(definition_draft_result(
                            &definition,
                            validation_errors,
                            false,
                        ))
                    }
                }
            }
            ResourceDefinitionDraftAction::Discard => {
                let definition_version = command
                    .definition_version
                    .as_deref()
                    .ok_or(ResourceError::InvalidInput)?;
                validate_version_code(definition_version)?;
                let expected_revision = command
                    .expected_draft_revision
                    .ok_or(ResourceError::DraftRevisionConflict)?;
                let key = ResourceDefinitionKey {
                    scope_kind: command.scope_kind,
                    scope_code: scope_code.clone(),
                    definition_code: command.definition_code.clone(),
                    definition_version: definition_version.to_string(),
                };
                let discarded = self.repository(|repository| {
                    repository.discard_definition_draft(&key, expected_revision)
                })?;
                Ok(ResourceDefinitionDraftResult {
                    scope_kind: command.scope_kind,
                    scope_code,
                    definition_code: command.definition_code,
                    definition_version: definition_version.to_string(),
                    draft_revision: expected_revision,
                    lifecycle_status: ResourceLifecycleStatus::Draft,
                    validation_status: ResourceValidationStatus::Valid,
                    validation_errors: Vec::new(),
                    discarded: Some(discarded),
                })
            }
        }
    }

    pub fn publish_definition(
        &self,
        actor_user_code: &str,
        command: ResourceDefinitionPublishCommand,
    ) -> Result<ResourceDefinitionActivationResult, ResourceError> {
        validate_definition_code(&command.definition_code)?;
        validate_version_code(&command.definition_version)?;
        let scope_code = self.workspace_scope(
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
        let definition = self
            .definition_by_key(&key)?
            .ok_or(ResourceError::NotFound)?;
        if definition.lifecycle_status != ResourceLifecycleStatus::Active {
            let issues =
                validate_definition_content(&definition, &self.config.limits, &self.storage)
                    .map_err(|_| ResourceError::ValidationFailed)?;
            if !issues.is_empty() {
                return Err(ResourceError::ValidationFailed);
            }
        }
        let activation = self.repository(|repository| {
            repository.activate_definition(
                &key,
                command.expected_draft_revision,
                command.expected_active_definition_version.as_deref(),
                &current_timestamp(),
            )
        })?;
        if activation.activation_result != ResourceActivationResult::AlreadyActive {
            self.publish_audit(
                "resource_definition.published",
                actor_user_code,
                &command.definition_code,
                &scope_code,
                &command.definition_version,
            );
        }
        Ok(ResourceDefinitionActivationResult {
            definition_code: command.definition_code,
            definition_version: command.definition_version,
            lifecycle_status: ResourceLifecycleStatus::Active,
            activation_result: activation.activation_result,
            previous_active_definition_version: activation.previous_active_version,
        })
    }

    fn logical_definition_records(
        &self,
        scope_kind: ResourceScopeKind,
        scope_code: &str,
        definition_code: Option<&str>,
        definitions: &[ResourceDefinition],
    ) -> Result<Vec<crate::application::resource::ResourceDefinitionRecord>, ResourceError> {
        let (effective, drafts) = match scope_kind {
            ResourceScopeKind::Platform => {
                let effective = definitions
                    .iter()
                    .filter(|definition| {
                        definition.key.scope_kind == ResourceScopeKind::Platform
                            && definition.key.scope_code == scope_code
                            && definition.lifecycle_status == ResourceLifecycleStatus::Active
                    })
                    .map(|definition| {
                        (
                            definition.key.definition_code.clone(),
                            super::projection::EffectiveDefinition {
                                definition: definition.clone(),
                                source_kind: ResourceSourceKind::Platform,
                            },
                        )
                    })
                    .collect::<BTreeMap<_, _>>();
                (effective, BTreeMap::new())
            }
            ResourceScopeKind::Workspace => (
                self.effective_definitions(scope_code)?,
                scoped_definition_drafts(definitions, scope_kind, scope_code),
            ),
        };
        let mut codes = effective.keys().cloned().collect::<BTreeSet<_>>();
        codes.extend(drafts.keys().cloned());

        Ok(codes
            .into_iter()
            .filter(|code| definition_code.map_or(true, |expected| expected == code))
            .filter_map(|code| {
                let draft = drafts.get(&code);
                effective.get(&code).map_or_else(
                    || {
                        draft.map(|draft| {
                            self.definition_record(
                                draft,
                                if scope_kind == ResourceScopeKind::Platform {
                                    ResourceSourceKind::Platform
                                } else {
                                    ResourceSourceKind::Local
                                },
                                definition_code.is_some(),
                            )
                        })
                    },
                    |effective| {
                        let mut record = self.definition_record(
                            &effective.definition,
                            effective.source_kind,
                            definition_code.is_some() && draft.is_none(),
                        );
                        record.local_draft = draft.map(|draft| {
                            self.definition_draft_record(draft, definition_code.is_some())
                        });
                        Some(record)
                    },
                )
            })
            .collect())
    }

    pub(super) fn create_definition_draft(
        &self,
        scope_code: &str,
        command: &ResourceDefinitionDraftCommand,
        source: Option<ResourceDefinition>,
    ) -> Result<ResourceDefinition, ResourceError> {
        let existing = self.repository(|repository| repository.list_definitions())?;
        let group_exists = existing.iter().any(|definition| {
            definition.key.scope_kind == command.scope_kind
                && definition.key.scope_code == scope_code
                && definition.key.definition_code == command.definition_code
        });
        let draft_exists = existing.iter().any(|definition| {
            definition.key.scope_kind == command.scope_kind
                && definition.key.scope_code == scope_code
                && definition.key.definition_code == command.definition_code
                && definition.lifecycle_status == ResourceLifecycleStatus::Draft
        });
        if (source.is_none() && group_exists) || (source.is_some() && draft_exists) {
            return Err(ResourceError::Conflict);
        }
        let now = current_timestamp();
        let definition_version = next_definition_version(
            &existing,
            command.scope_kind,
            scope_code,
            &command.definition_code,
        )?;
        let base = source.unwrap_or_else(|| ResourceDefinition {
            key: ResourceDefinitionKey {
                scope_kind: command.scope_kind,
                scope_code: scope_code.to_string(),
                definition_code: command.definition_code.clone(),
                definition_version: definition_version.clone(),
            },
            name: command.definition_code.clone(),
            description: String::new(),
            definition_status: ResourceDefinitionStatus::Enabled,
            allow_inheritance: true,
            attributes_schema: default_attributes_schema(),
            binary_schema: default_binary_schema(),
            binary_storage_profile: "@default".to_string(),
            draft_revision: 0,
            validation_status: ResourceValidationStatus::Valid,
            lifecycle_status: ResourceLifecycleStatus::Draft,
            created_at: now.clone(),
            updated_at: now,
            published_at: None,
        });
        let mut draft = ResourceDefinition {
            key: ResourceDefinitionKey {
                scope_kind: command.scope_kind,
                scope_code: scope_code.to_string(),
                definition_code: command.definition_code.clone(),
                definition_version,
            },
            lifecycle_status: ResourceLifecycleStatus::Draft,
            draft_revision: 0,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            published_at: None,
            ..base
        };
        draft = self.apply_definition_changes(draft, command)?;
        Ok(draft)
    }

    pub(super) fn apply_definition_changes(
        &self,
        mut definition: ResourceDefinition,
        command: &ResourceDefinitionDraftCommand,
    ) -> Result<ResourceDefinition, ResourceError> {
        if let Some(name) = &command.name {
            let name = name.trim();
            if name.is_empty() {
                return Err(ResourceError::InvalidInput);
            }
            definition.name = name.to_string();
        }
        if let Some(description) = &command.description {
            definition.description = description.trim().to_string();
        }
        if let Some(status) = command.definition_status {
            definition.definition_status = status;
        }
        if let Some(allow_inheritance) = command.allow_inheritance {
            definition.allow_inheritance =
                if definition.key.scope_kind == ResourceScopeKind::Platform {
                    true
                } else {
                    allow_inheritance
                };
        }
        if let Some(attributes_schema) = &command.attributes_schema {
            definition.attributes_schema = if attributes_schema
                .as_object()
                .is_some_and(|value| value.is_empty())
            {
                default_attributes_schema()
            } else {
                attributes_schema.clone()
            };
        }
        if let Some(binary_schema) = &command.binary_schema {
            definition.binary_schema = if binary_schema
                .as_object()
                .is_some_and(|value| value.is_empty())
            {
                default_binary_schema()
            } else {
                binary_schema.clone()
            };
        }
        if let Some(profile) = &command.binary_storage_profile {
            definition.binary_storage_profile = if profile.trim().is_empty() {
                "@default".to_string()
            } else {
                profile.trim().to_string()
            };
        }
        if definition.name.trim().is_empty() {
            return Err(ResourceError::InvalidInput);
        }
        definition.lifecycle_status = ResourceLifecycleStatus::Draft;
        definition.draft_revision = definition.draft_revision.saturating_add(1);
        definition.updated_at = current_timestamp();
        let issues = definition_validation_issues(&definition, self)?;
        definition.validation_status = if issues.is_empty() {
            ResourceValidationStatus::Valid
        } else {
            ResourceValidationStatus::Invalid
        };
        Ok(definition)
    }

    fn definition_source_for_override(
        &self,
        scope_code: &str,
        command: &ResourceDefinitionDraftCommand,
    ) -> Result<ResourceDefinition, ResourceError> {
        if command.scope_kind == ResourceScopeKind::Platform {
            return Err(ResourceError::NotFound);
        }
        let source = self
            .effective_definitions(scope_code)?
            .remove(&command.definition_code)
            .map(|effective| effective.definition)
            .ok_or(ResourceError::NotFound)?;
        if command
            .definition_version
            .as_deref()
            .is_some_and(|version| version != source.key.definition_version)
        {
            return Err(ResourceError::NotFound);
        }
        Ok(source)
    }

    pub(super) fn definition_by_key(
        &self,
        key: &ResourceDefinitionKey,
    ) -> Result<Option<ResourceDefinition>, ResourceError> {
        self.repository(|repository| {
            Ok(repository
                .list_definitions()?
                .into_iter()
                .find(|definition| definition.key == *key))
        })
    }

    pub(super) fn publish_audit(
        &self,
        event_name: &'static str,
        actor_user_code: &str,
        subject_code: &str,
        scope_code: &str,
        version: &str,
    ) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit(event_name, "resource_service")
                .subject("resource_definition", subject_code.to_string())
                .actor(BusActorRef {
                    user_code: actor_user_code.to_string(),
                    username: actor_user_code.to_string(),
                })
                .summary("Resource definition published")
                .attr("scopeCode", scope_code.to_string())
                .attr("version", version.to_string()),
        );
    }
}

fn definition_matches(
    record: &crate::application::resource::ResourceDefinitionRecord,
    query: &ResourceDefinitionQuery,
) -> bool {
    if let Some(definition_code) = query.definition_code.as_deref() {
        if record.definition_code != definition_code {
            return false;
        }
    }
    if let Some(definition_version) = query.definition_version.as_deref() {
        if record.definition_version != definition_version {
            return false;
        }
    }
    if let Some(status) = query.definition_status {
        if record.definition_status != status {
            return false;
        }
    }
    if let Some(status) = query.lifecycle_status {
        if record.lifecycle_status != status {
            return false;
        }
    }
    query.search.as_deref().is_none_or(|search| {
        let search = search.trim().to_ascii_lowercase();
        search.is_empty()
            || record
                .definition_code
                .to_ascii_lowercase()
                .contains(&search)
            || record.name.to_ascii_lowercase().contains(&search)
            || record.description.to_ascii_lowercase().contains(&search)
    })
}

fn scoped_definition_drafts(
    definitions: &[ResourceDefinition],
    scope_kind: ResourceScopeKind,
    scope_code: &str,
) -> BTreeMap<String, ResourceDefinition> {
    definitions
        .iter()
        .filter(|definition| {
            definition.key.scope_kind == scope_kind
                && definition.key.scope_code == scope_code
                && definition.lifecycle_status == ResourceLifecycleStatus::Draft
        })
        .map(|definition| (definition.key.definition_code.clone(), definition.clone()))
        .collect()
}

fn validate_definition_query(query: &ResourceDefinitionQuery) -> Result<(), ResourceError> {
    if query.definition_version.is_some() && query.definition_code.is_none()
        || query.definition_version.is_some() && query.include_history
        || query.lifecycle_status.is_some() && !query.include_history
    {
        return Err(ResourceError::InvalidInput);
    }
    Ok(())
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

pub(super) fn definition_validation_issues(
    definition: &ResourceDefinition,
    service: &ResourceService,
) -> Result<Vec<ResourceValidationIssue>, ResourceError> {
    validate_definition_persistence(definition, &service.config.limits)?;
    match validate_definition_content(definition, &service.config.limits, &service.storage) {
        Ok(issues) => Ok(issues),
        Err(ResourceError::InvalidBinarySchema | ResourceError::StorageProfileNotFound) => {
            Ok(vec![ResourceValidationIssue {
                path: "/".to_string(),
                message: "definition content is not valid for publication".to_string(),
            }])
        }
        Err(error) => Err(error),
    }
}

fn require_non_blank_name(value: Option<&str>) -> Result<(), ResourceError> {
    if value.is_none_or(|value| value.trim().is_empty()) {
        return Err(ResourceError::InvalidInput);
    }
    Ok(())
}

fn ensure_definition_update_has_content(
    command: &ResourceDefinitionDraftCommand,
) -> Result<(), ResourceError> {
    if command.name.is_none()
        && command.description.is_none()
        && command.definition_status.is_none()
        && command.allow_inheritance.is_none()
        && command.attributes_schema.is_none()
        && command.binary_schema.is_none()
        && command.binary_storage_profile.is_none()
    {
        return Err(ResourceError::InvalidInput);
    }
    Ok(())
}

const VERSION_SUFFIX_ALPHABET: &[u8] =
    b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn random_version_suffix(rng: &mut impl RngCore) -> String {
    (0..3)
        .map(|_| {
            let index = rng.random_range(0..VERSION_SUFFIX_ALPHABET.len());
            VERSION_SUFFIX_ALPHABET[index] as char
        })
        .collect()
}

fn default_attributes_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "properties": {},
        "additionalProperties": false
    })
}

fn default_binary_schema() -> Value {
    json!({ "minCount": 0, "maxCount": 0 })
}

pub(super) fn next_definition_version(
    definitions: &[ResourceDefinition],
    scope_kind: ResourceScopeKind,
    scope_code: &str,
    definition_code: &str,
) -> Result<String, ResourceError> {
    let sequence = definitions
        .iter()
        .filter(|definition| {
            definition.key.scope_kind == scope_kind
                && definition.key.scope_code == scope_code
                && definition.key.definition_code == definition_code
                && definition.lifecycle_status != ResourceLifecycleStatus::Draft
        })
        .count()
        + 1;
    let existing = definitions
        .iter()
        .filter(|definition| {
            definition.key.scope_kind == scope_kind
                && definition.key.scope_code == scope_code
                && definition.key.definition_code == definition_code
        })
        .map(|definition| definition.key.definition_version.as_str())
        .collect::<BTreeSet<_>>();
    let mut rng = rand::rng();
    for _ in 0..128 {
        let candidate = format!("v{sequence}-{}", random_version_suffix(&mut rng));
        if !existing.contains(candidate.as_str()) {
            return Ok(candidate);
        }
    }
    Err(ResourceError::Conflict)
}

fn page_bounds(current: usize, size: usize, total: usize) -> (usize, usize, usize, usize) {
    let current = current.max(1);
    let size = size.clamp(1, 50);
    let start = current.saturating_sub(1).saturating_mul(size).min(total);
    let end = start.saturating_add(size).min(total);
    (current, size, start, end)
}
