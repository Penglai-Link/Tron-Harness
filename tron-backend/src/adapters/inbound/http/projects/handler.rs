use super::{
    error_mapper::project_error_response,
    request::{ProjectCodeRequest, ProjectWorkspaceRequest, SaveProjectRequest},
};
use crate::{app::AppState, application::project::SaveProjectCommand, response::ApiResponse};
use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};

pub async fn list_projects(
    State(state): State<AppState>,
    Json(request): Json<ProjectWorkspaceRequest>,
) -> Response {
    match state.projects.list(&request.workspace_code) {
        Ok(data) => Json(ApiResponse::success(data)).into_response(),
        Err(error) => project_error_response(error),
    }
}
pub async fn get_project(
    State(state): State<AppState>,
    Json(request): Json<ProjectCodeRequest>,
) -> Response {
    match state.projects.get(&request.project_code) {
        Ok(data) => Json(ApiResponse::success(data)).into_response(),
        Err(error) => project_error_response(error),
    }
}
pub async fn save_project(
    State(state): State<AppState>,
    Json(request): Json<SaveProjectRequest>,
) -> Response {
    match state.projects.save(SaveProjectCommand {
        project_code: request.project_code,
        workspace_code: request.workspace_code,
        name: request.name,
        app_codes: request.app_codes,
        primary_app_code: request.primary_app_code,
    }) {
        Ok(data) => Json(ApiResponse::success(data)).into_response(),
        Err(error) => project_error_response(error),
    }
}
pub async fn archive_project(
    State(state): State<AppState>,
    Json(request): Json<ProjectCodeRequest>,
) -> Response {
    match state.projects.archive(&request.project_code) {
        Ok(()) => Json(ApiResponse::success(serde_json::json!({}))).into_response(),
        Err(error) => project_error_response(error),
    }
}
