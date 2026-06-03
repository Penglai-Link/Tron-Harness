use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::auth::AuthError, response::ApiResponse};

pub fn auth_error_response(error: AuthError) -> Response {
    let (status, code, message) = match error {
        AuthError::InvalidCredentials => (
            StatusCode::UNAUTHORIZED,
            "AUTH_INVALID_CREDENTIALS",
            "invalid credentials",
        ),
        AuthError::UserInactive => (
            StatusCode::FORBIDDEN,
            "AUTH_USER_INACTIVE",
            "user is not active",
        ),
        AuthError::Unauthorized => (
            StatusCode::UNAUTHORIZED,
            "AUTH_UNAUTHORIZED",
            "authentication required",
        ),
        AuthError::RefreshInvalid => (
            StatusCode::UNAUTHORIZED,
            "AUTH_REFRESH_INVALID",
            "refresh session is invalid",
        ),
        AuthError::StepUpFailed => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_STEP_UP_FAILED",
            "Password step-up failed",
        ),
        AuthError::StepUpExpired => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_STEP_UP_EXPIRED",
            "Password step-up token expired",
        ),
        AuthError::StepUpUnsupported => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_STEP_UP_UNSUPPORTED",
            "Password step-up is unsupported for this user",
        ),
        AuthError::StepUpTokenMismatch => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_STEP_UP_TOKEN_MISMATCH",
            "Password step-up token does not match this request",
        ),
        AuthError::StepUpScopeInvalid => (
            StatusCode::BAD_REQUEST,
            "VARIABLE_STEP_UP_SCOPE_INVALID",
            "Password step-up target is invalid",
        ),
        AuthError::ProviderNotFound => (
            StatusCode::NOT_FOUND,
            "AUTH_PROVIDER_NOT_FOUND",
            "auth provider was not found",
        ),
        AuthError::UserNotLinked => (
            StatusCode::FORBIDDEN,
            "AUTH_USER_NOT_LINKED",
            "oauth2 identity is not linked to an active internal user",
        ),
        AuthError::OAuth2StateInvalid => (
            StatusCode::BAD_REQUEST,
            "AUTH_OAUTH2_STATE_INVALID",
            "oauth2 state is invalid",
        ),
        AuthError::OAuth2CodeInvalid => (
            StatusCode::BAD_REQUEST,
            "AUTH_OAUTH2_CODE_INVALID",
            "oauth2 code is invalid",
        ),
        AuthError::OAuth2ProviderError => (
            StatusCode::BAD_REQUEST,
            "AUTH_OAUTH2_PROVIDER_ERROR",
            "oauth2 provider returned an error",
        ),
        AuthError::OAuth2ProfileInvalid => (
            StatusCode::BAD_REQUEST,
            "AUTH_OAUTH2_PROFILE_INVALID",
            "oauth2 profile payload is invalid",
        ),
        AuthError::OAuth2UpstreamFailed => (
            StatusCode::BAD_GATEWAY,
            "AUTH_OAUTH2_UPSTREAM_FAILED",
            "oauth2 upstream request failed",
        ),
        AuthError::LdapProfileInvalid => (
            StatusCode::BAD_GATEWAY,
            "AUTH_LDAP_PROFILE_INVALID",
            "ldap profile payload is invalid",
        ),
        AuthError::LdapUpstreamFailed => (
            StatusCode::BAD_GATEWAY,
            "AUTH_LDAP_UPSTREAM_FAILED",
            "ldap upstream request failed",
        ),
        AuthError::TokenEncodingFailed
        | AuthError::TokenDecodingFailed
        | AuthError::PasswordVerifyFailed
        | AuthError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "AUTH_INTERNAL_ERROR",
            "auth operation failed",
        ),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
