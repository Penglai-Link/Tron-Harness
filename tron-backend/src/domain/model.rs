use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::shared::id::SnowflakeId;

pub const PLATFORM_WORKSPACE_CODE: &str = "PLATFORM";
pub const PLATFORM_WORKSPACE_NAME: &str = "Platform";
pub const PLATFORM_SOURCE_LABEL: &str = "Platform";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelVendor {
    OpenAI,
    Anthropic,
    Google,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformModelStatus {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceModelStatus {
    Available,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformModel {
    pub id: SnowflakeId,
    pub model_code: String,
    pub name: String,
    pub purpose: String,
    pub vendor: ModelVendor,
    pub context_window: String,
    pub max_output: String,
    pub training_data: String,
    pub speed: String,
    pub price: String,
    pub status: PlatformModelStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceModelRemoval {
    pub id: SnowflakeId,
    pub workspace_code: String,
    pub model_code: String,
    pub removed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceModelRecord {
    pub model_code: String,
    pub name: String,
    pub purpose: String,
    pub vendor: ModelVendor,
    pub context: String,
    pub max_output: String,
    pub training_data: String,
    pub speed: String,
    pub price: String,
    pub status: WorkspaceModelStatus,
    pub source_workspace_code: String,
    pub source_workspace_name: String,
    pub source_label: String,
    pub removed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceModelMutationResult {
    pub workspace_code: String,
    pub model_code: String,
    pub status: WorkspaceModelStatus,
    pub removed_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceModelQuery {
    pub search: Option<String>,
    pub status: Option<WorkspaceModelStatus>,
    pub vendor: Option<ModelVendor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    ModelNotFound,
    InvalidModelCode,
    WorkspaceRequired,
    InvalidWorkspace,
    PlatformMutationDenied,
    StorageFailure,
}

impl fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModelNotFound => formatter.write_str("model was not found"),
            Self::InvalidModelCode => formatter.write_str("model code is invalid"),
            Self::WorkspaceRequired => formatter.write_str("workspaceCode is required"),
            Self::InvalidWorkspace => {
                formatter.write_str("model workspace must reference an active workspace")
            }
            Self::PlatformMutationDenied => formatter.write_str(
                "platform workspace is the source of truth; remove from a child workspace instead",
            ),
            Self::StorageFailure => formatter.write_str("model storage failure"),
        }
    }
}

impl Error for ModelError {}

pub fn validate_model_code(value: &str) -> Result<(), ModelError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ModelError::InvalidModelCode);
    }

    if trimmed
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.'))
    {
        return Err(ModelError::InvalidModelCode);
    }

    Ok(())
}

pub fn vendor_to_storage(value: ModelVendor) -> &'static str {
    match value {
        ModelVendor::OpenAI => "openai",
        ModelVendor::Anthropic => "anthropic",
        ModelVendor::Google => "google",
    }
}

pub fn vendor_from_storage(value: &str) -> Result<ModelVendor, ModelError> {
    match value {
        "openai" => Ok(ModelVendor::OpenAI),
        "anthropic" => Ok(ModelVendor::Anthropic),
        "google" => Ok(ModelVendor::Google),
        _ => Err(ModelError::StorageFailure),
    }
}

pub fn platform_status_to_storage(value: PlatformModelStatus) -> &'static str {
    match value {
        PlatformModelStatus::Enabled => "enabled",
        PlatformModelStatus::Disabled => "disabled",
    }
}

pub fn platform_status_from_storage(value: &str) -> Result<PlatformModelStatus, ModelError> {
    match value {
        "enabled" => Ok(PlatformModelStatus::Enabled),
        "disabled" => Ok(PlatformModelStatus::Disabled),
        _ => Err(ModelError::StorageFailure),
    }
}
