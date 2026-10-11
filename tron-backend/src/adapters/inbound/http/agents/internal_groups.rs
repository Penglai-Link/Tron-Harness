use super::internal_definitions::InternalAgentApiError;
use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    response::{ApiResponse, EmptyData},
};
use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateGroupRequest {
    name: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteGroupRequest {
    name: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGroupResponse {
    name: String,
}

pub async fn list_platform(
    State(state): State<AppState>,
    auth: RequestAuthContext,
) -> Result<Json<ApiResponse<Vec<String>>>, InternalAgentApiError> {
    state
        .agent_management
        .list_internal_agent_groups("")
        .map(|names| Json(ApiResponse::success(names)))
        .map_err(|e| InternalAgentApiError(e, auth.locale))
}
pub async fn create_platform(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Json(request): Json<CreateGroupRequest>,
) -> Result<Json<ApiResponse<CreateGroupResponse>>, InternalAgentApiError> {
    state
        .agent_management
        .create_internal_agent_group("", &request.name)
        .map(|name| Json(ApiResponse::success(CreateGroupResponse { name })))
        .map_err(|e| InternalAgentApiError(e, auth.locale))
}
pub async fn delete_platform(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Json(request): Json<DeleteGroupRequest>,
) -> Result<Json<ApiResponse<EmptyData>>, InternalAgentApiError> {
    state
        .agent_management
        .delete_internal_agent_group("", &request.name)
        .map(|()| Json(ApiResponse::success(EmptyData {})))
        .map_err(|e| InternalAgentApiError(e, auth.locale))
}
pub async fn list_workspace(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(workspace): Path<String>,
) -> Result<Json<ApiResponse<Vec<String>>>, InternalAgentApiError> {
    state
        .agent_management
        .list_internal_agent_groups(&workspace)
        .map(|names| Json(ApiResponse::success(names)))
        .map_err(|e| InternalAgentApiError(e, auth.locale))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateWorkspaceGroupRequest {
    name: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceGroupResponse {
    name: String,
}
pub async fn create_workspace(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(workspace): Path<String>,
    Json(request): Json<CreateWorkspaceGroupRequest>,
) -> Result<Json<ApiResponse<CreateWorkspaceGroupResponse>>, InternalAgentApiError> {
    state
        .agent_management
        .create_internal_agent_group(&workspace, &request.name)
        .map(|name| Json(ApiResponse::success(CreateWorkspaceGroupResponse { name })))
        .map_err(|e| InternalAgentApiError(e, auth.locale))
}
pub async fn delete_workspace(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(workspace): Path<String>,
    Json(request): Json<DeleteGroupRequest>,
) -> Result<Json<ApiResponse<EmptyData>>, InternalAgentApiError> {
    state
        .agent_management
        .delete_internal_agent_group(&workspace, &request.name)
        .map(|()| Json(ApiResponse::success(EmptyData {})))
        .map_err(|e| InternalAgentApiError(e, auth.locale))
}
