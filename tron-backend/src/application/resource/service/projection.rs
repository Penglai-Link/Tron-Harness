use std::collections::BTreeMap;

use crate::{
    application::resource::{
        ResourceBinarySummary, ResourceDefinitionDraftRecord, ResourceDefinitionRecord,
        ResourceDraftSummary, ResourceRecord,
    },
    domain::{
        resource::{
            PLATFORM_SCOPE_CODE, Resource, ResourceBinaryMapping, ResourceDefinition,
            ResourceError, ResourceLifecycleStatus, ResourceScopeKind, ResourceSourceKind,
        },
        workspace::WorkspaceStatus,
    },
};

use super::ResourceService;

#[derive(Debug, Clone)]
pub(super) struct EffectiveDefinition {
    pub definition: ResourceDefinition,
    pub source_kind: ResourceSourceKind,
}

#[derive(Debug, Clone)]
pub(super) struct EffectiveResource {
    pub resource: Resource,
    pub source_kind: ResourceSourceKind,
}

impl ResourceService {
    pub(super) fn active_workspace_code(
        &self,
        workspace_code: &str,
    ) -> Result<String, ResourceError> {
        let workspace_code = workspace_code.trim();
        if workspace_code.is_empty() {
            return Err(ResourceError::WorkspaceRequired);
        }
        let workspace = self
            .workspaces
            .get_workspace(workspace_code)
            .map_err(|_| ResourceError::InvalidWorkspace)?;
        if workspace.status != WorkspaceStatus::Active {
            return Err(ResourceError::InvalidWorkspace);
        }
        Ok(workspace.workspace_code)
    }

    pub(super) fn effective_definitions(
        &self,
        workspace_code: &str,
    ) -> Result<BTreeMap<String, EffectiveDefinition>, ResourceError> {
        let workspace_code = self.active_workspace_code(workspace_code)?;
        let workspace = self
            .workspaces
            .get_workspace(&workspace_code)
            .map_err(|_| ResourceError::InvalidWorkspace)?;
        let definitions = self.repository(|repository| repository.list_definitions())?;
        let mut effective = BTreeMap::new();

        for definition in definitions.iter().filter(|definition| {
            definition.key.scope_kind == ResourceScopeKind::Platform
                && definition.key.scope_code == PLATFORM_SCOPE_CODE
                && definition.lifecycle_status == ResourceLifecycleStatus::Active
        }) {
            effective.insert(
                definition.key.definition_code.clone(),
                EffectiveDefinition {
                    definition: definition.clone(),
                    source_kind: ResourceSourceKind::Platform,
                },
            );
        }

        for lineage in &workspace.path {
            for definition in definitions.iter().filter(|definition| {
                definition.key.scope_kind == ResourceScopeKind::Workspace
                    && definition.key.scope_code == lineage.workspace_code
                    && definition.lifecycle_status == ResourceLifecycleStatus::Active
            }) {
                if lineage.workspace_code != workspace_code && !definition.allow_inheritance {
                    continue;
                }
                let source_kind = if lineage.workspace_code == workspace_code {
                    if effective.contains_key(&definition.key.definition_code) {
                        ResourceSourceKind::Override
                    } else {
                        ResourceSourceKind::Local
                    }
                } else {
                    ResourceSourceKind::Inherited
                };
                effective.insert(
                    definition.key.definition_code.clone(),
                    EffectiveDefinition {
                        definition: definition.clone(),
                        source_kind,
                    },
                );
            }
        }
        Ok(effective)
    }

    pub(super) fn effective_resources(
        &self,
        workspace_code: &str,
    ) -> Result<BTreeMap<String, EffectiveResource>, ResourceError> {
        let workspace_code = self.active_workspace_code(workspace_code)?;
        let workspace = self
            .workspaces
            .get_workspace(&workspace_code)
            .map_err(|_| ResourceError::InvalidWorkspace)?;
        let resources = self.repository(|repository| repository.list_resources())?;
        let mut effective = BTreeMap::new();

        for resource in resources.iter().filter(|resource| {
            resource.key.scope_kind == ResourceScopeKind::Platform
                && resource.key.scope_code == PLATFORM_SCOPE_CODE
                && resource.lifecycle_status == ResourceLifecycleStatus::Active
        }) {
            effective.insert(
                resource.key.resource_code.clone(),
                EffectiveResource {
                    resource: resource.clone(),
                    source_kind: ResourceSourceKind::Platform,
                },
            );
        }

        for lineage in &workspace.path {
            for resource in resources.iter().filter(|resource| {
                resource.key.scope_kind == ResourceScopeKind::Workspace
                    && resource.key.scope_code == lineage.workspace_code
                    && resource.lifecycle_status == ResourceLifecycleStatus::Active
            }) {
                if lineage.workspace_code != workspace_code && !resource.allow_inheritance {
                    continue;
                }
                let source_kind = if lineage.workspace_code == workspace_code {
                    if effective.contains_key(&resource.key.resource_code) {
                        ResourceSourceKind::Override
                    } else {
                        ResourceSourceKind::Local
                    }
                } else {
                    ResourceSourceKind::Inherited
                };
                effective.insert(
                    resource.key.resource_code.clone(),
                    EffectiveResource {
                        resource: resource.clone(),
                        source_kind,
                    },
                );
            }
        }
        Ok(effective)
    }

    pub(super) fn definition_record(
        &self,
        definition: &ResourceDefinition,
        source_kind: ResourceSourceKind,
        include_schema: bool,
    ) -> ResourceDefinitionRecord {
        ResourceDefinitionRecord {
            scope_kind: definition.key.scope_kind,
            scope_code: definition.key.scope_code.clone(),
            source_kind,
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
            attributes_schema: include_schema.then(|| definition.attributes_schema.clone()),
            binary_schema: include_schema.then(|| definition.binary_schema.clone()),
            binary_storage_profile: include_schema
                .then(|| definition.binary_storage_profile.clone()),
            published_at: definition.published_at.clone(),
            local_draft: None,
        }
    }

    pub(super) fn definition_draft_record(
        &self,
        definition: &ResourceDefinition,
        include_schema: bool,
    ) -> ResourceDefinitionDraftRecord {
        ResourceDefinitionDraftRecord {
            scope_kind: definition.key.scope_kind,
            scope_code: definition.key.scope_code.clone(),
            definition_code: definition.key.definition_code.clone(),
            definition_version: definition.key.definition_version.clone(),
            name: definition.name.clone(),
            description: definition.description.clone(),
            definition_status: definition.definition_status,
            allow_inheritance: definition.allow_inheritance,
            lifecycle_status: definition.lifecycle_status,
            validation_status: definition.validation_status,
            draft_revision: definition.draft_revision,
            attributes_schema: include_schema.then(|| definition.attributes_schema.clone()),
            binary_schema: include_schema.then(|| definition.binary_schema.clone()),
            binary_storage_profile: include_schema
                .then(|| definition.binary_storage_profile.clone()),
            published_at: definition.published_at.clone(),
        }
    }

    pub(super) fn resource_record(
        &self,
        resource: &Resource,
        source_kind: ResourceSourceKind,
        mappings: &[ResourceBinaryMapping],
        local_draft: Option<&Resource>,
        include_detail: bool,
    ) -> ResourceRecord {
        ResourceRecord {
            resource_code: resource.key.resource_code.clone(),
            resource_version: include_detail.then(|| resource.key.resource_version.clone()),
            source_kind,
            definition_scope_kind: resource.definition_key.scope_kind,
            definition_scope_code: resource.definition_key.scope_code.clone(),
            definition_code: resource.definition_key.definition_code.clone(),
            definition_version: resource.definition_key.definition_version.clone(),
            name: resource.name.clone(),
            description: resource.description.clone(),
            attributes: include_detail.then(|| resource.attributes.clone()),
            allow_inheritance: resource.allow_inheritance,
            lifecycle_status: include_detail.then_some(resource.lifecycle_status),
            validation_status: include_detail.then_some(resource.validation_status),
            draft_revision: (include_detail
                && resource.lifecycle_status == ResourceLifecycleStatus::Draft)
                .then_some(resource.draft_revision),
            binaries: include_detail.then(|| binary_summaries(resource, mappings)),
            local_draft: local_draft.map(|draft| ResourceDraftSummary {
                resource_version: draft.key.resource_version.clone(),
                draft_revision: draft.draft_revision,
                validation_status: draft.validation_status,
                name: include_detail.then(|| draft.name.clone()),
                description: include_detail.then(|| draft.description.clone()),
                definition_scope_kind: include_detail.then_some(draft.definition_key.scope_kind),
                definition_scope_code: include_detail
                    .then(|| draft.definition_key.scope_code.clone()),
                definition_code: include_detail
                    .then(|| draft.definition_key.definition_code.clone()),
                definition_version: include_detail
                    .then(|| draft.definition_key.definition_version.clone()),
                attributes: include_detail.then(|| draft.attributes.clone()),
                allow_inheritance: include_detail.then_some(draft.allow_inheritance),
                lifecycle_status: include_detail.then_some(draft.lifecycle_status),
                binaries: include_detail.then(|| binary_summaries(draft, mappings)),
            }),
        }
    }
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
