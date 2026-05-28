use super::*;

impl WorkspaceService {
    pub fn create_workspace(
        &self,
        actor: &AuthenticatedUser,
        command: CreateWorkspaceCommand,
    ) -> Result<WorkspaceDetail, WorkspaceError> {
        let mut workspaces = self.all_workspaces()?;
        if command.name.trim().is_empty() {
            return Err(WorkspaceError::WorkspaceNameRequired);
        }

        let parent_workspace_code = command
            .parent_workspace_code
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if let Some(parent_code) = parent_workspace_code.as_deref() {
            let parent = find_workspace(&workspaces, parent_code)?;
            if parent.status == WorkspaceStatus::Archived {
                return Err(WorkspaceError::ArchivedMutationDenied);
            }
            let parent_depth = build_workspace_path(&workspaces, parent_code)?.len();
            if parent_depth >= 3 {
                return Err(WorkspaceError::InvalidHierarchy);
            }
        }
        let workspace_code = generate_workspace_code(&workspaces)?;

        let id = {
            let mut repository = self
                .repository
                .lock()
                .expect("workspace repository mutex poisoned");
            repository.next_id()?
        };
        let workspace = Workspace {
            id: SnowflakeId::new(id),
            workspace_code: workspace_code.clone(),
            name: command.name.trim().to_string(),
            description: optional_trimmed(command.description),
            parent_workspace_code,
            status: WorkspaceStatus::Active,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            archived_at: None,
        };
        {
            let mut repository = self
                .repository
                .lock()
                .expect("workspace repository mutex poisoned");
            repository.create_workspace(&workspace)?;
        }
        workspaces.push(workspace);
        let detail = self.get_workspace(&workspace_code)?;
        self.publish_workspace_detail_event(
            "workspace.created",
            "Created workspace",
            actor,
            &detail,
        );
        Ok(detail)
    }

    pub fn update_workspace(
        &self,
        actor: &AuthenticatedUser,
        workspace_code: &str,
        command: UpdateWorkspaceCommand,
    ) -> Result<WorkspaceDetail, WorkspaceError> {
        let mut workspace = self.get_workspace_record(workspace_code)?;
        if workspace.status == WorkspaceStatus::Archived {
            return Err(WorkspaceError::ArchivedMutationDenied);
        }
        if command.name.trim().is_empty() {
            return Err(WorkspaceError::WorkspaceNameRequired);
        }
        workspace.name = command.name.trim().to_string();
        workspace.description = optional_trimmed(command.description);
        workspace.updated_at = current_timestamp();
        {
            let mut repository = self
                .repository
                .lock()
                .expect("workspace repository mutex poisoned");
            repository.update_workspace(&workspace)?;
        }
        let detail = self.get_workspace(workspace_code)?;
        self.publish_workspace_detail_event(
            "workspace.updated",
            "Updated workspace",
            actor,
            &detail,
        );
        Ok(detail)
    }

    pub fn move_workspace(
        &self,
        actor: &AuthenticatedUser,
        workspace_code: &str,
        command: MoveWorkspaceCommand,
    ) -> Result<WorkspaceDetail, WorkspaceError> {
        let mut workspaces = self.all_workspaces()?;
        let target_index = workspaces
            .iter()
            .position(|workspace| workspace.workspace_code == workspace_code)
            .ok_or(WorkspaceError::NotFound)?;
        if workspaces[target_index].status == WorkspaceStatus::Archived {
            return Err(WorkspaceError::ArchivedMutationDenied);
        }

        let new_parent = command
            .parent_workspace_code
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if new_parent.as_deref() == Some(workspace_code) {
            return Err(WorkspaceError::InvalidHierarchy);
        }

        let subtree_codes = descendant_codes(&workspaces, workspace_code);
        if let Some(parent_code) = new_parent.as_deref() {
            if subtree_codes.contains(parent_code) {
                return Err(WorkspaceError::InvalidHierarchy);
            }
            let parent = find_workspace(&workspaces, parent_code)?;
            if parent.status == WorkspaceStatus::Archived {
                return Err(WorkspaceError::ArchivedMutationDenied);
            }
            let parent_depth = build_workspace_path(&workspaces, parent_code)?.len();
            let subtree_height = subtree_height(&workspaces, workspace_code, 1);
            if parent_depth + subtree_height > 3 {
                return Err(WorkspaceError::InvalidHierarchy);
            }
        } else if subtree_height(&workspaces, workspace_code, 1) > 3 {
            return Err(WorkspaceError::InvalidHierarchy);
        }

        workspaces[target_index].parent_workspace_code = new_parent;
        workspaces[target_index].updated_at = current_timestamp();
        {
            let mut repository = self
                .repository
                .lock()
                .expect("workspace repository mutex poisoned");
            repository.update_workspace(&workspaces[target_index])?;
        }
        let detail = self.get_workspace(workspace_code)?;
        self.publish_workspace_detail_event("workspace.moved", "Moved workspace", actor, &detail);
        Ok(detail)
    }

    pub fn archive_workspace(
        &self,
        actor: &AuthenticatedUser,
        workspace_code: &str,
    ) -> Result<WorkspaceLifecycleResult, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let workspace = find_workspace(&workspaces, workspace_code)?;
        let subtree_codes = descendant_codes(&workspaces, workspace_code)
            .into_iter()
            .collect::<Vec<_>>();
        let affected_workspace_count = {
            let mut repository = self
                .repository
                .lock()
                .expect("workspace repository mutex poisoned");
            repository.archive_subtree(&subtree_codes, &current_timestamp())?
        };
        Ok(WorkspaceLifecycleResult {
            workspace_code: workspace.workspace_code.clone(),
            status: WorkspaceStatus::Archived,
            archived_at: Some(current_timestamp()),
            affected_workspace_count,
        })
        .map(|result| {
            self.publish_workspace_lifecycle_event(
                "workspace.archived",
                "Archived workspace subtree",
                actor,
                &result,
            );
            result
        })
    }

    pub fn restore_workspace(
        &self,
        actor: &AuthenticatedUser,
        workspace_code: &str,
    ) -> Result<WorkspaceLifecycleResult, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let workspace = find_workspace(&workspaces, workspace_code)?;
        if let Some(parent_code) = workspace.parent_workspace_code.as_deref() {
            let parent = find_workspace(&workspaces, parent_code)?;
            if parent.status == WorkspaceStatus::Archived {
                return Err(WorkspaceError::ArchivedMutationDenied);
            }
        }
        let subtree_codes = descendant_codes(&workspaces, workspace_code)
            .into_iter()
            .collect::<Vec<_>>();
        let affected_workspace_count = {
            let mut repository = self
                .repository
                .lock()
                .expect("workspace repository mutex poisoned");
            repository.restore_subtree(&subtree_codes, &current_timestamp())?
        };
        Ok(WorkspaceLifecycleResult {
            workspace_code: workspace.workspace_code.clone(),
            status: WorkspaceStatus::Active,
            archived_at: None,
            affected_workspace_count,
        })
        .map(|result| {
            self.publish_workspace_lifecycle_event(
                "workspace.restored",
                "Restored workspace subtree",
                actor,
                &result,
            );
            result
        })
    }
}
