use super::*;
use crate::{application::workspace::WorkspaceSessionPurger, domain::workspace::WorkspaceError};

impl ChatService {
    /// Permanently delete every conversation linked to the workspaces. The
    /// preflight pass retires admission for all sessions first, so one running
    /// conversation aborts before any history or attachment is removed.
    pub fn purge_workspace_sessions(&self, workspace_codes: &[String]) -> Result<usize, ChatError> {
        let session_codes = self
            .repository()
            .session_codes_by_workspaces(workspace_codes)?;
        let mut removable = Vec::with_capacity(session_codes.len());
        for session_code in session_codes {
            match self.prepare_session_deletion(&session_code) {
                Ok(()) => removable.push(session_code),
                // A workspace link without a Framework session is an orphan;
                // the workspace transaction removes the link itself.
                Err(ChatError::SessionNotFound) => {}
                Err(error) => {
                    for prepared_code in &removable {
                        let _ = self.active_runs.reactivate_session(prepared_code);
                    }
                    return Err(error);
                }
            }
        }
        for session_code in &removable {
            self.delete_session(session_code)?;
        }
        Ok(removable.len())
    }
}

impl WorkspaceSessionPurger for ChatService {
    fn purge_workspace_sessions(
        &self,
        workspace_codes: &[String],
    ) -> Result<usize, WorkspaceError> {
        ChatService::purge_workspace_sessions(self, workspace_codes).map_err(|error| match error {
            ChatError::SessionRunActive => WorkspaceError::WorkspaceSessionsBusy,
            _ => WorkspaceError::SessionCleanupFailed,
        })
    }
}
