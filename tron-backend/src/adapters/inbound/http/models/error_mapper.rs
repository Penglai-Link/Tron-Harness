use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::model::ModelError, response::ApiResponse};

pub fn model_error_response(error: ModelError) -> Response {
    let (status, code, message) = match error {
        ModelError::NotFound => (
            StatusCode::NOT_FOUND,
            "MODEL_NOT_FOUND",
            "model was not found".to_string(),
        ),
        ModelError::WorkspaceRequired => (
            StatusCode::BAD_REQUEST,
            "MODEL_WORKSPACE_REQUIRED",
            "workspaceCode is required".to_string(),
        ),
        ModelError::InvalidWorkspace => (
            StatusCode::BAD_REQUEST,
            "MODEL_WORKSPACE_INVALID",
            "model workspace must reference an active workspace".to_string(),
        ),
        ModelError::DisplayNameRequired => (
            StatusCode::BAD_REQUEST,
            "MODEL_DISPLAY_NAME_REQUIRED",
            "displayName is required".to_string(),
        ),
        ModelError::VendorRequired => (
            StatusCode::BAD_REQUEST,
            "MODEL_VENDOR_REQUIRED",
            "vendor is required".to_string(),
        ),
        ModelError::BaseUrlRequired => (
            StatusCode::BAD_REQUEST,
            "MODEL_BASE_URL_REQUIRED",
            "baseUrl is required".to_string(),
        ),
        ModelError::InvalidBaseUrl => (
            StatusCode::BAD_REQUEST,
            "MODEL_BASE_URL_INVALID",
            "baseUrl must be a valid absolute URL".to_string(),
        ),
        ModelError::ApiKeyRefRequired => (
            StatusCode::BAD_REQUEST,
            "MODEL_API_KEY_REQUIRED",
            "apiKey is required".to_string(),
        ),
        ModelError::ModelNameRequired => (
            StatusCode::BAD_REQUEST,
            "MODEL_NAME_REQUIRED",
            "modelName is required".to_string(),
        ),
        ModelError::InvalidContextWindowTokens => (
            StatusCode::BAD_REQUEST,
            "MODEL_CONTEXT_WINDOW_INVALID",
            "contextWindowTokens must be greater than zero".to_string(),
        ),
        ModelError::DefaultReasoningEffortUnsupported => (
            StatusCode::BAD_REQUEST,
            "MODEL_DEFAULT_REASONING_EFFORT_UNSUPPORTED",
            "defaultReasoningEffort must be included in supportedReasoningEfforts".to_string(),
        ),
        ModelError::ReasoningEffortUnsupported => (
            StatusCode::BAD_REQUEST,
            "MODEL_REASONING_EFFORT_UNSUPPORTED",
            "reasoningEffort is not supported by the selected model".to_string(),
        ),
        ModelError::DuplicatePlatformDisplayName => (
            StatusCode::CONFLICT,
            "MODEL_DISPLAY_NAME_DUPLICATE",
            "displayName already exists in platform models".to_string(),
        ),
        ModelError::DuplicateWorkspaceDisplayName { workspace_code } => (
            StatusCode::CONFLICT,
            "MODEL_DISPLAY_NAME_DUPLICATE",
            format!("displayName already exists in workspace {workspace_code}"),
        ),
        ModelError::InvalidProtocolModelType => (
            StatusCode::BAD_REQUEST,
            "MODEL_PROTOCOL_MODEL_TYPE_INVALID",
            "protocol and modelType combination is invalid".to_string(),
        ),
        ModelError::ResponsesStatefulModeUnsupported => (
            StatusCode::BAD_REQUEST,
            "MODEL_RESPONSES_STATEFUL_MODE_UNSUPPORTED",
            "responsesStatefulEnabled is only supported for Responses protocol".to_string(),
        ),
        ModelError::PlatformSourceRequired => (
            StatusCode::BAD_REQUEST,
            "MODEL_PLATFORM_SOURCE_REQUIRED",
            "sourceModelCode is required".to_string(),
        ),
        ModelError::PlatformSourceInvalid => (
            StatusCode::BAD_REQUEST,
            "MODEL_PLATFORM_SOURCE_INVALID",
            "sourceModelCode must reference a platform model".to_string(),
        ),
        ModelError::PlatformReferenceMutationDenied => (
            StatusCode::BAD_REQUEST,
            "MODEL_PLATFORM_REFERENCE_MUTATION_INVALID",
            "platform model references must be managed from platform settings".to_string(),
        ),
        ModelError::InheritedMutationDenied => (
            StatusCode::BAD_REQUEST,
            "MODEL_INHERITED_MUTATION_INVALID",
            "inherited models are read-only in descendant workspaces".to_string(),
        ),
        ModelError::AllowInheritanceUnsupported => (
            StatusCode::BAD_REQUEST,
            "MODEL_ALLOW_INHERITANCE_INVALID",
            "allowInheritance can only be changed on workspace-local models".to_string(),
        ),
        ModelError::DefaultTargetDisabled => (
            StatusCode::BAD_REQUEST,
            "MODEL_DEFAULT_TARGET_DISABLED",
            "default model must be enabled before it can be assigned".to_string(),
        ),
        ModelError::DefaultTargetTypeMismatch => (
            StatusCode::BAD_REQUEST,
            "MODEL_DEFAULT_TARGET_TYPE_MISMATCH",
            "default model assignment must use a visible model of the requested modelType"
                .to_string(),
        ),
        ModelError::ReferencedByWorkspace { workspace_count } => (
            StatusCode::CONFLICT,
            "MODEL_REFERENCED_BY_WORKSPACE",
            format!("platform model is still referenced by {workspace_count} workspace record(s)"),
        ),
        ModelError::VerificationFailed(message) => {
            (StatusCode::BAD_GATEWAY, "MODEL_VERIFY_FAILED", message)
        }
        ModelError::RuntimeResolutionFailed(message) => {
            (StatusCode::BAD_GATEWAY, "MODEL_RESOLUTION_FAILED", message)
        }
        ModelError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "MODEL_STORAGE_FAILED",
            "model storage operation failed".to_string(),
        ),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
