mod authorization;
mod definitions;
mod projection;
mod resources;
mod seeds;
mod storage;
mod validation;

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

use super::{ObjectStorage, ResourceRepository};

pub use storage::StorageRouter;

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
    ) -> Result<Self, ResourceError> {
        let mut service = Self::new(
            repository,
            workspaces,
            roles,
            users,
            variables,
            config,
            storage_profiles,
        )?;
        service.event_bus = event_bus;
        Ok(service)
    }

    pub fn storage_roots(&self) -> Vec<PathBuf> {
        self.storage.storage_roots()
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
