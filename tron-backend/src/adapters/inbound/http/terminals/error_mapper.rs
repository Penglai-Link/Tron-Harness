use axum::{
    Json,
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    application::{llm::LlmRuntimeError, terminal::TerminalError},
    response::ApiResponse,
};

pub fn llm_runtime_error_response(error: LlmRuntimeError) -> Response {
    let (status, code, message) = match error {
        LlmRuntimeError::WorkspaceRequired => (
            StatusCode::BAD_REQUEST,
            "TERMINAL_WORKSPACE_REQUIRED",
            "workspaceCode is required".to_owned(),
        ),
        LlmRuntimeError::InputRequired => (
            StatusCode::BAD_REQUEST,
            "TERMINAL_COMMAND_PROMPT_REQUIRED",
            "prompt is required".to_owned(),
        ),
        LlmRuntimeError::TextModelUnavailable => (
            StatusCode::BAD_REQUEST,
            "TERMINAL_COMMAND_MODEL_UNAVAILABLE",
            "a text model must be configured for the workspace".to_owned(),
        ),
        LlmRuntimeError::ModelResolutionFailed(message) => (
            StatusCode::BAD_REQUEST,
            "TERMINAL_COMMAND_MODEL_INVALID",
            message,
        ),
        LlmRuntimeError::ProviderFailed(message)
        | LlmRuntimeError::InvalidProviderResponse(message) => (
            StatusCode::BAD_GATEWAY,
            "TERMINAL_COMMAND_GENERATION_FAILED",
            message,
        ),
        LlmRuntimeError::QueryRequired
        | LlmRuntimeError::DocumentsRequired
        | LlmRuntimeError::EmbeddingModelUnavailable
        | LlmRuntimeError::RerankModelUnavailable
        | LlmRuntimeError::InvalidChunkStrategy(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "TERMINAL_COMMAND_INTERNAL_ERROR",
            "terminal command generation failed".to_owned(),
        ),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}

pub fn invalid_terminal_request_response(
    rejection: JsonRejection,
    code: &'static str,
    message: &'static str,
) -> Response {
    let _ = rejection;
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::error(code, message)),
    )
        .into_response()
}

pub fn prompt_render_error_response() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiResponse::error(
            "TERMINAL_PROMPT_RENDER_FAILED",
            "terminal command prompt could not be rendered",
        )),
    )
        .into_response()
}

pub fn terminal_error_response(error: TerminalError) -> Response {
    let (status, code, message) = match error {
        TerminalError::WorkspaceRequired => (
            StatusCode::BAD_REQUEST,
            "TERMINAL_WORKSPACE_REQUIRED",
            "workspaceCode is required",
        ),
        TerminalError::UserRequired => (
            StatusCode::UNAUTHORIZED,
            "TERMINAL_USER_REQUIRED",
            "authenticated user is required",
        ),
        TerminalError::AppNotFound => (
            StatusCode::NOT_FOUND,
            "TERMINAL_APP_NOT_FOUND",
            "terminal app was not found",
        ),
        TerminalError::AppWorkspaceMismatch => (
            StatusCode::BAD_REQUEST,
            "TERMINAL_APP_WORKSPACE_INVALID",
            "appCode does not belong to workspaceCode",
        ),
        TerminalError::AppArchived => (
            StatusCode::BAD_REQUEST,
            "TERMINAL_APP_ARCHIVED",
            "terminal app must be active",
        ),
        TerminalError::LocalDirectoryUnavailable => (
            StatusCode::BAD_REQUEST,
            "TERMINAL_LOCAL_DIRECTORY_INVALID",
            "terminal app does not define a local directory",
        ),
        TerminalError::WorkingDirectoryUnavailable => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "TERMINAL_WORKING_DIRECTORY_UNAVAILABLE",
            "terminal working directory is unavailable",
        ),
        TerminalError::WorkspaceArchived => (
            StatusCode::BAD_REQUEST,
            "TERMINAL_WORKSPACE_ARCHIVED",
            "terminal workspace must be active",
        ),
        TerminalError::WorkspaceNotFound => (
            StatusCode::NOT_FOUND,
            "TERMINAL_WORKSPACE_NOT_FOUND",
            "terminal workspace was not found",
        ),
        TerminalError::TicketInvalid | TerminalError::TicketExpired => (
            StatusCode::UNAUTHORIZED,
            "TERMINAL_TICKET_INVALID",
            "terminal socket ticket is invalid or expired",
        ),
        TerminalError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "TERMINAL_INTERNAL_ERROR",
            "terminal operation failed",
        ),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
