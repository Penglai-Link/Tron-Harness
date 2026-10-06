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
        AgentInvocationError::SessionBusy(_) => (
            StatusCode::CONFLICT,
            "RUN_ALREADY_ACTIVE",
            "session already has an active agent run".to_string(),
        ),
        AgentInvocationError::RunStopped => (
            StatusCode::CONFLICT,
            "AGENT_RUN_STOPPED",
            "agent run was stopped".to_string(),
        ),
        AgentInvocationError::RuntimeFailed(message) => {
            (StatusCode::BAD_REQUEST, "AGENT_RUNTIME_FAILED", message)
        }
        error @ AgentInvocationError::RuntimeVariableResolutionFailed(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error.code(),
            error.message(),
        ),
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
        AgentInvocationError::EmptyFinalOutput(message) => {
            (StatusCode::BAD_GATEWAY, "AGENT_EMPTY_FINAL_OUTPUT", message)
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

pub fn agent_agui_error_response(error: AgentAguiStreamError, locale: &str) -> Response {
    let (status, code, message) = match error {
        AgentAguiStreamError::StreamKeyRequired => (
            StatusCode::BAD_REQUEST,
            "AGENT_AGUI_STREAM_KEY_REQUIRED",
            "stream key is required".to_string(),
        ),
        AgentAguiStreamError::ReplayReadFailed(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "AGENT_AGUI_REPLAY_READ_FAILED",
            "agent presentation replay is unavailable".to_string(),
        ),
        AgentAguiStreamError::ReplayWriteFailed(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "AGENT_AGUI_REPLAY_WRITE_FAILED",
            "agent presentation replay could not be saved".to_string(),
        ),
        AgentAguiStreamError::ReplayIncomplete(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "AGENT_AGUI_REPLAY_INCOMPLETE",
            "agent presentation replay is incomplete".to_string(),
        ),
    };
    let message = if locale.starts_with("zh") {
        match code {
            "AGENT_AGUI_STREAM_KEY_REQUIRED" => "流标识不能为空",
            "AGENT_AGUI_REPLAY_READ_FAILED" => "展示回放暂时无法读取",
            "AGENT_AGUI_REPLAY_WRITE_FAILED" => "展示回放未能保存",
            _ => "展示回放不完整，执行结果会单独报告",
        }
        .to_string()
    } else {
        message
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

#[cfg(test)]
#[path = "agui_error_tests.rs"]
mod agui_error_tests;
