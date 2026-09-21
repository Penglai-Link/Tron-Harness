use crate::domain::{resource::ResourceError, workspace::WorkspaceStatus};

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
        let _ = (actor_user_code, permission_code);
        if workspace_code == PLATFORM_RESOURCE_SCOPE {
            return Ok(());
        }
        self.active_workspace(workspace_code)
    }
}
