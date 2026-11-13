use super::{
    error_mapper::project_error_response,
    request::{
        ArchiveProjectRequest, DeleteProjectRequest, ProjectCodeRequest, ProjectWorkspaceRequest,
        SaveProjectRequest,
    },
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
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveProjectResponse {
    session_codes: Vec<String>,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteProjectResponse {
    session_codes: Vec<String>,
}

pub async fn archive_project(
    State(state): State<AppState>,
    Json(request): Json<ArchiveProjectRequest>,
) -> Response {
    match tokio::task::spawn_blocking(move || {
        state
            .projects
            .mutate_container(&request.project_code, &state.chats, false)
    })
    .await
    {
        Ok(Ok(session_codes)) => Json(ApiResponse::success(ArchiveProjectResponse {
            session_codes,
        }))
        .into_response(),
        Ok(Err(error)) => container_error_response(error),
        Err(_) => project_error_response(crate::domain::project::ProjectError::StorageFailure),
    }
}

pub async fn delete_project(
    State(state): State<AppState>,
    Json(request): Json<DeleteProjectRequest>,
) -> Response {
    match tokio::task::spawn_blocking(move || {
        state
            .projects
            .mutate_container(&request.project_code, &state.chats, true)
    })
    .await
    {
        Ok(Ok(session_codes)) => Json(ApiResponse::success(DeleteProjectResponse {
            session_codes,
        }))
        .into_response(),
        Ok(Err(error)) => container_error_response(error),
        Err(_) => project_error_response(crate::domain::project::ProjectError::StorageFailure),
    }
}

fn container_error_response(error: crate::application::project::ProjectContainerError) -> Response {
    match error {
        crate::application::project::ProjectContainerError::Project(error) => {
            project_error_response(error)
        }
        crate::application::project::ProjectContainerError::Chat(error) => {
            crate::adapters::inbound::http::chats::error_mapper::chat_error_response(error)
        }
    }
}
