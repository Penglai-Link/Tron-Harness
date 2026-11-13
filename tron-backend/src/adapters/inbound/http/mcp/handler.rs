mod transport;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    application::mcp::McpService,
    domain::mcp::{PlatformMcpServerListQuery, WorkspaceMcpServerListQuery},
    response::ApiResponse,
};

use super::{
    error_mapper::mcp_error_response,
    request::{
        CreatePlatformMcpServerRequest, UpdatePlatformMcpGatewayRequest,
        UpdatePlatformMcpServerRequest, VerifyPlatformMcpServerDraftRequest,
        VerifyPlatformMcpServerRequest,
    },
};

pub use transport::variables_transport;

pub async fn get_gateway(State(mcp): State<McpService>) -> Response {
    match mcp.get_gateway_config() {
        Ok(config) => Json(ApiResponse::success(config)).into_response(),
        Err(error) => mcp_error_response(error),
    }
}

pub async fn patch_gateway(
    State(mcp): State<McpService>,
    Json(request): Json<UpdatePlatformMcpGatewayRequest>,
) -> Response {
    match mcp.update_gateway_config(request.into()) {
        Ok(config) => Json(ApiResponse::success(config)).into_response(),
        Err(error) => mcp_error_response(error),
    }
}

pub async fn list_platform_servers(
    auth: crate::adapters::inbound::http::auth::RequestAuthContext,
    State(mcp): State<McpService>,
    Query(query): Query<PlatformMcpServerListQuery>,
) -> Response {
    match mcp.list_platform_servers(query) {
        Ok(records) => Json(ApiResponse::protocol_success(
            records
                .into_iter()
                .map(|server| {
                    let owner = mcp.capability_resource(&server.server_code);
                    PlatformMcpAssignmentVo {
                        provider_pack_id: owner.as_ref().map(|o| o.pack_id.clone()),
                        assignable: server.enabled,
                        disabled_reason: owner.filter(|o| !o.available).map(|o| {
                            if auth.locale.starts_with("zh") {
                                format!("能力包 {} 已禁用", o.pack_id)
                            } else {
                                format!("Capability pack {} is disabled", o.pack_id)
                            }
                        }),
                        server,
                    }
                })
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => mcp_error_response(error),
    }
}

pub async fn get_platform_server(
    State(mcp): State<McpService>,
    Path(server_code): Path<String>,
) -> Response {
    match mcp.get_platform_server(&server_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => mcp_error_response(error),
    }
}

pub async fn create_platform_server(
    State(mcp): State<McpService>,
    Json(request): Json<CreatePlatformMcpServerRequest>,
) -> Response {
    match mcp.create_platform_server(request.into()) {
        Ok(record) => (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response(),
        Err(error) => mcp_error_response(error),
    }
}

pub async fn verify_platform_server_draft(
    State(mcp): State<McpService>,
    Json(request): Json<VerifyPlatformMcpServerDraftRequest>,
) -> Response {
    match mcp.verify_platform_server_draft(request.into()).await {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => mcp_error_response(error),
    }
}

pub async fn patch_platform_server(
    State(mcp): State<McpService>,
    Path(server_code): Path<String>,
    Json(request): Json<UpdatePlatformMcpServerRequest>,
) -> Response {
    match mcp.update_platform_server(&server_code, request.into()) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => mcp_error_response(error),
    }
}

pub async fn verify_platform_server(
    State(mcp): State<McpService>,
    Path(server_code): Path<String>,
    Json(request): Json<VerifyPlatformMcpServerRequest>,
) -> Response {
    match mcp
        .verify_platform_server(&server_code, request.into())
        .await
    {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => mcp_error_response(error),
    }
}

pub async fn enable_platform_server(
    State(mcp): State<McpService>,
    Path(server_code): Path<String>,
) -> Response {
    match mcp.enable_platform_server(&server_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => mcp_error_response(error),
    }
}

pub async fn disable_platform_server(
    State(mcp): State<McpService>,
    Path(server_code): Path<String>,
) -> Response {
    match mcp.disable_platform_server(&server_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => mcp_error_response(error),
    }
}

pub async fn refresh_platform_server_health(
    State(mcp): State<McpService>,
    Path(server_code): Path<String>,
) -> Response {
    match mcp.refresh_platform_server_health(&server_code).await {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => mcp_error_response(error),
    }
}

pub async fn delete_platform_server(
    State(mcp): State<McpService>,
    Path(server_code): Path<String>,
) -> Response {
    match mcp.delete_platform_server(&server_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => mcp_error_response(error),
    }
}

pub async fn list_workspace_effective_servers(
    auth: crate::adapters::inbound::http::auth::RequestAuthContext,
    State(mcp): State<McpService>,
    Path(workspace_code): Path<String>,
    Query(query): Query<WorkspaceMcpServerListQuery>,
) -> Response {
    match mcp.list_workspace_effective_servers(&workspace_code, query) {
        Ok(records) => Json(ApiResponse::protocol_success(
            records
                .into_iter()
                .map(|server| {
                    let owner = mcp.capability_resource(&server.server_code);
                    WorkspaceMcpAssignmentVo {
                        provider_pack_id: owner.as_ref().map(|o| o.pack_id.clone()),
                        assignable: server.effective_enabled,
                        disabled_reason: owner.filter(|o| !o.available).map(|o| {
                            if auth.locale.starts_with("zh") {
                                format!("能力包 {} 已禁用", o.pack_id)
                            } else {
                                format!("Capability pack {} is disabled", o.pack_id)
                            }
                        }),
                        server,
                    }
                })
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => mcp_error_response(error),
    }
}

pub async fn enable_workspace_server(
    State(mcp): State<McpService>,
    Path((workspace_code, server_code)): Path<(String, String)>,
) -> Response {
    match mcp.enable_workspace_server(&workspace_code, &server_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => mcp_error_response(error),
    }
}

pub async fn disable_workspace_server(
    State(mcp): State<McpService>,
    Path((workspace_code, server_code)): Path<(String, String)>,
) -> Response {
    match mcp.disable_workspace_server(&workspace_code, &server_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => mcp_error_response(error),
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PlatformMcpAssignmentVo {
    #[serde(flatten)]
    server: crate::domain::mcp::PlatformMcpServerRecord,
    provider_pack_id: Option<String>,
    assignable: bool,
    disabled_reason: Option<String>,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceMcpAssignmentVo {
    #[serde(flatten)]
    server: crate::domain::mcp::WorkspaceEffectiveMcpServerRecord,
    provider_pack_id: Option<String>,
    assignable: bool,
    disabled_reason: Option<String>,
}
