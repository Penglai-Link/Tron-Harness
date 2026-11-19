use super::*;

impl WorkspaceService {
    /// Permanently remove an archived workspace and its archived descendants.
    /// Session and stored-resource cleanup runs before the business transaction
    /// so active runs refuse the operation without leaving a half-deleted
    /// workspace behind.
    pub fn delete_workspace(
        &self,
        actor: &AuthenticatedUser,
        workspace_code: &str,
    ) -> Result<WorkspaceDeletionResult, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let workspace = find_workspace(&workspaces, workspace_code)?;
        if workspace.status != WorkspaceStatus::Archived {
            return Err(WorkspaceError::ActiveWorkspaceDeleteDenied);
        }
        let subtree_codes = descendant_codes(&workspaces, workspace_code)
            .into_iter()
            .collect::<Vec<_>>();

        let deleted_session_count = self
            .session_purger
            .get()
            .map(|purger| purger.purge_workspace_sessions(&subtree_codes))
            .transpose()?
            .unwrap_or_default();

        let outcome = {
            let mut repository = self
                .repository
                .lock()
                .expect("workspace repository mutex poisoned");
            repository.delete_subtree(&subtree_codes)?
        };
        if let Some(purger) = self.resource_purger.get() {
            purger.purge_workspace_resources(&outcome.storage_locators);
        }

        let result = WorkspaceDeletionResult {
            workspace_code: workspace.workspace_code.clone(),
            affected_workspace_count: outcome.affected_workspace_count,
            deleted_session_count,
        };
        self.publish_workspace_deletion_event(
            "workspace.deleted",
            "Permanently deleted workspace subtree",
            actor,
            &result,
        );
        Ok(result)
    }
}
