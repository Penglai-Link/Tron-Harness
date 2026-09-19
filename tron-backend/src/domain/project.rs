use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProjectStatus {
    Active,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub id: i64,
    pub project_code: String,
    pub workspace_code: String,
    pub name: String,
    pub status: ProjectStatus,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectAppLink {
    pub id: i64,
    pub project_code: String,
    pub app_code: String,
    pub position: usize,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    pub project_code: String,
    pub workspace_code: String,
    pub name: String,
    pub status: ProjectStatus,
    pub app_codes: Vec<String>,
    pub primary_app_code: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectError {
    NotFound,
    ProjectCodeInvalid,
    NameRequired,
    DuplicateApp,
    DuplicateName,
    PrimaryAppInvalid,
    AppInvalid,
    WorkspaceInvalid,
    Archived,
    StorageFailure,
}

pub fn validate_project_code(value: &str) -> Result<(), ProjectError> {
    let value = value.trim();
    if value.is_empty() || !value.starts_with("PROJECT-") || value.chars().any(|ch| !(ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '-')) {
        return Err(ProjectError::ProjectCodeInvalid);
    }
    Ok(())
}

pub fn project_status_to_storage(value: ProjectStatus) -> &'static str {
    match value { ProjectStatus::Active => "ACTIVE", ProjectStatus::Archived => "ARCHIVED" }
}

pub fn project_status_from_storage(value: &str) -> Result<ProjectStatus, ProjectError> {
    match value { "ACTIVE" => Ok(ProjectStatus::Active), "ARCHIVED" => Ok(ProjectStatus::Archived), _ => Err(ProjectError::StorageFailure) }
}
