use std::{error::Error, fmt};

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
    InvalidContextWindowTokens,
    DefaultReasoningEffortUnsupported,
    ReasoningEffortUnsupported,
    DuplicatePlatformDisplayName,
    DuplicateWorkspaceDisplayName { workspace_code: String },
    InvalidProtocolModelType,
    ResponsesStatefulModeUnsupported,
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
            Self::InvalidContextWindowTokens => {
                formatter.write_str("contextWindowTokens must be greater than zero")
            }
            Self::DefaultReasoningEffortUnsupported => formatter
                .write_str("defaultReasoningEffort must be included in supportedReasoningEfforts"),
            Self::ReasoningEffortUnsupported => {
                formatter.write_str("reasoningEffort is not supported by the selected model")
            }
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
            Self::ResponsesStatefulModeUnsupported => formatter
                .write_str("responsesStatefulEnabled is only supported for Responses protocol"),
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
