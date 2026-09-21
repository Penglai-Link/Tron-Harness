use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    application::workspace::WorkspaceService,
    domain::workspace::WorkspaceListQuery,
    response::ApiResponse,
};

use super::{
    error_mapper::workspace_error_response,
    request::{
        CreateWorkspaceRequest, MoveWorkspaceRequest, UpdateWorkspaceRequest,
    },
};

pub async fn list_workspaces(
    State(workspaces): State<WorkspaceService>,
    Query(query): Query<WorkspaceListQuery>,
) -> Response {
    match workspaces.list_workspaces(query) {
        Ok(items) => Json(ApiResponse::success(items)).into_response(),
        Err(error) => workspace_error_response(error),
    }
}

pub async fn list_workspace_options(
    State(workspaces): State<WorkspaceService>,
    Query(query): Query<WorkspaceListQuery>,
) -> Response {
    match workspaces.list_workspace_options(query) {
        Ok(items) => Json(ApiResponse::success(items)).into_response(),
        Err(error) => workspace_error_response(error),
    }
}

pub async fn workspace_switcher(State(workspaces): State<WorkspaceService>) -> Response {
    match workspaces.workspace_switcher() {
        Ok(data) => Json(ApiResponse::success(data)).into_response(),
        Err(error) => workspace_error_response(error),
    }
}

pub async fn create_workspace(
    State(workspaces): State<WorkspaceService>,
    auth: RequestAuthContext,
    Json(request): Json<CreateWorkspaceRequest>,
) -> Response {
    match workspaces.create_workspace(auth.user(), request.into()) {
        Ok(detail) => (StatusCode::CREATED, Json(ApiResponse::success(detail))).into_response(),
        Err(error) => workspace_error_response(error),
    }
}

pub async fn get_workspace(
    State(workspaces): State<WorkspaceService>,
    Path(workspace_code): Path<String>,
) -> Response {
    match workspaces.get_workspace(&workspace_code) {
        Ok(detail) => Json(ApiResponse::success(detail)).into_response(),
        Err(error) => workspace_error_response(error),
    }
}

pub async fn patch_workspace(
    State(workspaces): State<WorkspaceService>,
    auth: RequestAuthContext,
    Path(workspace_code): Path<String>,
    Json(request): Json<UpdateWorkspaceRequest>,
) -> Response {
    match workspaces.update_workspace(auth.user(), &workspace_code, request.into()) {
        Ok(detail) => Json(ApiResponse::success(detail)).into_response(),
        Err(error) => workspace_error_response(error),
    }
}

pub async fn move_workspace(
    State(workspaces): State<WorkspaceService>,
    auth: RequestAuthContext,
    Path(workspace_code): Path<String>,
    Json(request): Json<MoveWorkspaceRequest>,
) -> Response {
    match workspaces.move_workspace(auth.user(), &workspace_code, request.into()) {
        Ok(detail) => Json(ApiResponse::success(detail)).into_response(),
        Err(error) => workspace_error_response(error),
    }
}

pub async fn archive_workspace(
    State(workspaces): State<WorkspaceService>,
    auth: RequestAuthContext,
    Path(workspace_code): Path<String>,
) -> Response {
    match workspaces.archive_workspace(auth.user(), &workspace_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => workspace_error_response(error),
    }
}

pub async fn restore_workspace(
    State(workspaces): State<WorkspaceService>,
    auth: RequestAuthContext,
    Path(workspace_code): Path<String>,
) -> Response {
    match workspaces.restore_workspace(auth.user(), &workspace_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => workspace_error_response(error),
    }
}
