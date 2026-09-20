use super::*;

impl TaskService {
    pub(super) fn ensure_active_workspace(&self, workspace_code: &str) -> Result<(), TaskError> {
        let detail = self
            .workspaces
            .get_workspace(workspace_code)
            .map_err(map_workspace_error)?;
        if detail.status != WorkspaceStatus::Active {
            return Err(TaskError::InvalidWorkspace);
        }
        Ok(())
    }
    pub(super) fn ensure_valid_parent(
        &self,
        parent_task_code: &str,
        workspace_code: &str,
    ) -> Result<(), TaskError> {
        let parent = self.get_task_record(parent_task_code)?;
        if parent.workspace_code != workspace_code || parent.status == TaskStatus::Archived {
            return Err(TaskError::InvalidParent);
        }
        Ok(())
    }

    pub(super) fn validated_assignees(
        &self,
        _workspace_code: &str,
        user_codes: Vec<String>,
    ) -> Result<Vec<TaskUserAssignment>, TaskError> {
        let mut unique = BTreeSet::new();
        user_codes
            .into_iter()
            .filter_map(normalized_required)
            .filter(|user_code| unique.insert(user_code.clone()))
            .map(|user_code| {
                Ok(TaskUserAssignment {
                    user_code,
                    assigned_at: current_timestamp(),
                })
            })
            .collect()
    }

    pub(super) fn validated_app_links(
        &self,
        workspace_code: &str,
        app_links: Vec<TaskCodeLink>,
    ) -> Result<Vec<TaskCodeLink>, TaskError> {
        let allowed_codes = self
            .apps
            .assignable_app_codes(workspace_code)
            .map_err(map_app_error)?;
        for app_link in &app_links {
            if !allowed_codes.contains(&app_link.code) {
                return Err(TaskError::InvalidApp);
            }
        }
        Ok(app_links)
    }
}
