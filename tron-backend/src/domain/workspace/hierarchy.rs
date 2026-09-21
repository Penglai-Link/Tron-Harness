use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::shared::id::SnowflakeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceStatus {
    Active,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub id: SnowflakeId,
    pub workspace_code: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_workspace_code: Option<String>,
    pub status: WorkspaceStatus,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTreeNode {
    pub id: String,
    pub workspace_code: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_workspace_code: Option<String>,
    pub depth: usize,
    pub status: WorkspaceStatus,
    pub child_count: usize,
    pub children: Vec<WorkspaceTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceOption {
    pub workspace_code: String,
    pub label: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_workspace_code: Option<String>,
    pub depth: usize,
    pub status: WorkspaceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLineageItem {
    pub workspace_code: String,
    pub name: String,
    pub depth: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDetail {
    pub id: String,
    pub workspace_code: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_workspace_code: Option<String>,
    pub depth: usize,
    pub status: WorkspaceStatus,
    pub path: Vec<WorkspaceLineageItem>,
    pub children: Vec<WorkspaceTreeNode>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSwitcherPrimarySpace {
    pub workspace_code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: WorkspaceStatus,
    pub child_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSwitcherSecondarySpace {
    pub workspace_code: String,
    pub primary_workspace_code: String,
    pub parent_workspace_code: Option<String>,
    pub label: String,
    pub name: String,
    pub description: Option<String>,
    pub status: WorkspaceStatus,
    pub depth: usize,
    pub child_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSwitcherData {
    pub primary_spaces: Vec<WorkspaceSwitcherPrimarySpace>,
    pub secondary_spaces: Vec<WorkspaceSwitcherSecondarySpace>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLifecycleResult {
    pub workspace_code: String,
    pub status: WorkspaceStatus,
    pub archived_at: Option<String>,
    pub affected_workspace_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedWorkspaceRecords<T> {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<T>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceListQuery {
    pub include_archived: Option<bool>,
    pub search: Option<String>,
    pub status: Option<WorkspaceStatus>,
    pub root_only: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceError {
    NotFound,
    InvalidWorkspaceCode,
    DuplicateWorkspaceCode,
    WorkspaceNameRequired,
    InvalidHierarchy,
    ArchivedMutationDenied,
    StorageFailure,
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("workspace was not found"),
            Self::InvalidWorkspaceCode => formatter.write_str("invalid workspace code"),
            Self::DuplicateWorkspaceCode => formatter.write_str("duplicate workspace code"),
            Self::WorkspaceNameRequired => formatter.write_str("workspace name is required"),
            Self::InvalidHierarchy => formatter.write_str("invalid workspace hierarchy"),
            Self::ArchivedMutationDenied => {
                formatter.write_str("archived workspace cannot be modified")
            }
            Self::StorageFailure => formatter.write_str("workspace storage failure"),
        }
    }
}

impl Error for WorkspaceError {}

pub fn validate_workspace_code(code: &str) -> Result<(), WorkspaceError> {
    let trimmed = code.trim();
    if trimmed.is_empty() {
        return Err(WorkspaceError::InvalidWorkspaceCode);
    }

    if trimmed
        .chars()
        .any(|ch| !(ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_' || ch == '-'))
    {
        return Err(WorkspaceError::InvalidWorkspaceCode);
    }

    Ok(())
}

pub fn workspace_status_to_storage(status: WorkspaceStatus) -> &'static str {
    match status {
        WorkspaceStatus::Active => "active",
        WorkspaceStatus::Archived => "archived",
    }
}

pub fn workspace_status_from_storage(value: &str) -> Result<WorkspaceStatus, WorkspaceError> {
    match value {
        "active" => Ok(WorkspaceStatus::Active),
        "archived" => Ok(WorkspaceStatus::Archived),
        _ => Err(WorkspaceError::StorageFailure),
    }
}
