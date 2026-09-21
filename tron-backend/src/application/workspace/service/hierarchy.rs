use std::collections::BTreeSet;

use super::*;

pub(super) fn find_workspace<'a>(
    workspaces: &'a [Workspace],
    workspace_code: &str,
) -> Result<&'a Workspace, WorkspaceError> {
    workspaces
        .iter()
        .find(|workspace| workspace.workspace_code == workspace_code)
        .ok_or(WorkspaceError::NotFound)
}

pub(super) fn build_workspace_path(
    workspaces: &[Workspace],
    workspace_code: &str,
) -> Result<Vec<WorkspaceLineageItem>, WorkspaceError> {
    let mut path = Vec::new();
    let mut current = Some(workspace_code.to_string());
    while let Some(code) = current {
        let workspace = find_workspace(workspaces, &code)?;
        path.push(WorkspaceLineageItem {
            workspace_code: workspace.workspace_code.clone(),
            name: workspace.name.clone(),
            depth: 0,
        });
        current = workspace.parent_workspace_code.clone();
    }
    path.reverse();
    for (index, item) in path.iter_mut().enumerate() {
        item.depth = index + 1;
    }
    Ok(path)
}

pub(super) fn generate_workspace_code(workspaces: &[Workspace]) -> Result<String, WorkspaceError> {
    let existing_codes = workspaces
        .iter()
        .map(|workspace| workspace.workspace_code.as_str())
        .collect::<BTreeSet<_>>();
    let mut rng = rand::rng();

    for _ in 0..WORKSPACE_CODE_GENERATION_ATTEMPTS {
        let candidate = format!("{GENERATED_WORKSPACE_CODE_PREFIX}{:016X}", rng.next_u64());
        if !existing_codes.contains(candidate.as_str()) {
            return Ok(candidate);
        }
    }

    Err(WorkspaceError::DuplicateWorkspaceCode)
}

pub(super) fn descendant_codes(workspaces: &[Workspace], workspace_code: &str) -> BTreeSet<String> {
    let mut codes = BTreeSet::new();
    collect_descendants(workspaces, workspace_code, &mut codes);
    codes
}

fn collect_descendants(workspaces: &[Workspace], workspace_code: &str, acc: &mut BTreeSet<String>) {
    if !acc.insert(workspace_code.to_string()) {
        return;
    }
    for child in workspaces
        .iter()
        .filter(|workspace| workspace.parent_workspace_code.as_deref() == Some(workspace_code))
    {
        collect_descendants(workspaces, &child.workspace_code, acc);
    }
}

pub(super) fn subtree_height(
    workspaces: &[Workspace],
    workspace_code: &str,
    depth: usize,
) -> usize {
    let max_child_depth = workspaces
        .iter()
        .filter(|workspace| workspace.parent_workspace_code.as_deref() == Some(workspace_code))
        .map(|workspace| subtree_height(workspaces, &workspace.workspace_code, depth + 1))
        .max()
        .unwrap_or(depth);
    max_child_depth
}
