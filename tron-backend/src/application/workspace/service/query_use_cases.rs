use super::*;

impl WorkspaceService {
    pub fn list_workspaces(
        &self,
        query: WorkspaceListQuery,
    ) -> Result<Vec<WorkspaceTreeNode>, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        Ok(build_workspace_tree(&workspaces, query))
    }

    pub fn list_workspace_options(
        &self,
        query: WorkspaceListQuery,
    ) -> Result<Vec<WorkspaceOption>, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let tree = build_workspace_tree(&workspaces, query);

        Ok(build_workspace_options(&tree))
    }

    pub fn workspace_switcher(&self) -> Result<WorkspaceSwitcherData, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let tree = build_workspace_tree(&workspaces, WorkspaceListQuery {
                include_archived: None,
                search: None,
                status: Some(WorkspaceStatus::Active),
                root_only: None,
            },
        );

        Ok(build_workspace_switcher(&tree))
    }

}
