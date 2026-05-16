use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::workspace::WorkspaceError, response::ApiResponse};

pub fn workspace_error_response(error: WorkspaceError) -> Response {
    let (status, code, message) = match error {
        WorkspaceError::NotFound => (
            StatusCode::NOT_FOUND,
            "ADMIN_WORKSPACE_NOT_FOUND",
            "workspace was not found",
        ),
        WorkspaceError::InvalidWorkspaceCode => (
            StatusCode::BAD_REQUEST,
            "ADMIN_WORKSPACE_CODE_INVALID",
            "workspaceCode must contain uppercase letters, digits, underscore, or hyphen only",
        ),
        WorkspaceError::DuplicateWorkspaceCode => (
            StatusCode::CONFLICT,
            "ADMIN_WORKSPACE_CODE_DUPLICATE",
            "workspaceCode already exists",
        ),
        WorkspaceError::WorkspaceNameRequired => (
            StatusCode::BAD_REQUEST,
            "ADMIN_WORKSPACE_NAME_REQUIRED",
            "workspace name is required",
        ),
        WorkspaceError::InvalidHierarchy => (
            StatusCode::BAD_REQUEST,
            "ADMIN_WORKSPACE_HIERARCHY_INVALID",
            "workspace hierarchy operation is invalid",
        ),
        WorkspaceError::ArchivedMutationDenied => (
            StatusCode::BAD_REQUEST,
            "ADMIN_WORKSPACE_ARCHIVED_INVALID",
            "archived workspace cannot be modified in this operation",
        ),
        WorkspaceError::InvalidAccessUser => (
            StatusCode::BAD_REQUEST,
            "ADMIN_WORKSPACE_ACCESS_USER_INVALID",
            "workspace direct user grants must reference active existing users",
        ),
        WorkspaceError::InvalidAccessRole => (
            StatusCode::BAD_REQUEST,
            "ADMIN_WORKSPACE_ACCESS_ROLE_INVALID",
            "workspace role grants must reference existing role codes",
        ),
        WorkspaceError::DuplicateAccessGrant => (
            StatusCode::BAD_REQUEST,
            "ADMIN_WORKSPACE_ACCESS_DUPLICATE",
            "workspace access payload contains duplicate grants",
        ),
        WorkspaceError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "ADMIN_WORKSPACE_STORAGE_FAILED",
            "workspace storage operation failed",
        ),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
