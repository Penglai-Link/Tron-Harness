use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use crate::{
    domain::auth::AuthenticatedUser,
    domain::role::{
        DEFAULT_ROLE_UPDATED_AT, Role, RoleError, RoleKind, RoleManagedBy, fixed_system_role_codes,
        normalize_permissions, permission_domains, validate_role_catalog,
    },
    shared::{
        event_bus::{BusEventDraft, EventBusHandle},
        id::SnowflakeId,
    },
};

#[derive(Debug, Clone)]
pub struct ReplaceRolesCommand {
    pub roles: Vec<RoleDraft>,
}

#[derive(Debug, Clone)]
pub struct RoleDraft {
    pub id: Option<String>,
    pub code: String,
    pub name: String,
    pub description: String,
    pub kind: RoleKind,
    pub managed_by: RoleManagedBy,
    pub permissions: BTreeMap<String, Vec<String>>,
    pub updated_at: Option<String>,
}

pub trait RoleRepository: Send {
    fn list_roles(&mut self) -> Result<Vec<Role>, RoleError>;
    fn next_id(&mut self) -> Result<i64, RoleError>;
    fn replace_roles(&mut self, roles: &[Role]) -> Result<Vec<Role>, RoleError>;
}

#[derive(Clone)]
pub struct RoleService {
    repository: Arc<Mutex<Box<dyn RoleRepository>>>,
    event_bus: EventBusHandle,
}

impl RoleService {
    pub fn new(repository: impl RoleRepository + 'static) -> Self {
        Self::with_event_bus(repository, EventBusHandle::disabled())
    }

    pub fn with_event_bus(
        repository: impl RoleRepository + 'static,
        event_bus: EventBusHandle,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            event_bus,
        }
    }

    pub fn list_permission_domains(&self) -> Vec<String> {
        permission_domains()
            .iter()
            .map(|value| (*value).to_string())
            .collect()
    }

    pub fn list_roles(&self) -> Result<Vec<Role>, RoleError> {
        let mut repository = self
            .repository
            .lock()
            .expect("role repository mutex poisoned");
        repository.list_roles()
    }

    pub fn replace_roles(
        &self,
        actor: &AuthenticatedUser,
        command: ReplaceRolesCommand,
    ) -> Result<Vec<Role>, RoleError> {
        let mut repository = self
            .repository
            .lock()
            .expect("role repository mutex poisoned");
        let roles = command
            .roles
            .into_iter()
            .map(|draft| build_role(&mut **repository, draft))
            .collect::<Result<Vec<_>, _>>()?;

        validate_role_catalog(&roles)?;
        let replaced = repository.replace_roles(&roles)?;
        drop(repository);

        let _ = self.event_bus.publish(
            BusEventDraft::audit("role.catalog.replaced", "role_service")
                .subject_type_only("role_catalog")
                .actor(actor)
                .summary("Replaced role catalog")
                .attr("roleCount", replaced.len().to_string()),
        );

        Ok(replaced)
    }
}

fn build_role(repository: &mut dyn RoleRepository, draft: RoleDraft) -> Result<Role, RoleError> {
    let id = if let Some(raw_id) = draft.id.as_deref() {
        SnowflakeId::from_decimal_string(raw_id)
            .map(|value| value.value())
            .ok()
            .filter(|value| *value > 0)
            .unwrap_or(repository.next_id()?)
    } else {
        repository.next_id()?
    };

    let normalized_permissions = normalize_permissions(&draft.permissions)?;
    let code = draft.code.trim().to_string();
    let kind = if fixed_system_role_codes().contains(&code.as_str()) {
        RoleKind::System
    } else {
        draft.kind
    };

    Ok(Role {
        id: SnowflakeId::new(id),
        code,
        name: draft.name.trim().to_string(),
        description: draft.description.trim().to_string(),
        kind,
        managed_by: draft.managed_by,
        permissions: normalized_permissions,
        updated_at: draft
            .updated_at
            .unwrap_or_else(|| DEFAULT_ROLE_UPDATED_AT.to_string()),
    })
}
