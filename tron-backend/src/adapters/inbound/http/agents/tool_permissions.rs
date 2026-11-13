use super::internal_definitions::InternalAgentApiError;
use crate::{
    adapters::inbound::http::auth::RequestAuthContext, app::AppState,
    domain::agent::AgentManagementError, response::ApiResponse,
};
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
};
use serde::{Deserialize, Serialize};
use tron_harness_capability::{ToolPermission, ToolPermissionSetting, ToolRisk};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplaceToolPermissionsRequest {
    permissions: Vec<ToolPermissionRequest>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ToolPermissionRequest {
    tool_code: String,
    permission: ToolPermission,
    confirm_level: ToolRisk,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolPermissionView {
    tool_code: String,
    permission: ToolPermission,
    confirm_level: ToolRisk,
}
#[derive(Serialize)]
pub struct ToolPermissionsView {
    permissions: Vec<ToolPermissionView>,
}
#[derive(Serialize)]
pub struct ReplacedToolPermissionsView {
    permissions: Vec<ToolPermissionView>,
}
fn views(settings: Vec<ToolPermissionSetting>) -> Vec<ToolPermissionView> {
    settings
        .into_iter()
        .map(|s| ToolPermissionView {
            tool_code: s.tool_code,
            permission: s.permission,
            confirm_level: s.confirm_level,
        })
        .collect()
}
pub async fn list(
    State(state): State<AppState>,
    auth: RequestAuthContext,
) -> Result<Json<ApiResponse<ToolPermissionsView>>, InternalAgentApiError> {
    if let Err(error) = state.agent_framework.ensure_capability_catalog().await {
        tracing::warn!(%error, "Serving known tool permissions after metadata loading failed");
    }
    let settings = state
        .agent_management
        .tool_permissions()
        .map_err(|e| InternalAgentApiError(e, auth.locale))?;
    Ok(Json(ApiResponse::success(ToolPermissionsView {
        permissions: views(settings),
    })))
}
pub async fn replace(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    request: Result<Json<ReplaceToolPermissionsRequest>, JsonRejection>,
) -> Result<Json<ApiResponse<ReplacedToolPermissionsView>>, InternalAgentApiError> {
    let Json(request) = request.map_err(|_| {
        InternalAgentApiError(
            AgentManagementError::ToolPermissionInvalid,
            auth.locale.clone(),
        )
    })?;
    let settings = request
        .permissions
        .into_iter()
        .map(|r| ToolPermissionSetting {
            tool_code: r.tool_code,
            permission: r.permission,
            confirm_level: r.confirm_level,
        })
        .collect();
    let settings = state
        .agent_management
        .replace_tool_permissions(settings)
        .map_err(|e| InternalAgentApiError(e, auth.locale))?;
    Ok(Json(ApiResponse::success(ReplacedToolPermissionsView {
        permissions: views(settings),
    })))
}
