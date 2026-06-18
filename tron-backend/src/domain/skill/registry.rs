use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillSourceType {
    Marketplace,
    GitHub,
    Git,
    Url,
    Local,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillUpdateStatus {
    Unknown,
    UpToDate,
    UpdateAvailable,
    Manual,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillManagedBy {
    Cli,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillWorkspaceSourceKind {
    Registry,
    Local,
    Inherited,
    Override,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillRegistryState {
    Present,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkillRagIndexStatus {
    Pending,
    Indexed,
    Disabled,
    Warning,
    Failed,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillSearchMode {
    Find,
    Repo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrySkill {
    pub skill_name: String,
    pub source_ref: String,
    pub source_type: SkillSourceType,
    pub registry_path: String,
    pub declared_version: Option<String>,
    pub update_status: SkillUpdateStatus,
    pub managed_by: SkillManagedBy,
    pub last_checked_at: Option<String>,
    pub last_updated_at: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSkillBinding {
    pub workspace_code: String,
    pub skill_name: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillRagIndexRecord {
    pub workspace_code: String,
    pub skill_name: String,
    pub source_version: String,
    pub desired_enabled: bool,
    pub vector_status: SkillRagIndexStatus,
    pub graph_status: SkillRagIndexStatus,
    pub content_hash: Option<String>,
    pub structure_hash: Option<String>,
    pub last_indexed_at: Option<String>,
    pub last_error: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRagIndexSummary {
    pub desired_enabled: bool,
    pub source_version: String,
    pub vector_status: SkillRagIndexStatus,
    pub graph_status: SkillRagIndexStatus,
    pub content_hash: Option<String>,
    pub structure_hash: Option<String>,
    pub last_indexed_at: Option<String>,
    pub updated_at: String,
    pub last_error: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillRagIndexBatchSummary {
    pub indexed_count: usize,
    pub disabled_count: usize,
    pub failed_count: usize,
    pub warning_count: usize,
    pub warnings: Vec<String>,
}

impl SkillRagIndexRecord {
    pub fn summary(&self) -> SkillRagIndexSummary {
        SkillRagIndexSummary {
            desired_enabled: self.desired_enabled,
            source_version: self.source_version.clone(),
            vector_status: self.vector_status,
            graph_status: self.graph_status,
            content_hash: self.content_hash.clone(),
            structure_hash: self.structure_hash.clone(),
            last_indexed_at: self.last_indexed_at.clone(),
            updated_at: self.updated_at.clone(),
            last_error: self.last_error.clone(),
            warnings: self
                .last_error
                .as_deref()
                .map(split_rag_index_warnings)
                .unwrap_or_default(),
        }
    }
}

fn split_rag_index_warnings(message: &str) -> Vec<String> {
    message
        .split(';')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySkillListItem {
    pub skill_name: String,
    pub source_ref: String,
    pub description: Option<String>,
    pub source_type: SkillSourceType,
    pub registry_path: String,
    pub declared_version: Option<String>,
    pub update_status: SkillUpdateStatus,
    pub managed_by: SkillManagedBy,
    pub last_checked_at: Option<String>,
    pub last_updated_at: Option<String>,
    pub last_error: Option<String>,
    pub enabled: bool,
    pub source_kind: SkillWorkspaceSourceKind,
    pub source_workspace_code: Option<String>,
    pub source_workspace_name: Option<String>,
    pub source_label: Option<String>,
    pub shadowed_workspace_code: Option<String>,
    pub shadowed_workspace_name: Option<String>,
    pub shadowed_label: Option<String>,
    pub shadowed_enabled: Option<bool>,
    pub enabled_workspace_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySkillFile {
    pub path: String,
    pub is_markdown: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySkillDetail {
    pub skill_name: String,
    pub source_ref: String,
    pub source_type: SkillSourceType,
    pub registry_path: String,
    pub declared_version: Option<String>,
    pub update_status: SkillUpdateStatus,
    pub managed_by: SkillManagedBy,
    pub last_checked_at: Option<String>,
    pub last_updated_at: Option<String>,
    pub last_error: Option<String>,
    pub skill_markdown: String,
    pub files: Vec<RegistrySkillFile>,
    pub enabled: bool,
    pub source_kind: SkillWorkspaceSourceKind,
    pub source_workspace_code: Option<String>,
    pub source_workspace_name: Option<String>,
    pub source_label: Option<String>,
    pub shadowed_workspace_code: Option<String>,
    pub shadowed_workspace_name: Option<String>,
    pub shadowed_label: Option<String>,
    pub shadowed_enabled: Option<bool>,
    pub enabled_workspace_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSkillListItem {
    pub skill_name: String,
    pub source_ref: Option<String>,
    pub source_type: Option<SkillSourceType>,
    pub registry_path: Option<String>,
    pub declared_version: Option<String>,
    pub update_status: Option<SkillUpdateStatus>,
    pub managed_by: Option<SkillManagedBy>,
    pub last_checked_at: Option<String>,
    pub last_updated_at: Option<String>,
    pub last_error: Option<String>,
    pub enabled: bool,
    pub source_kind: SkillWorkspaceSourceKind,
    pub source_workspace_code: Option<String>,
    pub source_workspace_name: Option<String>,
    pub source_label: Option<String>,
    pub shadowed_workspace_code: Option<String>,
    pub shadowed_workspace_name: Option<String>,
    pub shadowed_label: Option<String>,
    pub shadowed_enabled: Option<bool>,
    pub registry_state: SkillRegistryState,
    pub rag_index: Option<SkillRagIndexSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSkillDetail {
    pub skill_name: String,
    pub source_ref: Option<String>,
    pub source_type: Option<SkillSourceType>,
    pub registry_path: Option<String>,
    pub declared_version: Option<String>,
    pub update_status: Option<SkillUpdateStatus>,
    pub managed_by: Option<SkillManagedBy>,
    pub last_checked_at: Option<String>,
    pub last_updated_at: Option<String>,
    pub last_error: Option<String>,
    pub skill_markdown: Option<String>,
    pub files: Vec<RegistrySkillFile>,
    pub enabled: bool,
    pub source_kind: SkillWorkspaceSourceKind,
    pub source_workspace_code: Option<String>,
    pub source_workspace_name: Option<String>,
    pub source_label: Option<String>,
    pub shadowed_workspace_code: Option<String>,
    pub shadowed_workspace_name: Option<String>,
    pub shadowed_label: Option<String>,
    pub shadowed_enabled: Option<bool>,
    pub registry_state: SkillRegistryState,
    pub rag_index: Option<SkillRagIndexSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSearchCandidate {
    pub package_ref: String,
    pub source_ref: String,
    pub skill_name: String,
    pub description: Option<String>,
    pub installs_label: Option<String>,
    pub detail_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRegistryMutationResult {
    pub skill_name: String,
    pub workspace_code: Option<String>,
    pub enabled: bool,
    pub rag_index: Option<SkillRagIndexSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRegistryUninstallResult {
    pub skill_name: String,
    pub removed_bindings_count: usize,
    pub kept_bindings_count: usize,
    pub rag_index: Option<SkillRagIndexBatchSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillBindingMutationResult {
    pub workspace_code: String,
    pub skill_name: String,
    pub enabled: bool,
    pub rag_index: Option<SkillRagIndexSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRegistrySyncResult {
    pub registry_root: String,
    pub discovered_count: usize,
    pub removed_count: usize,
    pub checked: bool,
    pub updated: bool,
    pub rag_index: Option<SkillRagIndexBatchSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedWorkspaceSkill {
    pub skill_name: String,
    pub source_path: String,
    pub target_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedWorkspaceSkills {
    pub workspace_code: String,
    pub worktree_path: String,
    pub skills: Vec<PreparedWorkspaceSkill>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySkillListQuery {
    pub workspace_code: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSkillListQuery {
    pub search: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillError {
    SearchQueryRequired,
    SourceRefRequired,
    SkillNameRequired,
    WorkspaceRequired,
    InvalidWorkspace,
    NotFound,
    DuplicateBinding,
    InvalidRegistryPath,
    StorageFailure,
    CommandFailed,
}

impl fmt::Display for SkillError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SearchQueryRequired => formatter.write_str("search query is required"),
            Self::SourceRefRequired => formatter.write_str("sourceRef is required"),
            Self::SkillNameRequired => formatter.write_str("skillName is required"),
            Self::WorkspaceRequired => formatter.write_str("workspaceCode is required"),
            Self::InvalidWorkspace => {
                formatter.write_str("skill workspace must reference an active workspace")
            }
            Self::NotFound => formatter.write_str("skill was not found"),
            Self::DuplicateBinding => {
                formatter.write_str("skill binding already exists in the workspace")
            }
            Self::InvalidRegistryPath => formatter.write_str("invalid skill registry path"),
            Self::StorageFailure => formatter.write_str("skill storage failure"),
            Self::CommandFailed => formatter.write_str("skills command failed"),
        }
    }
}

impl Error for SkillError {}
