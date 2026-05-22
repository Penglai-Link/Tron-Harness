use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::shared::id::SnowflakeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppRuntimeRole {
    Frontend,
    Backend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppStatus {
    Active,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub id: SnowflakeId,
    pub app_code: String,
    pub workspace_code: String,
    pub name: String,
    pub description: Option<String>,
    pub local_directory: String,
    pub runtime_role: AppRuntimeRole,
    pub repository_url: Option<String>,
    pub default_branch: Option<String>,
    pub status: AppStatus,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppTag {
    pub app_code: String,
    pub tag: String,
    pub linked_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppListItem {
    pub app_code: String,
    pub workspace_code: String,
    pub name: String,
    pub description: Option<String>,
    pub local_directory: String,
    pub runtime_role: AppRuntimeRole,
    pub repository_url: Option<String>,
    pub default_branch: Option<String>,
    pub tags: Vec<String>,
    pub status: AppStatus,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDetail {
    pub id: String,
    pub app_code: String,
    pub workspace_code: String,
    pub name: String,
    pub description: Option<String>,
    pub local_directory: String,
    pub runtime_role: AppRuntimeRole,
    pub repository_url: Option<String>,
    pub default_branch: Option<String>,
    pub tags: Vec<String>,
    pub status: AppStatus,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppOption {
    pub app_code: String,
    pub name: String,
    pub description: Option<String>,
    pub runtime_role: AppRuntimeRole,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppLifecycleResult {
    pub app_code: String,
    pub status: AppStatus,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedAppRecords<T> {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<T>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppListQuery {
    pub current: Option<usize>,
    pub size: Option<usize>,
    pub search: Option<String>,
    pub runtime_role: Option<AppRuntimeRole>,
    pub status: Option<AppStatus>,
    pub include_archived: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppOptionQuery {
    pub search: Option<String>,
    pub runtime_role: Option<AppRuntimeRole>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    NotFound,
    InvalidAppCode,
    WorkspaceRequired,
    InvalidWorkspace,
    NameRequired,
    LocalDirectoryRequired,
    DuplicateName,
    ArchivedMutationDenied,
    StorageFailure,
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("app was not found"),
            Self::InvalidAppCode => formatter.write_str("invalid app code"),
            Self::WorkspaceRequired => formatter.write_str("workspaceCode is required"),
            Self::InvalidWorkspace => formatter.write_str("app workspace is invalid"),
            Self::NameRequired => formatter.write_str("app name is required"),
            Self::LocalDirectoryRequired => formatter.write_str("localDirectory is required"),
            Self::DuplicateName => formatter.write_str("duplicate app name in workspace"),
            Self::ArchivedMutationDenied => formatter.write_str("archived app cannot be modified"),
            Self::StorageFailure => formatter.write_str("app storage failure"),
        }
    }
}

impl Error for AppError {}

pub fn validate_app_code(code: &str) -> Result<(), AppError> {
    let trimmed = code.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidAppCode);
    }
    if trimmed
        .chars()
        .any(|ch| !(ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_' || ch == '-'))
    {
        return Err(AppError::InvalidAppCode);
    }
    Ok(())
}

pub fn app_status_to_storage(status: AppStatus) -> &'static str {
    match status {
        AppStatus::Active => "active",
        AppStatus::Archived => "archived",
    }
}

pub fn app_status_from_storage(value: &str) -> Result<AppStatus, AppError> {
    match value {
        "active" => Ok(AppStatus::Active),
        "archived" => Ok(AppStatus::Archived),
        _ => Err(AppError::StorageFailure),
    }
}

pub fn app_runtime_role_to_storage(runtime_role: AppRuntimeRole) -> &'static str {
    match runtime_role {
        AppRuntimeRole::Frontend => "frontend",
        AppRuntimeRole::Backend => "backend",
    }
}

pub fn app_runtime_role_from_storage(value: &str) -> Result<AppRuntimeRole, AppError> {
    match value {
        "frontend" => Ok(AppRuntimeRole::Frontend),
        "backend" => Ok(AppRuntimeRole::Backend),
        _ => Err(AppError::StorageFailure),
    }
}
