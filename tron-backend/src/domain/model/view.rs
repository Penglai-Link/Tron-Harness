use serde::Serialize;

use crate::domain::model_provider::{
    ModelProviderCode, ModelSelection, ProviderCatalogModelStatus,
};
use crate::domain::reasoning::ReasoningEffort;

use super::{
    ModelCapability, ModelProtocol, ModelScopeKind, ModelType, ModelVerifyStatus,
    ModelViewSourceKind,
};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelView {
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
    pub source_kind: ModelViewSourceKind,
    pub source_model_code: Option<String>,
    pub source_label: String,
    pub source_workspace_code: Option<String>,
    pub source_workspace_name: Option<String>,
    pub source_workspace_path: Vec<ModelWorkspacePathItem>,
    pub allow_inheritance: Option<bool>,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelWorkspacePathItem {
    pub workspace_code: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDefaultModelView {
    pub model_type: ModelType,
    pub model_code: Option<String>,
    pub display_name: Option<String>,
    pub source_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceModelCatalog {
    pub workspace_code: String,
    pub defaults: Vec<WorkspaceDefaultModelView>,
    pub records: Vec<ModelView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSelectOption {
    pub option_code: String,
    pub model_code: String,
    pub display_name: String,
    pub model_name: String,
    pub source_kind: ModelViewSourceKind,
    pub source_label: String,
    pub vendor: String,
    pub protocol: ModelProtocol,
    pub model_type: ModelType,
    pub enabled: bool,
    pub model_selection: ModelSelection,
    pub provider_code: Option<ModelProviderCode>,
    pub access_platform: Option<String>,
    pub upstream_vendor_name: Option<String>,
    pub remote_status: Option<ProviderCatalogModelStatus>,
    pub context_window_tokens: Option<u32>,
    pub quality_type: Option<crate::domain::model_provider::ProviderModelQuality>,
    pub input_coefficient: Option<f64>,
    pub cache_hit_coefficient: Option<f64>,
    pub output_coefficient: Option<f64>,
    pub supported_reasoning_efforts: Vec<ReasoningEffort>,
    pub default_reasoning_effort: Option<ReasoningEffort>,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelMutationResult {
    pub workspace_code: Option<String>,
    pub model_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDefaultMutationResult {
    pub workspace_code: String,
    pub model_type: ModelType,
    pub model_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDefaultMutationResult {
    pub model_type: ModelType,
    pub model_code: String,
}
