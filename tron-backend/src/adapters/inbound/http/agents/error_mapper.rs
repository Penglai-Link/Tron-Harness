use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    application::agent::{AgentAguiStreamError, AgentInvocationError},
    domain::agent::AgentRunHistoryError,
    response::ApiResponse,
};

pub fn agent_error_response(error: AgentInvocationError) -> Response {
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
    };
    (status, Json(ApiResponse::error(code, message))).into_response()
}

pub fn agent_history_error_response(error: AgentRunHistoryError) -> Response {
    let (status, code, message) = match error {
        AgentRunHistoryError::RunNotFound => (
            StatusCode::NOT_FOUND,
            "AGENT_RUN_NOT_FOUND",
            "agent run was not found".to_string(),
        ),
        AgentRunHistoryError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "AGENT_HISTORY_STORAGE_FAILED",
            "agent run history storage failed".to_string(),
        ),
    };
    (status, Json(ApiResponse::error(code, message))).into_response()
}

pub fn agent_agui_error_response(error: AgentAguiStreamError) -> Response {
    let (status, code, message) = match error {
        AgentAguiStreamError::StreamKeyRequired => (
            StatusCode::BAD_REQUEST,
            "AGENT_AGUI_STREAM_KEY_REQUIRED",
            "stream key is required".to_string(),
        ),
    };
    (status, Json(ApiResponse::error(code, message))).into_response()
}
