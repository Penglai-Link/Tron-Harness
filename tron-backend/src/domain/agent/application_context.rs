use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplicationDirectoryStatus {
    Ready,
    NotGit,
    MissingDirectory,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationDirectoryContext {
    pub app_code: String,
    pub app_name: String,
    pub is_primary: bool,
    pub configured_directory: String,
    pub working_directory: Option<String>,
    pub repository_root: Option<String>,
    pub branch_name: Option<String>,
    pub head_commit: Option<String>,
    pub worktree_path: Option<String>,
    pub is_worktree: bool,
    pub status: ApplicationDirectoryStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitWorktreeContext {
    pub path: String,
    pub branch_name: Option<String>,
    pub head_commit: Option<String>,
}
