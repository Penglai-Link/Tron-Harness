use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{application::user::AccountError, domain::user::UserError, response::ApiResponse};

pub fn avatar_error_response(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
) -> Response {
    (status, Json(ApiResponse::error(code, message))).into_response()
}

pub fn account_error_response(error: AccountError) -> Response {
    let (status, code, message) = match error {
        AccountError::CurrentPasswordIncorrect => (
            StatusCode::BAD_REQUEST,
            "ACCOUNT_CURRENT_PASSWORD_INCORRECT",
            "current password is incorrect",
        ),
        AccountError::PasswordChangeUnsupported => (
            StatusCode::BAD_REQUEST,
            "ACCOUNT_PASSWORD_CHANGE_UNSUPPORTED",
            "password is managed by the external identity provider",
        ),
        AccountError::PasswordUnchanged => (
            StatusCode::BAD_REQUEST,
            "ACCOUNT_PASSWORD_UNCHANGED",
            "new password must differ from the current password",
        ),
        AccountError::PasswordVerifyFailed => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "ACCOUNT_PASSWORD_VERIFY_FAILED",
            "password verification failed",
        ),
        AccountError::User(UserError::NotFound) => (
            StatusCode::NOT_FOUND,
            "ACCOUNT_PROFILE_NOT_FOUND",
            "account profile was not found",
        ),
        AccountError::User(UserError::RequiredProfileField) => (
            StatusCode::BAD_REQUEST,
            "ACCOUNT_PROFILE_FIELD_REQUIRED",
            "profile fields cannot be blank",
        ),
        AccountError::User(UserError::WeakInitialPassword) => (
            StatusCode::BAD_REQUEST,
            "ACCOUNT_PASSWORD_WEAK",
            "new password does not satisfy the password policy",
        ),
        AccountError::User(UserError::PasswordHashFailed) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "ACCOUNT_PASSWORD_HASH_FAILED",
            "password hash could not be generated",
        ),
        AccountError::User(UserError::StorageFailure) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "ACCOUNT_STORAGE_FAILED",
            "account storage operation failed",
        ),
        AccountError::User(_) => (
            StatusCode::BAD_REQUEST,
            "ACCOUNT_PROFILE_INVALID",
            "account profile is invalid",
        ),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
