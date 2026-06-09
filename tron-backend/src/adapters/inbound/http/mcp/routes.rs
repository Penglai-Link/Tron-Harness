use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{
    create_platform_server, delete_platform_server, disable_platform_server,
    disable_workspace_server, enable_platform_server, enable_workspace_server, get_gateway,
    get_platform_server, list_platform_servers, list_workspace_effective_servers, patch_gateway,
    patch_platform_server, refresh_platform_server_health, variables_transport,
    verify_platform_server, verify_platform_server_draft,
};

pub fn management_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/mcp/gateway", get(get_gateway).patch(patch_gateway))
        .route(
            "/api/v1/mcp/servers",
            get(list_platform_servers).post(create_platform_server),
        )
        .route(
            "/api/v1/mcp/servers/verify",
            post(verify_platform_server_draft),
        )
        .route(
            "/api/v1/mcp/servers/{server_code}",
            get(get_platform_server)
                .patch(patch_platform_server)
                .delete(delete_platform_server),
        )
        .route(
            "/api/v1/mcp/servers/{server_code}/verify",
            post(verify_platform_server),
        )
        .route(
            "/api/v1/mcp/servers/{server_code}/enable",
            post(enable_platform_server),
        )
        .route(
            "/api/v1/mcp/servers/{server_code}/disable",
            post(disable_platform_server),
        )
        .route(
            "/api/v1/mcp/servers/{server_code}/health",
            post(refresh_platform_server_health),
        )
        .route(
            "/api/v1/mcp/workspaces/{workspace_code}/servers",
            get(list_workspace_effective_servers),
        )
        .route(
            "/api/v1/mcp/workspaces/{workspace_code}/servers/{server_code}/enable",
            post(enable_workspace_server),
        )
        .route(
            "/api/v1/mcp/workspaces/{workspace_code}/servers/{server_code}/disable",
            post(disable_workspace_server),
        )
}

pub fn transport_routes() -> Router<AppState> {
    Router::new().route("/mcp/variables", post(variables_transport))
}
