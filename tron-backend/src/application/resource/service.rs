mod authorization;
mod binaries;
mod connection_attributes;
mod connections;
mod projection;
mod resources;
mod storage;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    application::{
        role::RoleService, user::UserService, variable::VariableService,
        workspace::WorkspaceService,
    },
    config::ResourceConfig,
    domain::resource::ResourceError,
    shared::event_bus::EventBusHandle,
};

use super::{ObjectStorage, ResourceConnectionVerifier, ResourceRepository};

pub use storage::StorageRouter;

/// The platform settings resource scope has no workspace record or upstream
/// workspace inheritance.
const PLATFORM_RESOURCE_SCOPE: &str = "PLATFORM";

#[derive(Clone)]
pub struct ResourceService {
    repository: Arc<Mutex<Box<dyn ResourceRepository>>>,
    workspaces: WorkspaceService,
    roles: RoleService,
    users: UserService,
    variables: VariableService,
    config: ResourceConfig,
    storage: Arc<StorageRouter>,
    event_bus: EventBusHandle,
    connection_verifier: Arc<dyn ResourceConnectionVerifier>,
}

impl ResourceService {
    pub fn new(
        repository: impl ResourceRepository + 'static,
        workspaces: WorkspaceService,
        roles: RoleService,
        users: UserService,
        variables: VariableService,
        config: ResourceConfig,
        storage_profiles: std::collections::BTreeMap<String, Arc<dyn ObjectStorage>>,
        connection_verifier: Arc<dyn ResourceConnectionVerifier>,
    ) -> Result<Self, ResourceError> {
        let storage = StorageRouter::new(&config.storage, storage_profiles)?;
        Ok(Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            workspaces,
            roles,
            users,
            variables,
            config,
            storage: Arc::new(storage),
            event_bus: EventBusHandle::disabled(),
            connection_verifier,
        })
    }

    pub fn with_event_bus(
        repository: impl ResourceRepository + 'static,
        workspaces: WorkspaceService,
        roles: RoleService,
        users: UserService,
        variables: VariableService,
        config: ResourceConfig,
        storage_profiles: std::collections::BTreeMap<String, Arc<dyn ObjectStorage>>,
        event_bus: EventBusHandle,
        connection_verifier: Arc<dyn ResourceConnectionVerifier>,
    ) -> Result<Self, ResourceError> {
        let mut service = Self::new(
            repository,
            workspaces,
            roles,
            users,
            variables,
            config,
            storage_profiles,
            connection_verifier,
        )?;
        service.event_bus = event_bus;
        Ok(service)
    }

    pub fn with_connection_verifier(
        mut self,
        connection_verifier: Arc<dyn ResourceConnectionVerifier>,
    ) -> Self {
        self.connection_verifier = connection_verifier;
        self
    }

    pub fn storage_roots(&self) -> Vec<PathBuf> {
        self.storage.storage_roots()
    }

    pub fn max_binary_bytes(&self) -> u64 {
        self.config.limits.max_binary_bytes
    }

    pub fn max_commit_bytes(&self) -> u64 {
        self.config.limits.max_commit_bytes
    }

    pub fn max_commit_control_bytes(&self) -> usize {
        self.config.limits.attributes_max_bytes
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
