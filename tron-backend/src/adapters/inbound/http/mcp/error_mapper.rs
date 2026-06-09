use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::mcp::McpError, response::ApiResponse};

pub fn mcp_error_response(error: McpError) -> Response {
    let (status, code, message) = match error {
        McpError::WorkspaceRequired => (
            StatusCode::BAD_REQUEST,
            "MCP_WORKSPACE_INVALID",
            "workspaceCode is required",
        ),
        McpError::VerifyContextRequired => (
            StatusCode::BAD_REQUEST,
            "MCP_VERIFY_CONTEXT_REQUIRED",
            "workspaceCode is required to resolve variable references",
        ),
        McpError::InvalidWorkspace => (
            StatusCode::BAD_REQUEST,
            "MCP_WORKSPACE_INVALID",
            "workspace must reference an active workspace",
        ),
        McpError::ServerNotFound => (
            StatusCode::NOT_FOUND,
            "MCP_SERVER_NOT_FOUND",
            "MCP server was not found",
        ),
        McpError::DuplicateServerCode => (
            StatusCode::CONFLICT,
            "MCP_SERVER_CODE_DUPLICATE",
            "MCP server code already exists",
        ),
        McpError::InvalidServerCode => (
            StatusCode::BAD_REQUEST,
            "MCP_SERVER_INVALID",
            "serverCode must use lowercase letters, digits, underscore, or hyphen",
        ),
        McpError::InvalidServerName => (
            StatusCode::BAD_REQUEST,
            "MCP_SERVER_INVALID",
            "name is required",
        ),
        McpError::InvalidServerUrl => (
            StatusCode::BAD_REQUEST,
            "MCP_SERVER_INVALID",
            "url must be a valid http or https URL",
        ),
        McpError::InvalidServerDefinition => (
            StatusCode::BAD_REQUEST,
            "MCP_SERVER_INVALID",
            "MCP server definition is invalid",
        ),
        McpError::EffectiveEnableDenied => (
            StatusCode::CONFLICT,
            "MCP_EFFECTIVE_ENABLE_DENIED",
            "platform-disabled server cannot be effectively enabled in a workspace",
        ),
        McpError::GatewayDisabled => (
            StatusCode::SERVICE_UNAVAILABLE,
            "MCP_GATEWAY_DISABLED",
            "MCP gateway is disabled",
        ),
        McpError::VerificationFailed(_) => (
            StatusCode::BAD_GATEWAY,
            "MCP_VERIFICATION_FAILED",
            "MCP verification failed",
        ),
        McpError::RuntimeDiscoveryFailed(_) => (
            StatusCode::BAD_GATEWAY,
            "MCP_RUNTIME_DISCOVERY_FAILED",
            "MCP runtime discovery failed",
        ),
        McpError::TransportUnauthorized => (
            StatusCode::UNAUTHORIZED,
            "MCP_TRANSPORT_UNAUTHORIZED",
            "transport bearer authorization is required",
        ),
        McpError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "MCP_STORAGE_FAILED",
            "MCP storage operation failed",
        ),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
