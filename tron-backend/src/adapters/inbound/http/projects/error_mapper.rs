use axum::{Json, http::StatusCode, response::{IntoResponse, Response}};
use crate::{domain::project::ProjectError, response::ApiResponse};

pub fn project_error_response(error: ProjectError) -> Response {
    let (status, code, message) = match error {
        ProjectError::NotFound => (StatusCode::NOT_FOUND, "PROJECT_NOT_FOUND", "project was not found"),
        ProjectError::ProjectCodeInvalid => (StatusCode::BAD_REQUEST, "PROJECT_CODE_INVALID", "projectCode is invalid"),
        ProjectError::NameRequired => (StatusCode::BAD_REQUEST, "PROJECT_NAME_REQUIRED", "project name is required"),
        ProjectError::DuplicateApp => (StatusCode::BAD_REQUEST, "PROJECT_APP_DUPLICATE", "project cannot include an app more than once"),
        ProjectError::DuplicateName => (StatusCode::CONFLICT, "PROJECT_NAME_DUPLICATE", "project name already exists in the workspace"),
        ProjectError::PrimaryAppInvalid => (StatusCode::BAD_REQUEST, "PROJECT_PRIMARY_APP_INVALID", "primaryAppCode must be one of the selected apps"),
        ProjectError::AppInvalid => (StatusCode::BAD_REQUEST, "PROJECT_APP_INVALID", "project app must be active and in the current workspace"),
        ProjectError::WorkspaceInvalid => (StatusCode::BAD_REQUEST, "PROJECT_WORKSPACE_INVALID", "project workspace must be active"),
        ProjectError::Archived => (StatusCode::CONFLICT, "PROJECT_ARCHIVED", "archived project cannot be used"),
        ProjectError::StorageFailure => (StatusCode::INTERNAL_SERVER_ERROR, "PROJECT_STORAGE_FAILED", "project storage operation failed"),
    };
    (status, Json(ApiResponse::error(code, message))).into_response()
}
