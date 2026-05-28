use super::*;

pub(super) fn workspace_path_entries(workspace: &WorkspaceDetail) -> Vec<LineageEntry> {
    workspace
        .path
        .iter()
        .map(|item| LineageEntry {
            code: item.workspace_code.clone(),
            name: item.name.clone(),
        })
        .collect()
}

pub(super) fn map_workspace_error(error: WorkspaceError) -> ModelError {
    match error {
        WorkspaceError::StorageFailure => ModelError::StorageFailure,
        WorkspaceError::NotFound => ModelError::InvalidWorkspace,
        _ => ModelError::InvalidWorkspace,
    }
}
