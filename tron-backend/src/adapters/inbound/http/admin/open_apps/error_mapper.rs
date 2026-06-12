use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::open_app::OpenAppError, response::ApiResponse};

pub fn open_app_error_response(error: OpenAppError) -> Response {
    let (status, code, message) = match error {
        OpenAppError::NotFound => (
            StatusCode::NOT_FOUND,
            "OPEN_APP_NOT_FOUND",
            "open-app was not found",
        ),
        OpenAppError::NameRequired => (
            StatusCode::BAD_REQUEST,
            "OPEN_APP_NAME_REQUIRED",
            "open-app name is required",
        ),
        OpenAppError::DuplicateName => (
            StatusCode::CONFLICT,
            "OPEN_APP_NAME_DUPLICATE",
            "open-app name already exists",
        ),
        OpenAppError::InvalidOpenAppCode => (
            StatusCode::BAD_REQUEST,
            "OPEN_APP_CODE_INVALID",
            "open-app code is invalid",
        ),
        OpenAppError::InvalidUserCode | OpenAppError::UserForbidden => (
            StatusCode::BAD_REQUEST,
            "OPEN_APP_USER_CODE_INVALID",
            "userCode is invalid",
        ),
        OpenAppError::InvalidCapability
        | OpenAppError::InvalidOperation
        | OpenAppError::OperationForbidden => (
            StatusCode::BAD_REQUEST,
            "OPEN_APP_CAPABILITY_INVALID",
            "capability grant is invalid",
        ),
        OpenAppError::OpenAppDisabled => (
            StatusCode::CONFLICT,
            "OPEN_APP_DISABLED",
            "open-app is not enabled",
        ),
        OpenAppError::Unauthorized
        | OpenAppError::InvalidSignatureVersion
        | OpenAppError::InvalidTimestamp
        | OpenAppError::InvalidSignature
        | OpenAppError::NonceReplayed => (
            StatusCode::BAD_REQUEST,
            "OPEN_APP_REQUEST_INVALID",
            "open-app request is invalid",
        ),
        OpenAppError::SecretCryptoFailure | OpenAppError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "OPEN_APP_INTERNAL_ERROR",
            "open-app management failed",
        ),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
