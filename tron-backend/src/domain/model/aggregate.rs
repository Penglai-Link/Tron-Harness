use crate::domain::reasoning::ReasoningEffort;
use crate::shared::id::SnowflakeId;

use super::{
    ModelCapability, ModelProtocol, ModelScopeKind, ModelSourceKind, ModelType, ModelVerifyStatus,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub id: SnowflakeId,
    pub scope_kind: ModelScopeKind,
    pub workspace_code: Option<String>,
    pub model_code: String,
    pub display_name: String,
    pub vendor: String,
    pub protocol: ModelProtocol,
    pub model_type: ModelType,
    pub model_capabilities: Vec<ModelCapability>,
    pub base_url: String,
    pub api_path: Option<String>,
    pub api_key_ref: String,
    pub model_name: String,
    pub enabled: bool,
    pub description: Option<String>,
    pub thinking_enabled: bool,
    pub supported_reasoning_efforts: Vec<ReasoningEffort>,
    pub default_reasoning_effort: Option<ReasoningEffort>,
    pub structured_output_enabled: bool,
    pub responses_stateful_enabled: bool,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub context_window_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub max_retries: Option<u32>,
    pub last_verified_at: Option<String>,
    pub last_verify_status: Option<ModelVerifyStatus>,
    pub last_verify_message: Option<String>,
    pub source_kind: ModelSourceKind,
    pub source_model_code: Option<String>,
    pub allow_inheritance: Option<bool>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDefaultModelAssignment {
    pub workspace_code: String,
    pub model_type: ModelType,
    pub model_code: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformDefaultModelAssignment {
    pub model_type: ModelType,
    pub model_code: String,
    pub updated_at: String,
}
