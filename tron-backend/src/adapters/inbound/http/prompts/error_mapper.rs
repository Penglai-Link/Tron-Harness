use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::domain::prompt::PromptError;

use super::response::PromptApiResponse;

pub fn prompt_error_response(error: PromptError) -> Response {
    let (status, code, message) = match error {
        PromptError::WorkspaceRequired => (
            StatusCode::BAD_REQUEST,
            "PROMPT_WORKSPACE_REQUIRED",
            "workspaceCode is required".into(),
        ),
        PromptError::InvalidWorkspace => (
            StatusCode::BAD_REQUEST,
            "PROMPT_WORKSPACE_INVALID",
            "prompt workspace must reference an active workspace".into(),
        ),
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
            "displayName already exists in prompt scope".into(),
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
        PromptError::InheritedMutationDenied => (
            StatusCode::BAD_REQUEST,
            "PROMPT_INHERITED_MUTATION_INVALID",
            "inherited prompt cannot be deleted".into(),
        ),
        PromptError::AllowInheritanceUnsupported => (
            StatusCode::BAD_REQUEST,
            "PROMPT_ALLOW_INHERITANCE_INVALID",
            "allowInheritance is supported only for workspace-local prompts".into(),
        ),
        PromptError::RevertUnsupported => (
            StatusCode::BAD_REQUEST,
            "PROMPT_REVERT_INVALID",
            "prompt has no workspace override to revert".into(),
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
