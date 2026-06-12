use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    application::agent::{AgentAguiStreamError, AgentInvocationError},
    domain::agent::{AgentManagementError, AgentRunHistoryError},
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

pub fn agent_management_error_response(error: AgentManagementError) -> Response {
    let (status, code, message) = match error {
        AgentManagementError::InvalidWorkspace => (
            StatusCode::BAD_REQUEST,
            "AGENT_WORKSPACE_INVALID",
            "workspace must reference an active workspace".to_string(),
        ),
        AgentManagementError::InternalAgentNotFound => (
            StatusCode::NOT_FOUND,
            "AGENT_INTERNAL_NOT_FOUND",
            "internal agent was not found".to_string(),
        ),
        AgentManagementError::SystemAgentNotFound => (
            StatusCode::NOT_FOUND,
            "AGENT_SYSTEM_NOT_FOUND",
            "system agent was not found".to_string(),
        ),
        AgentManagementError::SystemAgentUnavailable => (
            StatusCode::BAD_REQUEST,
            "AGENT_SYSTEM_UNAVAILABLE",
            "system agent is unavailable on the current machine".to_string(),
        ),
        AgentManagementError::InternalToolAssignmentStale(message) => (
            StatusCode::BAD_REQUEST,
            "AGENT_INTERNAL_TOOL_ASSIGNMENT_STALE",
            format!("internal tool assignment is stale: {message}"),
        ),
        AgentManagementError::InternalMcpServerAssignmentStale(message) => (
            StatusCode::BAD_REQUEST,
            "AGENT_INTERNAL_MCP_SERVER_ASSIGNMENT_STALE",
            format!("internal MCP server assignment is stale: {message}"),
        ),
        AgentManagementError::InternalToolUnknown(message) => (
            StatusCode::BAD_REQUEST,
            "AGENT_INTERNAL_TOOL_UNKNOWN",
            format!("unknown internal tool: {message}"),
        ),
        AgentManagementError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "AGENT_MANAGEMENT_STORAGE_FAILED",
            "agent management storage operation failed".to_string(),
        ),
    };
    (status, Json(ApiResponse::error(code, message))).into_response()
}
