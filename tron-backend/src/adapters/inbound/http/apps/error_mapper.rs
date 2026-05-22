use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::app::AppError, response::ApiResponse};

pub fn app_error_response(error: AppError) -> Response {
    let (status, code, message) = match error {
        AppError::NotFound => (StatusCode::NOT_FOUND, "APP_NOT_FOUND", "app was not found"),
        AppError::InvalidAppCode => (
            StatusCode::BAD_REQUEST,
            "APP_CODE_INVALID",
            "appCode is invalid",
        ),
        AppError::WorkspaceRequired => (
            StatusCode::BAD_REQUEST,
            "APP_WORKSPACE_REQUIRED",
            "workspaceCode is required",
        ),
        AppError::InvalidWorkspace => (
            StatusCode::BAD_REQUEST,
            "APP_WORKSPACE_INVALID",
            "app workspace must reference an active workspace",
        ),
        AppError::NameRequired => (
            StatusCode::BAD_REQUEST,
            "APP_NAME_REQUIRED",
            "app name is required",
        ),
        AppError::LocalDirectoryRequired => (
            StatusCode::BAD_REQUEST,
            "APP_LOCAL_DIRECTORY_REQUIRED",
            "localDirectory is required",
        ),
        AppError::DuplicateName => (
            StatusCode::CONFLICT,
            "APP_NAME_DUPLICATE",
            "app name already exists in the current workspace",
        ),
        AppError::ArchivedMutationDenied => (
            StatusCode::BAD_REQUEST,
            "APP_ARCHIVED_INVALID",
            "restore archived app before modifying it",
        ),
        AppError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "APP_STORAGE_FAILED",
            "app storage operation failed",
        ),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
