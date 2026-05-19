use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::variable::VariableError, response::ApiResponse};

pub fn variable_error_response(error: VariableError) -> Response {
    let (status, code, message) = match error {
        VariableError::NotFound => (
            StatusCode::NOT_FOUND,
            "VARIABLE_NOT_FOUND",
            "variable was not found",
        ),
        VariableError::WorkspaceRequired => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_WORKSPACE_REQUIRED",
            "workspaceCode is required",
        ),
        VariableError::InvalidWorkspace => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_WORKSPACE_INVALID",
            "variable workspace must reference an active workspace",
        ),
        VariableError::InvalidKey => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_KEY_INVALID",
            "variable key must use letters, digits, underscore, hyphen, or dot",
        ),
        VariableError::DuplicateKey => (
            StatusCode::CONFLICT,
            "VARIABLE_KEY_DUPLICATE",
            "variable key already exists in the current workspace",
        ),
        VariableError::InheritedMutationDenied => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_INHERITED_MUTATION_INVALID",
            "override the inherited variable locally before modifying it",
        ),
        VariableError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "VARIABLE_STORAGE_FAILED",
            "variable storage operation failed",
        ),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
