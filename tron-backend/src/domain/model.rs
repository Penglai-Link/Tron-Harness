use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::shared::id::SnowflakeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelScopeKind {
    Platform,
    Workspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelSourceKind {
    Platform,
    WorkspaceLocal,
    WorkspaceOverride,
    WorkspaceStatusPatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelViewSourceKind {
    Platform,
    Local,
    Inherited,
    Override,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelProtocol {
    #[serde(rename = "OPENAI_COMPATIBLE")]
    OpenAiCompatible,
    Responses,
    Rerank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelType {
    Text,
    Embedding,
    Rerank,
    Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelCapability {
    MultimodalAll,
    TextGeneration,
    DeepThinking,
    VisionUnderstanding,
    ImageGeneration,
    VideoGeneration,
    ThreedGeneration,
    SpeechRecognition,
    SpeechSynthesis,
    MultimodalVector,
    TextVector,
    RealtimeMultimodal,
    RealtimeSpeechSynthesis,
    RealtimeSpeechRecognition,
    RealtimeSpeechTranslation,
    Rerank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelVerifyStatus {
    Succeeded,
    Failed,
}

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
    pub structured_output_enabled: bool,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub timeout_ms: Option<u64>,
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
    pub structured_output_enabled: bool,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub timeout_ms: Option<u64>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSelectOption {
    pub model_code: String,
    pub display_name: String,
    pub model_name: String,
    pub source_kind: ModelViewSourceKind,
    pub source_label: String,
    pub vendor: String,
    pub protocol: ModelProtocol,
    pub model_type: ModelType,
    pub enabled: bool,
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

#[derive(Debug, Clone, PartialEq)]
pub enum ModelError {
    NotFound,
    WorkspaceRequired,
    InvalidWorkspace,
    DisplayNameRequired,
    VendorRequired,
    BaseUrlRequired,
    InvalidBaseUrl,
    ApiKeyRefRequired,
    ModelNameRequired,
    DuplicatePlatformDisplayName,
    DuplicateWorkspaceDisplayName { workspace_code: String },
    InvalidProtocolModelType,
    PlatformSourceRequired,
    PlatformSourceInvalid,
    PlatformReferenceMutationDenied,
    InheritedMutationDenied,
    AllowInheritanceUnsupported,
    DefaultTargetDisabled,
    DefaultTargetTypeMismatch,
    ReferencedByWorkspace { workspace_count: usize },
    VerificationFailed(String),
    StorageFailure,
}

impl fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("model was not found"),
            Self::WorkspaceRequired => formatter.write_str("workspaceCode is required"),
            Self::InvalidWorkspace => {
                formatter.write_str("model workspace must reference an active workspace")
            }
            Self::DisplayNameRequired => formatter.write_str("displayName is required"),
            Self::VendorRequired => formatter.write_str("vendor is required"),
            Self::BaseUrlRequired => formatter.write_str("baseUrl is required"),
            Self::InvalidBaseUrl => formatter.write_str("baseUrl must be a valid absolute URL"),
            Self::ApiKeyRefRequired => formatter.write_str("apiKey is required"),
            Self::ModelNameRequired => formatter.write_str("modelName is required"),
            Self::DuplicatePlatformDisplayName => {
                formatter.write_str("displayName already exists in platform models")
            }
            Self::DuplicateWorkspaceDisplayName { workspace_code } => write!(
                formatter,
                "displayName already exists in workspace {workspace_code}"
            ),
            Self::InvalidProtocolModelType => {
                formatter.write_str("protocol and modelType combination is invalid")
            }
            Self::PlatformSourceRequired => formatter.write_str("sourceModelCode is required"),
            Self::PlatformSourceInvalid => {
                formatter.write_str("sourceModelCode must reference a platform model")
            }
            Self::PlatformReferenceMutationDenied => formatter
                .write_str("platform model references must be managed from platform settings"),
            Self::InheritedMutationDenied => {
                formatter.write_str("inherited models are read-only in descendant workspaces")
            }
            Self::AllowInheritanceUnsupported => formatter
                .write_str("allowInheritance can only be changed on workspace-local models"),
            Self::DefaultTargetDisabled => {
                formatter.write_str("default model must be enabled before it can be assigned")
            }
            Self::DefaultTargetTypeMismatch => formatter.write_str(
                "default model assignment must use a visible model of the requested modelType",
            ),
            Self::ReferencedByWorkspace { workspace_count } => write!(
                formatter,
                "platform model is still referenced by {workspace_count} workspace record(s)"
            ),
            Self::VerificationFailed(message) => formatter.write_str(message),
            Self::StorageFailure => formatter.write_str("model storage failure"),
        }
    }
}

impl Error for ModelError {}

pub fn validate_model_configuration(
    display_name: &str,
    vendor: &str,
    base_url: &str,
    api_key_ref: &str,
    model_name: &str,
    protocol: ModelProtocol,
    model_type: ModelType,
    model_capabilities: &[ModelCapability],
) -> Result<(), ModelError> {
    if display_name.trim().is_empty() {
        return Err(ModelError::DisplayNameRequired);
    }
    if vendor.trim().is_empty() {
        return Err(ModelError::VendorRequired);
    }
    if base_url.trim().is_empty() {
        return Err(ModelError::BaseUrlRequired);
    }
    if url::Url::parse(base_url.trim()).is_err() {
        return Err(ModelError::InvalidBaseUrl);
    }
    if api_key_ref.trim().is_empty() {
        return Err(ModelError::ApiKeyRefRequired);
    }
    if model_name.trim().is_empty() {
        return Err(ModelError::ModelNameRequired);
    }
    if !protocol_model_type_allowed(protocol, model_type) {
        return Err(ModelError::InvalidProtocolModelType);
    }
    if !model_capabilities
        .iter()
        .any(|value| model_capability_model_type(*value) == model_type)
    {
        return Err(ModelError::InvalidProtocolModelType);
    }
    if model_capabilities
        .iter()
        .any(|value| !protocol_model_type_allowed(protocol, model_capability_model_type(*value)))
    {
        return Err(ModelError::InvalidProtocolModelType);
    }
    Ok(())
}

pub fn protocol_model_type_allowed(protocol: ModelProtocol, model_type: ModelType) -> bool {
    matches!(
        (protocol, model_type),
        (ModelProtocol::OpenAiCompatible, ModelType::Text)
            | (ModelProtocol::OpenAiCompatible, ModelType::Embedding)
            | (ModelProtocol::Responses, ModelType::Text)
            | (ModelProtocol::Responses, ModelType::Image)
            | (ModelProtocol::Rerank, ModelType::Rerank)
    )
}

pub fn default_model_capabilities_for_type(model_type: ModelType) -> Vec<ModelCapability> {
    match model_type {
        ModelType::Text => vec![ModelCapability::TextGeneration],
        ModelType::Embedding => vec![ModelCapability::TextVector],
        ModelType::Rerank => vec![ModelCapability::Rerank],
        ModelType::Image => vec![ModelCapability::ImageGeneration],
    }
}

pub fn model_capability_model_type(value: ModelCapability) -> ModelType {
    match value {
        ModelCapability::MultimodalAll
        | ModelCapability::TextGeneration
        | ModelCapability::DeepThinking
        | ModelCapability::SpeechRecognition
        | ModelCapability::SpeechSynthesis
        | ModelCapability::RealtimeMultimodal
        | ModelCapability::RealtimeSpeechSynthesis
        | ModelCapability::RealtimeSpeechRecognition
        | ModelCapability::RealtimeSpeechTranslation => ModelType::Text,
        ModelCapability::VisionUnderstanding
        | ModelCapability::ImageGeneration
        | ModelCapability::VideoGeneration
        | ModelCapability::ThreedGeneration => ModelType::Image,
        ModelCapability::MultimodalVector | ModelCapability::TextVector => ModelType::Embedding,
        ModelCapability::Rerank => ModelType::Rerank,
    }
}
