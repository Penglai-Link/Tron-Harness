use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    application::{agent::AgentInvocationError, chat::ChatSendError},
    domain::chat::ChatError,
    response::ApiResponse,
};

pub fn chat_error_response(error: ChatError) -> Response {
    let (status, code, message) = match error {
        ChatError::MessageRequired => (
            StatusCode::BAD_REQUEST,
            "CHAT_MESSAGE_REQUIRED",
            "message content is required".to_string(),
        ),
        ChatError::SessionNotFound => (
            StatusCode::NOT_FOUND,
            "CHAT_SESSION_NOT_FOUND",
            "chat session was not found".to_string(),
        ),
        ChatError::SessionTitleRequired => (
            StatusCode::BAD_REQUEST,
            "CHAT_SESSION_TITLE_REQUIRED",
            "session title is required".to_string(),
        ),
        ChatError::WorkspaceRequired => (
            StatusCode::BAD_REQUEST,
            "CHAT_WORKSPACE_REQUIRED",
            "workspaceCode is required".to_string(),
        ),
        ChatError::WorkspaceForbidden => (
            StatusCode::FORBIDDEN,
            "CHAT_WORKSPACE_FORBIDDEN",
            "workspace access is forbidden".to_string(),
        ),
        ChatError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "CHAT_STORAGE_FAILED",
            "chat storage failed".to_string(),
        ),
    };
    (status, Json(ApiResponse::error(code, message))).into_response()
}

pub fn chat_send_error_response(error: ChatSendError) -> Response {
    match error {
        ChatSendError::Chat(error) => chat_error_response(error),
        ChatSendError::Agent(error) => agent_error_response(error),
    }
}

fn agent_error_response(error: AgentInvocationError) -> Response {
    let (status, code, message) = match error {
        AgentInvocationError::PromptRequired => (
            StatusCode::BAD_REQUEST,
            "AGENT_PROMPT_REQUIRED",
            "prompt is required".to_string(),
        ),
        AgentInvocationError::WorkspaceRequired => (
            StatusCode::BAD_REQUEST,
            "AGENT_WORKSPACE_REQUIRED",
            "workspaceCode is required".to_string(),
        ),
        AgentInvocationError::WorkspaceInvalid => (
            StatusCode::BAD_REQUEST,
            "AGENT_WORKSPACE_INVALID",
            "workspace must reference an active workspace".to_string(),
        ),
        AgentInvocationError::ModelRequired => (
            StatusCode::BAD_REQUEST,
            "AGENT_MODEL_REQUIRED",
            "model config is required".to_string(),
        ),
        AgentInvocationError::HistoryStorageFailed => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "AGENT_HISTORY_STORAGE_FAILED",
            "agent run history storage failed".to_string(),
        ),
        AgentInvocationError::RuntimeFailed(message) => {
            (StatusCode::BAD_REQUEST, "AGENT_RUNTIME_FAILED", message)
        }
        AgentInvocationError::RuntimeSecretDeliveryUntrusted => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_RUNTIME_SECRET_DELIVERY_UNTRUSTED",
            "Sensitive runtime variable delivery is not trusted".to_string(),
        ),
        AgentInvocationError::ModelFailed(message) => {
            (StatusCode::BAD_GATEWAY, "AGENT_MODEL_FAILED", message)
        }
        AgentInvocationError::ToolFailed(message) => {
            (StatusCode::BAD_GATEWAY, "AGENT_TOOL_FAILED", message)
        }
        AgentInvocationError::MaxStepsExceeded => (
            StatusCode::BAD_REQUEST,
            "AGENT_MAX_STEPS_EXCEEDED",
            "agent run exceeded max steps".to_string(),
        ),
        AgentInvocationError::MaxToolCallsExceeded => (
            StatusCode::BAD_REQUEST,
            "AGENT_MAX_TOOL_CALLS_EXCEEDED",
            "agent run exceeded max tool calls".to_string(),
        ),
    };
    (status, Json(ApiResponse::error(code, message))).into_response()
}
