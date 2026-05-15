use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::role::RoleError, response::ApiResponse};

pub fn role_error_response(error: RoleError) -> Response {
    let (status, code, message) = match error {
        RoleError::DuplicateRoleCode => (
            StatusCode::BAD_REQUEST,
            "ADMIN_ROLE_CODE_DUPLICATE",
            "duplicate role code",
        ),
        RoleError::DuplicateRoleName => (
            StatusCode::BAD_REQUEST,
            "ADMIN_ROLE_NAME_DUPLICATE",
            "duplicate role name",
        ),
        RoleError::InvalidPermissionCode => (
            StatusCode::BAD_REQUEST,
            "ADMIN_ROLE_PERMISSION_CODE_INVALID",
            "invalid permission code",
        ),
        RoleError::InvalidPermissionDomain => (
            StatusCode::BAD_REQUEST,
            "ADMIN_ROLE_PERMISSION_DOMAIN_INVALID",
            "invalid permission domain",
        ),
        RoleError::InvalidRoleCode => (
            StatusCode::BAD_REQUEST,
            "ADMIN_ROLE_CODE_INVALID",
            "invalid role code",
        ),
        RoleError::MissingRequiredSystemRole => (
            StatusCode::BAD_REQUEST,
            "ADMIN_ROLE_SYSTEM_ROLE_MISSING",
            "missing required system role",
        ),
        RoleError::RoleNameRequired => (
            StatusCode::BAD_REQUEST,
            "ADMIN_ROLE_NAME_REQUIRED",
            "role name is required",
        ),
        RoleError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "ADMIN_ROLE_STORAGE_FAILURE",
            "role storage failure",
        ),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
