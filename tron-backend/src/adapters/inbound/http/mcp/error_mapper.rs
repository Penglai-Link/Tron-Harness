use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::mcp::McpError, response::ApiResponse};

pub fn mcp_error_response(error: McpError) -> Response {
    use tron_i18n::t;
    // MCP handlers run inside the authenticated request scope, which carries the
    // resolved request locale; fall back to the documented zh-CN default.
    let locale = crate::shared::auth_holder::AuthHolder::get()
        .map(|context| context.locale.clone())
        .unwrap_or_else(|| tron_i18n::DEFAULT_LOCALE.as_str().to_owned());
    let (status, code) = match &error {
        McpError::WorkspaceRequired => (StatusCode::BAD_REQUEST, "MCP_WORKSPACE_INVALID"),
        McpError::VerifyContextRequired => (StatusCode::BAD_REQUEST, "MCP_VERIFY_CONTEXT_REQUIRED"),
        McpError::InvalidWorkspace => (StatusCode::BAD_REQUEST, "MCP_WORKSPACE_INVALID"),
        McpError::ServerNotFound => (StatusCode::NOT_FOUND, "MCP_SERVER_NOT_FOUND"),
        McpError::DuplicateServerCode => (StatusCode::CONFLICT, "MCP_SERVER_CODE_DUPLICATE"),
        McpError::InvalidServerCode => (StatusCode::BAD_REQUEST, "MCP_SERVER_INVALID"),
        McpError::InvalidServerName => (StatusCode::BAD_REQUEST, "MCP_SERVER_INVALID"),
        McpError::InvalidServerUrl => (StatusCode::BAD_REQUEST, "MCP_SERVER_INVALID"),
        McpError::InvalidServerDefinition => (StatusCode::BAD_REQUEST, "MCP_SERVER_INVALID"),
        McpError::EffectiveEnableDenied => (StatusCode::CONFLICT, "MCP_EFFECTIVE_ENABLE_DENIED"),
        McpError::GatewayDisabled => (StatusCode::SERVICE_UNAVAILABLE, "MCP_GATEWAY_DISABLED"),
        McpError::VerificationFailed(_) => (StatusCode::BAD_GATEWAY, "MCP_VERIFICATION_FAILED"),
        McpError::RuntimeDiscoveryFailed(_) => {
            (StatusCode::BAD_GATEWAY, "MCP_RUNTIME_DISCOVERY_FAILED")
        }
        McpError::TransportUnauthorized => (StatusCode::UNAUTHORIZED, "MCP_TRANSPORT_UNAUTHORIZED"),
        McpError::StorageFailure => (StatusCode::INTERNAL_SERVER_ERROR, "MCP_STORAGE_FAILED"),
    };
    let message = match &error {
        McpError::WorkspaceRequired => {
            t!("tron.mcp.workspace_required", locale = &locale)
        }
        McpError::VerifyContextRequired => {
            t!("tron.mcp.verify_context_required", locale = &locale)
        }
        McpError::InvalidWorkspace => {
            t!("tron.mcp.invalid_workspace", locale = &locale)
        }
        McpError::ServerNotFound => t!("tron.mcp.server_not_found", locale = &locale),
        McpError::DuplicateServerCode => {
            t!("tron.mcp.server_code_duplicate", locale = &locale)
        }
        McpError::InvalidServerCode => {
            t!("tron.mcp.invalid_server_code", locale = &locale)
        }
        McpError::InvalidServerName => {
            t!("tron.mcp.invalid_server_name", locale = &locale)
        }
        McpError::InvalidServerUrl => t!("tron.mcp.invalid_server_url", locale = &locale),
        McpError::InvalidServerDefinition => {
            t!("tron.mcp.invalid_definition", locale = &locale)
        }
        McpError::EffectiveEnableDenied => {
            t!("tron.mcp.effective_enable_denied", locale = &locale)
        }
        McpError::GatewayDisabled => t!("tron.mcp.gateway_disabled", locale = &locale),
        McpError::VerificationFailed(_) => {
            t!("tron.mcp.verification_failed", locale = &locale)
        }
        McpError::RuntimeDiscoveryFailed(_) => {
            t!("tron.mcp.runtime_discovery_failed", locale = &locale)
        }
        McpError::TransportUnauthorized => {
            t!("tron.mcp.transport_unauthorized", locale = &locale)
        }
        McpError::StorageFailure => t!("tron.mcp.storage_failed", locale = &locale),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
