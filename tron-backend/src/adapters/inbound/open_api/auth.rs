use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

use crate::{
    adapters::inbound::http::auth::{
        RequestAuthContext, authenticate_headers, read_access_token,
    },
    application::user::UserService,
    domain::auth::AuthError,
    response::ApiResponse,
    shared::{
        auth_holder::AuthHolder,
        locale::resolve_request_locale,
    },
};

const OPEN_API_APP_ID_HEADER: &str = "x-tron-app-id";
const OPEN_API_APP_KEY_HEADER: &str = "x-tron-app-key";
const OPEN_API_USER_ID_HEADER: &str = "x-tron-user-id";

fn auth_error_response(error: AuthError) -> Response {
    let (status, code, message) = match error {
        AuthError::UserInactive => (
            StatusCode::FORBIDDEN,
            "AUTH_USER_INACTIVE",
            "user is not active",
        ),
        AuthError::Unauthorized | AuthError::InvalidCredentials => (
            StatusCode::UNAUTHORIZED,
            "AUTH_UNAUTHORIZED",
            "authentication required",
        ),
        AuthError::RefreshInvalid => (
            StatusCode::UNAUTHORIZED,
            "AUTH_REFRESH_INVALID",
            "refresh session is invalid",
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

fn read_open_api_header(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn authenticate_open_api_headers(
    users: &UserService,
    headers: &HeaderMap,
) -> Result<RequestAuthContext, AuthError> {
    let app_id = read_open_api_header(headers, OPEN_API_APP_ID_HEADER)
        .ok_or(AuthError::Unauthorized)?;
    let app_key = read_open_api_header(headers, OPEN_API_APP_KEY_HEADER)
        .ok_or(AuthError::Unauthorized)?;
    let user_id = read_open_api_header(headers, OPEN_API_USER_ID_HEADER)
        .ok_or(AuthError::Unauthorized)?;
    if app_id.is_empty() || app_key.is_empty() || user_id.is_empty() {
        return Err(AuthError::Unauthorized);
    }

    let authenticated = users.authenticate_open_api_user(&user_id)?;
    let locale = resolve_request_locale(
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok()),
    );

    Ok(RequestAuthContext {
        access_token: format!("open-api:{app_id}:{user_id}"),
        locale,
        session: authenticated.session,
        token_identity: authenticated.token_identity,
    })
}

pub async fn require_open_api_auth(
    State(users): State<UserService>,
    mut request: Request,
    next: Next,
) -> Response {
    let auth = if read_access_token(request.headers()).is_some() {
        authenticate_headers(&users, request.headers())
    } else {
        authenticate_open_api_headers(&users, request.headers())
    };

    match auth {
        Ok(auth) => {
            let holder_context = auth.holder_context();
            request.extensions_mut().insert(auth);
            AuthHolder::scope(holder_context, next.run(request)).await
        }
        Err(error) => auth_error_response(error),
    }
}
