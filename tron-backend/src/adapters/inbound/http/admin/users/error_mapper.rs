use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::user::UserError, response::ApiResponse};

pub fn user_error_response(error: UserError) -> Response {
    let (status, code, message) = match error {
        UserError::NotFound => (
            StatusCode::NOT_FOUND,
            "ADMIN_USER_NOT_FOUND",
            "user was not found",
        ),
        UserError::InvalidUserCode => (
            StatusCode::BAD_REQUEST,
            "ADMIN_USER_CODE_INVALID",
            "userCode must contain uppercase letters, digits, underscore, or hyphen only",
        ),
        UserError::DuplicateUserCode => (
            StatusCode::CONFLICT,
            "ADMIN_USER_CODE_DUPLICATE",
            "userCode already exists",
        ),
        UserError::DuplicateUsername => (
            StatusCode::CONFLICT,
            "ADMIN_USER_USERNAME_DUPLICATE",
            "username already exists",
        ),
        UserError::DuplicateEmail => (
            StatusCode::CONFLICT,
            "ADMIN_USER_EMAIL_DUPLICATE",
            "email already exists",
        ),
        UserError::RequiredProfileField => (
            StatusCode::BAD_REQUEST,
            "ADMIN_USER_PROFILE_FIELD_REQUIRED",
            "username, firstName, lastName, displayName, and email are required",
        ),
        UserError::WeakInitialPassword => (
            StatusCode::BAD_REQUEST,
            "ADMIN_USER_PASSWORD_WEAK",
            "initialPassword does not satisfy the MVP password policy",
        ),
        UserError::InvalidStatus => (
            StatusCode::BAD_REQUEST,
            "ADMIN_USER_STATUS_INVALID",
            "status must be Active or Archived",
        ),
        UserError::InvalidRoleCode => (
            StatusCode::BAD_REQUEST,
            "ADMIN_USER_ROLE_CODE_INVALID",
            "roleCodes must contain fixed MVP role codes only",
        ),
        UserError::InvalidSpaceCode => (
            StatusCode::BAD_REQUEST,
            "ADMIN_USER_SPACE_CODE_INVALID",
            "spaceCodes must be non-empty uppercase stable codes",
        ),
        UserError::PasswordHashFailed => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "ADMIN_USER_PASSWORD_HASH_FAILED",
            "password hash could not be generated",
        ),
        UserError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "ADMIN_USER_STORAGE_FAILED",
            "user storage operation failed",
        ),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
