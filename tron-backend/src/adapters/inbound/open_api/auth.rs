use axum::{
    Json,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    application::open_app::{OpenAppService, SignedOpenApiRequest, resolve_open_api_route_grant},
    domain::open_app::OpenAppError,
    response::ApiResponse,
    shared::{auth_holder::AuthHolder, locale::resolve_request_locale},
};

const OPEN_API_APP_CODE_HEADER: &str = "x-tron-app-code";
const OPEN_API_USER_CODE_HEADER: &str = "x-tron-user-code";
const OPEN_API_TIMESTAMP_HEADER: &str = "x-tron-timestamp";
const OPEN_API_NONCE_HEADER: &str = "x-tron-nonce";
const OPEN_API_SIGNATURE_HEADER: &str = "x-tron-signature";
const OPEN_API_SIGNATURE_VERSION_HEADER: &str = "x-tron-signature-version";

fn open_api_error_response(error: OpenAppError) -> Response {
    let (status, code, message) = match error {
        OpenAppError::InvalidTimestamp => (
            StatusCode::UNAUTHORIZED,
            "OPEN_API_TIMESTAMP_INVALID",
            "signature timestamp is invalid",
        ),
        OpenAppError::NonceReplayed => (
            StatusCode::UNAUTHORIZED,
            "OPEN_API_NONCE_REPLAYED",
            "nonce was replayed",
        ),
        OpenAppError::OpenAppDisabled => (
            StatusCode::UNAUTHORIZED,
            "OPEN_API_APP_DISABLED",
            "open-app is not enabled or credential is revoked",
        ),
        OpenAppError::UserForbidden => (
            StatusCode::FORBIDDEN,
            "OPEN_API_USER_FORBIDDEN",
            "acting user is not allowed",
        ),
        OpenAppError::OperationForbidden => (
            StatusCode::FORBIDDEN,
            "OPEN_API_OPERATION_FORBIDDEN",
            "requested operation is not allowed",
        ),
        OpenAppError::InvalidSignatureVersion
        | OpenAppError::Unauthorized
        | OpenAppError::InvalidSignature
        | OpenAppError::InvalidOpenAppCode
        | OpenAppError::InvalidUserCode => (
            StatusCode::UNAUTHORIZED,
            "OPEN_API_UNAUTHORIZED",
            "authentication required",
        ),
        OpenAppError::SecretCryptoFailure | OpenAppError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "OPEN_API_INTERNAL_ERROR",
            "open-api auth operation failed",
        ),
        OpenAppError::NotFound
        | OpenAppError::NameRequired
        | OpenAppError::DuplicateName
        | OpenAppError::InvalidCapability
        | OpenAppError::InvalidOperation => (
            StatusCode::BAD_REQUEST,
            "OPEN_API_REQUEST_INVALID",
            "open-api auth request is invalid",
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

fn build_request_auth_context(
    auth: crate::application::open_app::AuthenticatedOpenAppRequest,
    locale: String,
) -> RequestAuthContext {
    RequestAuthContext {
        access_token: format!(
            "open-api:{}:{}",
            auth.open_app.open_app_code, auth.authenticated.token_identity.user_code
        ),
        locale,
        session: auth.authenticated.session,
        token_identity: auth.authenticated.token_identity,
    }
}

pub async fn require_open_api_auth(
    State(open_apps): State<OpenAppService>,
    request: Request,
    next: Next,
) -> Response {
    let locale = resolve_request_locale(
        request
            .headers()
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok()),
    );
    let route_grant =
        match resolve_open_api_route_grant(request.method().as_str(), request.uri().path()) {
            Some(route_grant) => route_grant,
            None => return open_api_error_response(OpenAppError::OperationForbidden),
        };
    let (parts, body) = request.into_parts();
    let body_bytes = match to_bytes(body, usize::MAX).await {
        Ok(body) => body,
        Err(_) => return open_api_error_response(OpenAppError::StorageFailure),
    };
    let signed_request = match build_signed_request(
        &parts.headers,
        parts.method.as_str(),
        parts.uri.path(),
        parts.uri.query(),
        &body_bytes,
    ) {
        Ok(request) => request,
        Err(error) => return open_api_error_response(error),
    };
    let auth = match open_apps.authenticate_signed_request(signed_request, route_grant) {
        Ok(auth) => build_request_auth_context(auth, locale),
        Err(error) => return open_api_error_response(error),
    };

    let mut request = Request::from_parts(parts, Body::from(body_bytes));
    let holder_context = auth.holder_context();
    request.extensions_mut().insert(auth);
    AuthHolder::scope(holder_context, next.run(request)).await
}

fn build_signed_request(
    headers: &HeaderMap,
    method: &str,
    path: &str,
    raw_query: Option<&str>,
    body: &[u8],
) -> Result<SignedOpenApiRequest, OpenAppError> {
    let timestamp = read_open_api_header(headers, OPEN_API_TIMESTAMP_HEADER)
        .ok_or(OpenAppError::Unauthorized)?
        .parse::<i64>()
        .map_err(|_| OpenAppError::InvalidTimestamp)?;
    Ok(SignedOpenApiRequest {
        method: method.to_owned(),
        path: path.to_owned(),
        raw_query: raw_query.map(ToOwned::to_owned),
        body: body.to_vec(),
        open_app_code: read_open_api_header(headers, OPEN_API_APP_CODE_HEADER)
            .ok_or(OpenAppError::Unauthorized)?,
        user_code: read_open_api_header(headers, OPEN_API_USER_CODE_HEADER)
            .ok_or(OpenAppError::Unauthorized)?,
        timestamp,
        nonce: read_open_api_header(headers, OPEN_API_NONCE_HEADER)
            .ok_or(OpenAppError::Unauthorized)?,
        signature_version: read_open_api_header(headers, OPEN_API_SIGNATURE_VERSION_HEADER)
            .ok_or(OpenAppError::Unauthorized)?,
        signature: read_open_api_header(headers, OPEN_API_SIGNATURE_HEADER)
            .ok_or(OpenAppError::Unauthorized)?,
    })
}
