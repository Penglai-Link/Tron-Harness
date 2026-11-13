mod authorization;
mod binaries;
mod projection;
mod resources;
mod session_attachments;
mod storage;
mod system;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    application::workspace::WorkspaceService, config::ResourceConfig,
    domain::resource::ResourceError, shared::event_bus::EventBusHandle,
};

use super::{ObjectStorage, ResourceRepository};

pub use storage::StorageRouter;

/// The platform settings resource scope has no workspace record or upstream
/// workspace inheritance.
const PLATFORM_RESOURCE_SCOPE: &str = "PLATFORM";

/// System-managed resources are persisted separately from workspace resources
/// and are intentionally excluded from the public resource projections.
pub const SYSTEM_RESOURCE_SCOPE: &str = "SYSTEM";
pub const SYSTEM_AVATAR_REFERENCE: &str = "system://avatar";

pub trait ResourcePrivacyPolicy: Send + Sync {
    fn can_read(&self, user: &str, workspace: &str, resource: &str) -> bool;
    fn is_protected(&self, workspace: &str, resource: &str) -> bool;
    fn can_read_agent(
        &self,
        user: &str,
        workspace: &str,
        resource: &str,
        agent: Option<&str>,
    ) -> bool {
        let _ = agent;
        !self.is_protected(workspace, resource) && self.can_read(user, workspace, resource)
    }
    fn can_read_agent_in_workspace(
        &self,
        user: &str,
        storage_scope: &str,
        resource: &str,
        agent: Option<&str>,
        run_workspace: Option<&str>,
    ) -> bool {
        if self.is_protected(storage_scope, resource)
            && run_workspace.is_some_and(|workspace| workspace != storage_scope)
        {
            return false;
        }
        self.can_read_agent(user, storage_scope, resource, agent)
    }
}
#[derive(Clone)]
pub struct ResourceService {
    repository: Arc<Mutex<Box<dyn ResourceRepository>>>,
    workspaces: WorkspaceService,
    config: ResourceConfig,
    runtime_config: Option<crate::application::system_config::BackendRuntimeConfig>,
    storage: Arc<StorageRouter>,
    event_bus: EventBusHandle,
    profile_scoped: bool,
    privacy: Option<Arc<dyn ResourcePrivacyPolicy>>,
    agent_scope: Option<Option<String>>,
    agent_workspace: Option<String>,
}

impl ResourceService {
    pub fn new(
        repository: impl ResourceRepository + 'static,
        workspaces: WorkspaceService,
        config: ResourceConfig,
        storage_profiles: std::collections::BTreeMap<String, Arc<dyn ObjectStorage>>,
    ) -> Result<Self, ResourceError> {
        let storage = StorageRouter::new(&config.storage, storage_profiles)?;
        Ok(Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            workspaces,
            config,
            runtime_config: None,
            storage: Arc::new(storage),
            event_bus: EventBusHandle::disabled(),
            profile_scoped: false,
            privacy: None,
            agent_scope: None,
            agent_workspace: None,
        })
    }

    pub fn with_event_bus(
        repository: impl ResourceRepository + 'static,
        workspaces: WorkspaceService,
        config: ResourceConfig,
        storage_profiles: std::collections::BTreeMap<String, Arc<dyn ObjectStorage>>,
        event_bus: EventBusHandle,
    ) -> Result<Self, ResourceError> {
        let mut service = Self::new(repository, workspaces, config, storage_profiles)?;
        service.event_bus = event_bus;
        Ok(service)
    }

    pub fn with_runtime_config(
        mut self,
        runtime: crate::application::system_config::BackendRuntimeConfig,
    ) -> Self {
        self.runtime_config = Some(runtime);
        self
    }

    fn effective_config(&self) -> ResourceConfig {
        self.runtime_config
            .as_ref()
            .map(|runtime| runtime.snapshot().resource)
            .unwrap_or_else(|| self.config.clone())
    }

    pub fn with_profile_scope(mut self) -> Self {
        self.profile_scoped = true;
        self
    }

    /// Tool-facing resources are fail-closed for protected KB files when Agent identity is absent.
    pub fn for_agent(&self, agent: Option<&str>) -> Self {
        let mut service = self.clone();
        service.agent_scope = Some(agent.map(str::to_owned));
        service
    }
    pub fn for_agent_in_workspace(&self, agent: Option<&str>, workspace: &str) -> Self {
        let mut service = self.for_agent(agent);
        service.agent_workspace = Some(workspace.into());
        service
    }
    pub fn with_privacy_policy(mut self, policy: Arc<dyn ResourcePrivacyPolicy>) -> Self {
        self.privacy = Some(policy);
        self
    }
    pub(super) fn resource_read_allowed(
        &self,
        user: &str,
        workspace: &str,
        resource: &str,
    ) -> bool {
        self.privacy
            .as_ref()
            .is_none_or(|p| match &self.agent_scope {
                Some(agent) => p.can_read_agent_in_workspace(
                    user,
                    workspace,
                    resource,
                    agent.as_deref(),
                    self.agent_workspace.as_deref(),
                ),
                None => p.can_read(user, workspace, resource),
            })
    }
    pub(super) fn authorize_resource_read(
        &self,
        user: &str,
        workspace: &str,
        resource: &str,
    ) -> Result<(), ResourceError> {
        if self.resource_read_allowed(user, workspace, resource) {
            Ok(())
        } else {
            Err(ResourceError::NotFound)
        }
    }
    pub(super) fn authorize_resource_mutation(
        &self,
        user: &str,
        workspace: &str,
        resource: &str,
    ) -> Result<(), ResourceError> {
        self.authorize_resource_read(user, workspace, resource)?;
        // KB originals are immutable while referenced; product removal preserves the original.
        if self
            .privacy
            .as_ref()
            .is_some_and(|p| p.is_protected(workspace, resource))
        {
            Err(ResourceError::Conflict)
        } else {
            Ok(())
        }
    }
    /// Internal cleanup is authorized and reference-checked by the owning product transaction.
    pub(crate) fn remove_managed_binary_object(&self, locator: &str) -> Result<(), ResourceError> {
        self.storage.delete(locator)
    }

    pub fn storage_roots(&self) -> Vec<PathBuf> {
        self.storage.storage_roots()
    }

    pub fn max_binary_bytes(&self) -> u64 {
        self.effective_config().limits.max_binary_bytes
    }

    pub fn max_commit_bytes(&self) -> u64 {
        self.effective_config().limits.max_commit_bytes
    }

    pub fn max_commit_control_bytes(&self) -> usize {
        self.effective_config().limits.attributes_max_bytes
    }

    fn repository<T>(
        &self,
        operation: impl FnOnce(&mut dyn ResourceRepository) -> Result<T, ResourceError>,
    ) -> Result<T, ResourceError> {
        let mut repository = self
            .repository
            .lock()
            .expect("resource repository mutex poisoned");
        operation(&mut **repository)
    }
}
