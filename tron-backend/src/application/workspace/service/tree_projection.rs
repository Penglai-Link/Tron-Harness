use std::collections::BTreeSet;

use super::*;

pub(super) fn build_workspace_tree(
    workspaces: &[Workspace],
    query: WorkspaceListQuery,
) -> Vec<WorkspaceTreeNode> {
    let visible_status = visible_workspace_status(query.status, query.include_archived);
    let included_codes = search_included_codes(workspaces, query.search.as_deref(), visible_status);
    let mut roots = workspaces
        .iter()
        .filter(|workspace| workspace.parent_workspace_code.is_none())
        .filter(|workspace| workspace_matches_status(workspace, visible_status))
        .filter(|workspace| {
            included_codes
                .as_ref()
                .is_none_or(|codes| codes.contains(&workspace.workspace_code))
        })
        .map(|workspace| {
            build_workspace_node(
                workspaces,
                workspace,
                1,
                visible_status,
                included_codes.as_ref(),
                query.root_only.unwrap_or(false),
            )
        })
        .collect::<Vec<_>>();
    roots.sort_by(|left, right| left.name.cmp(&right.name));
    roots
}

pub(super) fn build_workspace_options(tree: &[WorkspaceTreeNode]) -> Vec<WorkspaceOption> {
    let mut options = Vec::new();
    for node in tree {
        collect_workspace_options(node, &mut Vec::new(), &mut options);
    }
    options
}

fn collect_workspace_options(
    node: &WorkspaceTreeNode,
    lineage_names: &mut Vec<String>,
    acc: &mut Vec<WorkspaceOption>,
) {
    lineage_names.push(node.name.clone());
    acc.push(WorkspaceOption {
        workspace_code: node.workspace_code.clone(),
        label: lineage_names.join(" / "),
        name: node.name.clone(),
        description: node.description.clone(),
        parent_workspace_code: node.parent_workspace_code.clone(),
        depth: node.depth,
        status: node.status,
    });

    for child in &node.children {
        collect_workspace_options(child, lineage_names, acc);
    }

    lineage_names.pop();
}

pub(super) fn build_workspace_switcher(tree: &[WorkspaceTreeNode]) -> WorkspaceSwitcherData {
    let primary_spaces = tree
        .iter()
        .map(|node| WorkspaceSwitcherPrimarySpace {
            workspace_code: node.workspace_code.clone(),
            name: node.name.clone(),
            description: node.description.clone(),
            status: node.status,
            child_count: node.child_count,
        })
        .collect();

    let mut secondary_spaces = Vec::new();
    for node in tree {
        collect_workspace_switcher_secondary(
            node,
            &node.workspace_code,
            &mut Vec::new(),
            &mut secondary_spaces,
        );
    }

    WorkspaceSwitcherData {
        primary_spaces,
        secondary_spaces,
    }
}

fn collect_workspace_switcher_secondary(
    node: &WorkspaceTreeNode,
    primary_workspace_code: &str,
    lineage_names: &mut Vec<String>,
    acc: &mut Vec<WorkspaceSwitcherSecondarySpace>,
) {
    lineage_names.push(node.name.clone());
    acc.push(WorkspaceSwitcherSecondarySpace {
        workspace_code: node.workspace_code.clone(),
        primary_workspace_code: primary_workspace_code.to_string(),
        parent_workspace_code: node.parent_workspace_code.clone(),
        label: lineage_names.join(" / "),
        name: node.name.clone(),
        description: node.description.clone(),
        status: node.status,
        depth: node.depth,
        child_count: node.child_count,
    });

    for child in &node.children {
        collect_workspace_switcher_secondary(child, primary_workspace_code, lineage_names, acc);
    }

    lineage_names.pop();
}

fn build_workspace_node(
    workspaces: &[Workspace],
    workspace: &Workspace,
    depth: usize,
    visible_status: Option<WorkspaceStatus>,
    included_codes: Option<&BTreeSet<String>>,
    root_only: bool,
) -> WorkspaceTreeNode {
    let mut children = if root_only {
        Vec::new()
    } else {
        workspaces
            .iter()
            .filter(|candidate| {
                candidate.parent_workspace_code.as_deref() == Some(&workspace.workspace_code)
            })
            .filter(|candidate| workspace_matches_status(candidate, visible_status))
            .filter(|candidate| {
                included_codes.is_none_or(|codes| codes.contains(&candidate.workspace_code))
            })
            .map(|candidate| {
                build_workspace_node(
                    workspaces,
                    candidate,
                    depth + 1,
                    visible_status,
                    included_codes,
                    false,
                )
            })
            .collect::<Vec<_>>()
    };
    children.sort_by(|left, right| left.name.cmp(&right.name));
    WorkspaceTreeNode {
        id: workspace.id.as_decimal_string(),
        workspace_code: workspace.workspace_code.clone(),
        name: workspace.name.clone(),
        description: workspace.description.clone(),
        parent_workspace_code: workspace.parent_workspace_code.clone(),
        depth,
        status: workspace.status,
        child_count: children.len(),
        children,
    }
}

pub(super) fn build_child_summaries(
    workspaces: &[Workspace],
    workspace_code: &str,
    parent_depth: usize,
) -> Vec<WorkspaceTreeNode> {
    let mut children = workspaces
        .iter()
        .filter(|workspace| workspace.parent_workspace_code.as_deref() == Some(workspace_code))
        .map(|workspace| WorkspaceTreeNode {
            id: workspace.id.as_decimal_string(),
            workspace_code: workspace.workspace_code.clone(),
            name: workspace.name.clone(),
            description: workspace.description.clone(),
            parent_workspace_code: workspace.parent_workspace_code.clone(),
            depth: parent_depth + 1,
            status: workspace.status,
            child_count: workspaces
                .iter()
                .filter(|candidate| {
                    candidate.parent_workspace_code.as_deref() == Some(&workspace.workspace_code)
                })
                .count(),
            children: Vec::new(),
        })
        .collect::<Vec<_>>();
    children.sort_by(|left, right| left.name.cmp(&right.name));
    children
}

fn search_included_codes(
    workspaces: &[Workspace],
    search: Option<&str>,
    visible_status: Option<WorkspaceStatus>,
) -> Option<BTreeSet<String>> {
    let search = search.map(str::trim).filter(|value| !value.is_empty())?;
    let search = search.to_ascii_lowercase();
    let mut included = workspaces
        .iter()
        .filter(|workspace| workspace_matches_status(workspace, visible_status))
        .filter(|workspace| {
            workspace
                .workspace_code
                .to_ascii_lowercase()
                .contains(&search)
                || workspace.name.to_ascii_lowercase().contains(&search)
                || workspace
                    .description
                    .as_deref()
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .contains(&search)
        })
        .map(|workspace| workspace.workspace_code.clone())
        .collect::<BTreeSet<_>>();
    if included.is_empty() {
        return Some(included);
    }
    let matched_codes = included.iter().cloned().collect::<Vec<_>>();
    for code in matched_codes {
        let mut current = find_workspace(workspaces, &code)
            .ok()
            .and_then(|workspace| workspace.parent_workspace_code.clone());
        while let Some(parent_code) = current {
            let Some(parent) = workspaces.iter().find(|workspace| {
                workspace.workspace_code == parent_code
                    && workspace_matches_status(workspace, visible_status)
            }) else {
                break;
            };
            included.insert(parent.workspace_code.clone());
            current = parent.parent_workspace_code.clone();
        }
    }
    Some(included)
}

fn visible_workspace_status(
    status: Option<WorkspaceStatus>,
    include_archived: Option<bool>,
) -> Option<WorkspaceStatus> {
    match (status, include_archived.unwrap_or(false)) {
        (None, true) => None,
        (Some(status), _) => Some(status),
        (None, false) => Some(WorkspaceStatus::Active),
    }
}

fn workspace_matches_status(
    workspace: &Workspace,
    visible_status: Option<WorkspaceStatus>,
) -> bool {
    visible_status.is_none_or(|status| workspace.status == status)
}
