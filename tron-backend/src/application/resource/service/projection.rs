use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{
    resource::{ManagedResource, ResourceError, ResourceSourceKind},
    workspace::WorkspaceStatus,
};

use super::{PLATFORM_RESOURCE_SCOPE, ResourceService};

#[derive(Debug, Clone)]
pub(super) struct EffectiveResource {
    pub resource: ManagedResource,
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
        if workspace_code == PLATFORM_RESOURCE_SCOPE {
            return Ok(PLATFORM_RESOURCE_SCOPE.to_string());
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

    fn all_managed_resources(&self) -> Result<Vec<ManagedResource>, ResourceError> {
        let mut resources = Vec::new();
        resources.extend(
            self.repository(|repository| repository.list_files())?
                .into_iter()
                .map(ManagedResource::File),
        );
        resources.extend(
            self.repository(|repository| repository.list_data_source_connections())?
                .into_iter()
                .map(ManagedResource::DataSourceConnection),
        );
        resources.extend(
            self.repository(|repository| repository.list_servers())?
                .into_iter()
                .map(ManagedResource::Server),
        );
        resources.extend(
            self.repository(|repository| repository.list_server_accounts())?
                .into_iter()
                .map(ManagedResource::ServerAccount),
        );
        Ok(resources)
    }

    pub(super) fn effective_resources(
        &self,
        workspace_code: &str,
    ) -> Result<BTreeMap<String, EffectiveResource>, ResourceError> {
        let workspace_code = self.active_workspace_code(workspace_code)?;
        let resources = self.all_managed_resources()?;
        let mut effective = BTreeMap::new();

        if workspace_code == PLATFORM_RESOURCE_SCOPE {
            let mut resource_codes = BTreeSet::new();
            for resource in resources
                .iter()
                .filter(|resource| resource.workspace_code() == PLATFORM_RESOURCE_SCOPE)
            {
                if !resource_codes.insert(resource.resource_code()) {
                    return Err(ResourceError::Conflict);
                }
                effective.insert(
                    resource.resource_code().to_string(),
                    EffectiveResource {
                        resource: resource.clone(),
                        source_kind: ResourceSourceKind::Local,
                    },
                );
            }
            return Ok(effective);
        }

        let workspace = self
            .workspaces
            .get_workspace(&workspace_code)
            .map_err(|_| ResourceError::InvalidWorkspace)?;

        // A fresh installation has no PLATFORM workspace row, but platform
        // resources still form the inherited base for every workspace. Legacy
        // installations include PLATFORM in the path and use the normal loop.
        if !workspace
            .path
            .iter()
            .any(|lineage| lineage.workspace_code == PLATFORM_RESOURCE_SCOPE)
        {
            let mut resource_codes = BTreeSet::new();
            for resource in resources
                .iter()
                .filter(|resource| resource.workspace_code() == PLATFORM_RESOURCE_SCOPE)
            {
                if !resource.allow_inheritance() {
                    continue;
                }
                if !resource_codes.insert(resource.resource_code()) {
                    return Err(ResourceError::Conflict);
                }
                effective.insert(
                    resource.resource_code().to_string(),
                    EffectiveResource {
                        resource: resource.clone(),
                        source_kind: ResourceSourceKind::Inherited,
                    },
                );
            }
        }

        for lineage in &workspace.path {
            let mut resource_codes = BTreeSet::new();
            for resource in resources
                .iter()
                .filter(|resource| resource.workspace_code() == lineage.workspace_code)
            {
                if lineage.workspace_code != workspace_code && !resource.allow_inheritance() {
                    continue;
                }
                if !resource_codes.insert(resource.resource_code()) {
                    return Err(ResourceError::Conflict);
                }
                let source_kind = if lineage.workspace_code == workspace_code {
                    if effective.contains_key(resource.resource_code()) {
                        ResourceSourceKind::Override
                    } else {
                        ResourceSourceKind::Local
                    }
                } else {
                    ResourceSourceKind::Inherited
                };
                effective.insert(
                    resource.resource_code().to_string(),
                    EffectiveResource {
                        resource: resource.clone(),
                        source_kind,
                    },
                );
            }
        }
        Ok(effective)
    }

    pub(super) fn managed_resource(
        &self,
        workspace_code: &str,
        resource_code: &str,
    ) -> Result<Option<ManagedResource>, ResourceError> {
        let resources = self.all_managed_resources()?;
        let mut matches = resources.into_iter().filter(|resource| {
            resource.workspace_code() == workspace_code && resource.resource_code() == resource_code
        });
        let resource = matches.next();
        if matches.next().is_some() {
            return Err(ResourceError::Conflict);
        }
        Ok(resource)
    }

    pub(super) fn ensure_resource_create_is_available(
        &self,
        workspace_code: &str,
        resource_code: &str,
    ) -> Result<(), ResourceError> {
        if self
            .managed_resource(workspace_code, resource_code)?
            .is_some()
        {
            return Err(ResourceError::Conflict);
        }
        Ok(())
    }
}
