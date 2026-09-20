use crate::domain::{
    resource::ResourceError,
    user::UserStatus,
    workspace::{WorkspaceError, WorkspaceStatus},
};

use super::{PLATFORM_RESOURCE_SCOPE, ResourceService};

impl ResourceService {
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
        workspace_code: &str,
        write: bool,
    ) -> Result<String, ResourceError> {
        let workspace_code = workspace_code.trim().to_string();
        if workspace_code.is_empty() {
            return Err(ResourceError::WorkspaceRequired);
        }
        if write {
            self.authorize_workspace_write(actor_user_code, &workspace_code)?;
        } else {
            self.authorize_workspace_read(actor_user_code, &workspace_code)?;
        }
        Ok(workspace_code)
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
        if self.profile_scoped {
            return self.active_workspace(workspace_code);
        }
        let role_codes = self.active_actor_role_codes(actor_user_code)?;
        if !self
            .roles
            .has_permission(&role_codes, "Resources", permission_code)
            .map_err(|_| ResourceError::StorageFailure)?
        {
            return Err(ResourceError::Forbidden);
        }
        let is_admin = role_codes.iter().any(|role| role == "ADMIN");
        // New installations do not materialize PLATFORM as a workspace. Keep
        // its resources in the system scope for administrators while retaining
        // workspace-grant authorization for installations that still have the
        // legacy PLATFORM workspace.
        if workspace_code == PLATFORM_RESOURCE_SCOPE {
            match self.workspaces.get_workspace(workspace_code) {
                Ok(workspace) if workspace.status == WorkspaceStatus::Active => {}
                Ok(_) => return Err(ResourceError::InvalidWorkspace),
                Err(WorkspaceError::NotFound) => {
                    return is_admin
                        .then_some(())
                        .ok_or(ResourceError::Forbidden);
                }
                Err(_) => return Err(ResourceError::StorageFailure),
            }
        } else {
            self.active_workspace(workspace_code)?;
        }
        if is_admin {
            return Ok(());
        }
        if !self
            .workspaces
            .user_has_effective_access(workspace_code, actor_user_code)
            .map_err(|_| ResourceError::Forbidden)?
        {
            return Err(ResourceError::Forbidden);
        }
        Ok(())
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
