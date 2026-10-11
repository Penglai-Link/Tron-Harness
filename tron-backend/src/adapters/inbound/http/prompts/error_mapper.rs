use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::domain::prompt::PromptError;

use super::response::PromptApiResponse;

pub fn prompt_error_response(error: PromptError) -> Response {
    let (status, code, message) = match error {
        PromptError::NotFound => (
            StatusCode::NOT_FOUND,
            "PROMPT_NOT_FOUND",
            "prompt was not found".into(),
        ),
        PromptError::InvalidInput(message) => {
            (StatusCode::BAD_REQUEST, "PROMPT_INPUT_INVALID", message)
        }
        PromptError::DuplicateDisplayName => (
            StatusCode::CONFLICT,
            "PROMPT_DISPLAY_NAME_DUPLICATE",
            "displayName already exists".into(),
        ),
        PromptError::BuiltinMutationDenied => (
            StatusCode::BAD_REQUEST,
            "PROMPT_BUILTIN_MUTATION_INVALID",
            "built-in prompts cannot be deleted".into(),
        ),
        PromptError::RuntimeRequired => (
            StatusCode::BAD_REQUEST,
            "PROMPT_RUNTIME_REQUIRED",
            "runtime-required prompts cannot be disabled".into(),
        ),
        PromptError::VersionConflict => (
            StatusCode::CONFLICT,
            "PROMPT_VERSION_CONFLICT",
            "prompt version conflict".into(),
        ),
        PromptError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "PROMPT_STORAGE_FAILED",
            "prompt storage operation failed".into(),
        ),
    };

    (status, Json(PromptApiResponse::error(code, message))).into_response()
}
