use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::model::ModelError, response::ApiResponse};

pub fn model_error_response(error: ModelError) -> Response {
    let (status, code, message) = match error {
        ModelError::ModelNotFound => (
            StatusCode::NOT_FOUND,
            "MODEL_NOT_FOUND",
            "model was not found",
        ),
        ModelError::InvalidModelCode => (
            StatusCode::BAD_REQUEST,
            "MODEL_CODE_INVALID",
            "modelCode is invalid",
        ),
        ModelError::WorkspaceRequired => (
            StatusCode::BAD_REQUEST,
            "MODEL_WORKSPACE_REQUIRED",
            "workspaceCode is required",
        ),
        ModelError::InvalidWorkspace => (
            StatusCode::BAD_REQUEST,
            "MODEL_WORKSPACE_INVALID",
            "model workspace must reference an active workspace",
        ),
        ModelError::PlatformMutationDenied => (
            StatusCode::BAD_REQUEST,
            "MODEL_PLATFORM_MUTATION_DENIED",
            "platform workspace is the source of truth; remove from a child workspace instead",
        ),
        ModelError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "MODEL_STORAGE_FAILED",
            "model storage operation failed",
        ),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
