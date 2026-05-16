use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    application::workspace::WorkspaceService,
    domain::workspace::{WorkspaceAccessListQuery, WorkspaceListQuery, WorkspaceStatistics},
    response::ApiResponse,
};

use super::{
    error_mapper::workspace_error_response,
    request::{
        CreateWorkspaceRequest, MoveWorkspaceRequest, ReplaceWorkspaceRolesRequest,
        ReplaceWorkspaceUsersRequest, UpdateWorkspaceRequest,
    },
};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CountDto {
    count: usize,
}

pub async fn list_workspaces(
    State(workspaces): State<WorkspaceService>,
    Query(query): Query<WorkspaceListQuery>,
) -> Response {
    match workspaces.list_workspaces(query) {
        Ok(items) => Json(ApiResponse::success(items)).into_response(),
        Err(error) => workspace_error_response(error),
    }
}

pub async fn total_workspaces(State(workspaces): State<WorkspaceService>) -> Response {
    statistics_response(workspaces, |statistics| statistics.total_workspaces)
}

pub async fn active_workspaces(State(workspaces): State<WorkspaceService>) -> Response {
    statistics_response(workspaces, |statistics| statistics.active_workspaces)
}

pub async fn direct_user_grants(State(workspaces): State<WorkspaceService>) -> Response {
    statistics_response(workspaces, |statistics| statistics.direct_user_grants)
}

pub async fn role_grants(State(workspaces): State<WorkspaceService>) -> Response {
    statistics_response(workspaces, |statistics| statistics.role_grants)
}

fn statistics_response(
    workspaces: WorkspaceService,
    select_count: impl FnOnce(WorkspaceStatistics) -> usize,
) -> Response {
    match workspaces.statistics() {
        Ok(statistics) => Json(ApiResponse::success(CountDto {
            count: select_count(statistics),
        }))
        .into_response(),
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

pub async fn list_user_access(
    State(workspaces): State<WorkspaceService>,
    Path(workspace_code): Path<String>,
    Query(query): Query<WorkspaceAccessListQuery>,
) -> Response {
    match workspaces.list_user_access(&workspace_code, query) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => workspace_error_response(error),
    }
}

pub async fn replace_user_access(
    State(workspaces): State<WorkspaceService>,
    auth: RequestAuthContext,
    Path(workspace_code): Path<String>,
    Json(request): Json<ReplaceWorkspaceUsersRequest>,
) -> Response {
    match workspaces.replace_user_access(auth.user(), &workspace_code, request.into()) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => workspace_error_response(error),
    }
}

pub async fn list_role_access(
    State(workspaces): State<WorkspaceService>,
    Path(workspace_code): Path<String>,
) -> Response {
    match workspaces.list_role_access(&workspace_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => workspace_error_response(error),
    }
}

pub async fn replace_role_access(
    State(workspaces): State<WorkspaceService>,
    auth: RequestAuthContext,
    Path(workspace_code): Path<String>,
    Json(request): Json<ReplaceWorkspaceRolesRequest>,
) -> Response {
    match workspaces.replace_role_access(auth.user(), &workspace_code, request.into()) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => workspace_error_response(error),
    }
}

pub async fn list_effective_users(
    State(workspaces): State<WorkspaceService>,
    Path(workspace_code): Path<String>,
    Query(query): Query<WorkspaceAccessListQuery>,
) -> Response {
    match workspaces.list_effective_users(&workspace_code, query) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => workspace_error_response(error),
    }
}
