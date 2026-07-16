use crate::domain::{
    resource::{PLATFORM_SCOPE_CODE, ResourceError, ResourceScopeKind},
    user::UserStatus,
    workspace::WorkspaceStatus,
};

use super::ResourceService;

impl ResourceService {
    pub(super) fn authorize_platform_read(
        &self,
        actor_user_code: &str,
    ) -> Result<(), ResourceError> {
        self.authorize_permission(actor_user_code, "R")
    }

    pub(super) fn authorize_platform_admin(
        &self,
        actor_user_code: &str,
    ) -> Result<(), ResourceError> {
        self.authorize_permission(actor_user_code, "A")
    }

    pub(super) fn authorize_workspace_read(
        &self,
        actor_user_code: &str,
        workspace_code: &str,
    ) -> Result<(), ResourceError> {
        self.authorize_workspace(actor_user_code, workspace_code, "R")
    }

    pub(super) fn authorize_workspace_write(
        &self,
        actor_user_code: &str,
        workspace_code: &str,
    ) -> Result<(), ResourceError> {
        self.authorize_workspace(actor_user_code, workspace_code, "W")
    }

    pub(super) fn workspace_scope(
        &self,
        actor_user_code: &str,
        scope_kind: ResourceScopeKind,
        workspace_code: Option<&str>,
        write: bool,
    ) -> Result<String, ResourceError> {
        match scope_kind {
            ResourceScopeKind::Platform => {
                if workspace_code.is_some() {
                    return Err(ResourceError::InvalidScope);
                }
                if write {
                    self.authorize_platform_admin(actor_user_code)?;
                } else {
                    self.authorize_platform_read(actor_user_code)?;
                }
                Ok(PLATFORM_SCOPE_CODE.to_string())
            }
            ResourceScopeKind::Workspace => {
                let workspace_code = workspace_code
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or(ResourceError::WorkspaceRequired)?;
                if write {
                    self.authorize_workspace_write(actor_user_code, workspace_code)?;
                } else {
                    self.authorize_workspace_read(actor_user_code, workspace_code)?;
                }
                Ok(workspace_code.to_string())
            }
        }
    }

    pub(super) fn active_workspace(&self, workspace_code: &str) -> Result<(), ResourceError> {
        let workspace = self
            .workspaces
            .get_workspace(workspace_code)
            .map_err(|_| ResourceError::InvalidWorkspace)?;
        if workspace.status != WorkspaceStatus::Active {
            return Err(ResourceError::InvalidWorkspace);
        }
        Ok(())
    }

    fn authorize_workspace(
        &self,
        actor_user_code: &str,
        workspace_code: &str,
        permission_code: &str,
    ) -> Result<(), ResourceError> {
        let role_codes = self.active_actor_role_codes(actor_user_code)?;
        if !self
            .roles
            .has_permission(&role_codes, "Resources", permission_code)
            .map_err(|_| ResourceError::StorageFailure)?
        {
            return Err(ResourceError::Forbidden);
        }
        if !self
            .workspaces
            .user_has_effective_access(workspace_code, actor_user_code)
            .map_err(|_| ResourceError::Forbidden)?
        {
            return Err(ResourceError::Forbidden);
        }
        self.active_workspace(workspace_code)
    }

    fn authorize_permission(
        &self,
        actor_user_code: &str,
        permission_code: &str,
    ) -> Result<(), ResourceError> {
        let role_codes = self.active_actor_role_codes(actor_user_code)?;
        if self
            .roles
            .has_permission(&role_codes, "Resources", permission_code)
            .map_err(|_| ResourceError::StorageFailure)?
        {
            Ok(())
        } else {
            Err(ResourceError::Forbidden)
        }
    }

    fn active_actor_role_codes(&self, actor_user_code: &str) -> Result<Vec<String>, ResourceError> {
        let actor_user_code = actor_user_code.trim();
        if actor_user_code.is_empty() {
            return Err(ResourceError::Forbidden);
        }
        let user = self
            .users
            .get_user(actor_user_code)
            .map_err(|_| ResourceError::Forbidden)?;
        if user.status != UserStatus::Active {
            return Err(ResourceError::Forbidden);
        }
        Ok(user.role_codes)
    }
}
