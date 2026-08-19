use serde::Serialize;
use serde_json::Value;

use crate::domain::resource::{ResourceKind, ResourceSourceKind};

#[derive(Debug, Clone)]
pub struct ResourcePageQuery {
    pub workspace_code: String,
    pub kinds: Vec<ResourceKind>,
    pub source_kind: Option<ResourceSourceKind>,
    pub search: Option<String>,
    pub current: usize,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct ResourceGetQuery {
    pub workspace_code: String,
    pub resource_code: String,
}

#[derive(Debug, Clone)]
pub struct ResourceCreateCommand {
    pub workspace_code: String,
    pub resource_code: String,
    pub kind: ResourceKind,
    pub name: String,
    pub description: String,
    pub allow_inheritance: bool,
}

#[derive(Debug, Clone)]
pub struct ResourceUpdateCommand {
    pub workspace_code: String,
    pub resource_code: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub allow_inheritance: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ResourceDeleteCommand {
    pub workspace_code: String,
    pub resource_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDeleteResult {
    pub resource_code: String,
    pub deleted: bool,
}

#[derive(Debug)]
pub struct ResourceCommitBinary {
    pub source: std::fs::File,
    pub file_name: String,
}

#[derive(Debug, Clone)]
pub struct ResourceBinaryUploadCommand {
    pub workspace_code: String,
    pub resource_code: String,
}

#[derive(Debug, Clone)]
pub struct ResourceBinaryDownloadCommand {
    pub workspace_code: String,
    pub resource_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHttpBinaryDownload {
    pub file_name: String,
    pub byte_size: u64,
}

#[derive(Debug, Clone)]
pub struct ResourceBinaryAgentCommand {
    pub resource_code: String,
    pub source_path: Option<String>,
    pub target_path: Option<String>,
    pub overwrite: bool,
    pub agent_local_directory: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceFileSummary {
    pub file_name: String,
    pub file_extension: String,
    pub byte_size: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRecord {
    pub workspace_code: String,
    pub source_kind: ResourceSourceKind,
    pub resource_code: String,
    pub kind: ResourceKind,
    pub name: String,
    pub description: String,
    pub allow_inheritance: bool,
    pub file: Option<ResourceFileSummary>,
    pub attributes: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePage {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<ResourceRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCommitResult {
    pub resource_code: String,
    pub kind: ResourceKind,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBinaryResult {
    pub action: String,
    pub resource_code: String,
    pub file_name: Option<String>,
    pub byte_size: Option<u64>,
    pub target_path: Option<String>,
}
