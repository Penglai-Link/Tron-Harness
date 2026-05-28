use super::*;

impl WorkspaceService {
    pub fn list_workspaces(
        &self,
        query: WorkspaceListQuery,
    ) -> Result<Vec<WorkspaceTreeNode>, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let direct_counts = self.direct_user_counts(&workspaces)?;
        let role_counts = self.role_grant_counts(&workspaces)?;
        let effective_counts = self.effective_user_counts(&workspaces)?;
        Ok(build_workspace_tree(
            &workspaces,
            &direct_counts,
            &role_counts,
            &effective_counts,
            query,
        ))
    }

    pub fn list_workspace_options(
        &self,
        query: WorkspaceListQuery,
    ) -> Result<Vec<WorkspaceOption>, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let direct_counts = self.direct_user_counts(&workspaces)?;
        let role_counts = self.role_grant_counts(&workspaces)?;
        let effective_counts = self.effective_user_counts(&workspaces)?;
        let tree = build_workspace_tree(
            &workspaces,
            &direct_counts,
            &role_counts,
            &effective_counts,
            query,
        );

        Ok(build_workspace_options(&tree))
    }

    pub fn workspace_switcher(&self) -> Result<WorkspaceSwitcherData, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let direct_counts = self.direct_user_counts(&workspaces)?;
        let role_counts = self.role_grant_counts(&workspaces)?;
        let effective_counts = self.effective_user_counts(&workspaces)?;
        let tree = build_workspace_tree(
            &workspaces,
            &direct_counts,
            &role_counts,
            &effective_counts,
            WorkspaceListQuery {
                include_archived: None,
                search: None,
                status: Some(WorkspaceStatus::Active),
                root_only: None,
            },
        );

        Ok(build_workspace_switcher(&tree))
    }

    pub fn statistics(&self) -> Result<WorkspaceStatistics, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let mut direct_user_grants = 0;
        let mut role_grants = 0;

        for workspace in &workspaces {
            direct_user_grants += self.list_raw_user_access(&workspace.workspace_code)?.len();
            role_grants += self.list_raw_role_access(&workspace.workspace_code)?.len();
        }

        Ok(WorkspaceStatistics {
            total_workspaces: workspaces.len(),
            active_workspaces: workspaces
                .iter()
                .filter(|workspace| workspace.status == WorkspaceStatus::Active)
                .count(),
            direct_user_grants,
            role_grants,
        })
    }

    pub fn get_workspace(&self, workspace_code: &str) -> Result<WorkspaceDetail, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let workspace = find_workspace(&workspaces, workspace_code)?.clone();
        let direct_counts = self.direct_user_counts(&workspaces)?;
        let role_counts = self.role_grant_counts(&workspaces)?;
        let effective_counts = self.effective_user_counts(&workspaces)?;
        let path = build_workspace_path(&workspaces, &workspace.workspace_code)?;
        let depth = path.len();
        let children = build_child_summaries(
            &workspaces,
            &workspace.workspace_code,
            depth,
            &direct_counts,
            &role_counts,
            &effective_counts,
        );

        Ok(WorkspaceDetail {
            id: workspace.id.as_decimal_string(),
            workspace_code: workspace.workspace_code.clone(),
            name: workspace.name.clone(),
            description: workspace.description.clone(),
            parent_workspace_code: workspace.parent_workspace_code.clone(),
            depth,
            status: workspace.status,
            path,
            children,
            direct_user_count: *direct_counts.get(&workspace.workspace_code).unwrap_or(&0),
            role_grant_count: *role_counts.get(&workspace.workspace_code).unwrap_or(&0),
            effective_user_count: *effective_counts
                .get(&workspace.workspace_code)
                .unwrap_or(&0),
            created_at: workspace.created_at.clone(),
            updated_at: workspace.updated_at.clone(),
            archived_at: workspace.archived_at.clone(),
        })
    }
}
