use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    domain::connector::{ConnectionError, ConnectionView},
    response::{ApiResponse, EmptyData},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/connector-providers", get(providers))
        .route("/api/v1/connector-connections", get(list).post(create))
        .route(
            "/api/v1/connector-connections/{connection_code}",
            patch(update).delete(delete),
        )
        .route(
            "/api/v1/connector-connections/{connection_code}/check",
            post(check),
        )
        .route(
            "/api/v1/connector-connections/{connection_code}/repositories/search",
            post(search),
        )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateRequest {
    provider_id: String,
    name: String,
    endpoint: String,
    token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UpdateRequest {
    provider_id: String,
    name: String,
    endpoint: String,
    token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SearchRequest {
    query: String,
    cursor: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderResponse {
    id: String,
    display_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionResponse {
    connection_code: String,
    provider_id: String,
    name: String,
    endpoint: String,
}
impl From<ConnectionView> for ConnectionResponse {
    fn from(value: ConnectionView) -> Self {
        Self {
            connection_code: value.connection_code,
            provider_id: value.provider_id,
            name: value.name,
            endpoint: value.endpoint,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RepositoryResponse {
    id: String,
    name: String,
    full_name: String,
    web_url: String,
    clone_url: Option<String>,
    ssh_url: Option<String>,
    default_branch: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchResponse {
    records: Vec<RepositoryResponse>,
    next_cursor: Option<String>,
}

async fn providers(State(state): State<AppState>) -> Response {
    let items = state
        .integrations
        .providers()
        .into_iter()
        .map(|p| ProviderResponse {
            id: p.id,
            display_name: p.display_name,
        })
        .collect::<Vec<_>>();
    Json(ApiResponse::protocol_success(items)).into_response()
}

async fn list(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    headers: HeaderMap,
) -> Response {
    match state.integrations.list("PLATFORM", &auth.user().user_code) {
        Ok(items) => Json(ApiResponse::protocol_success(
            items
                .into_iter()
                .map(ConnectionResponse::from)
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => error_response(error, &headers),
    }
}

async fn create(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    headers: HeaderMap,
    Json(request): Json<CreateRequest>,
) -> Response {
    match state.integrations.save(
        "PLATFORM",
        &auth.user().user_code,
        None,
        &request.provider_id,
        &request.name,
        &request.endpoint,
        Some(&request.token),
    ) {
        Ok(item) => (
            StatusCode::CREATED,
            Json(ApiResponse::protocol_success(ConnectionResponse::from(
                item,
            ))),
        )
            .into_response(),
        Err(error) => error_response(error, &headers),
    }
}

async fn update(
    State(state): State<AppState>,
    Path(code): Path<String>,
    auth: RequestAuthContext,
    headers: HeaderMap,
    Json(request): Json<UpdateRequest>,
) -> Response {
    match state.integrations.save(
        "PLATFORM",
        &auth.user().user_code,
        Some(&code),
        &request.provider_id,
        &request.name,
        &request.endpoint,
        request.token.as_deref(),
    ) {
        Ok(item) => Json(ApiResponse::protocol_success(ConnectionResponse::from(
            item,
        )))
        .into_response(),
        Err(error) => error_response(error, &headers),
    }
}

async fn delete(
    State(state): State<AppState>,
    Path(code): Path<String>,
    auth: RequestAuthContext,
    headers: HeaderMap,
) -> Response {
    match state
        .integrations
        .delete("PLATFORM", &auth.user().user_code, &code)
    {
        Ok(()) => Json(ApiResponse::protocol_success(EmptyData {})).into_response(),
        Err(error) => error_response(error, &headers),
    }
}

async fn check(
    State(state): State<AppState>,
    Path(code): Path<String>,
    auth: RequestAuthContext,
    headers: HeaderMap,
) -> Response {
    match state
        .integrations
        .check("PLATFORM", &auth.user().user_code, &code)
        .await
    {
        Ok(()) => Json(ApiResponse::protocol_success(EmptyData {})).into_response(),
        Err(error) => error_response(error, &headers),
    }
}

async fn search(
    State(state): State<AppState>,
    Path(code): Path<String>,
    auth: RequestAuthContext,
    headers: HeaderMap,
    Json(request): Json<SearchRequest>,
) -> Response {
    let page = eliteforge_agent_connector::PageRequest {
        cursor: request.cursor,
        limit: 50,
    };
    match state
        .integrations
        .repositories(
            "PLATFORM",
            &auth.user().user_code,
            &code,
            request.query.trim(),
            page,
        )
        .await
    {
        Ok(page) => Json(ApiResponse::protocol_success(SearchResponse {
            records: page
                .items
                .into_iter()
                .map(|r| RepositoryResponse {
                    id: r.id,
                    name: r.name,
                    full_name: r.full_name,
                    web_url: r.web_url,
                    clone_url: r.clone_url,
                    ssh_url: r.ssh_url,
                    default_branch: r.default_branch,
                })
                .collect(),
            next_cursor: page.next_cursor,
        }))
        .into_response(),
        Err(error) => error_response(error, &headers),
    }
}

pub(crate) fn error_response(error: ConnectionError, headers: &HeaderMap) -> Response {
    use crate::shared::locale::SupportedLocale;
    use eliteforge_agent_connector::ConnectorError as Remote;
    use tron_i18n::t;
    let locale = SupportedLocale::from_request(
        headers
            .get("accept-language")
            .and_then(|value| value.to_str().ok()),
    );
    let (status, code) = match &error {
        ConnectionError::InvalidInput => (StatusCode::BAD_REQUEST, "CONNECTOR_INPUT_INVALID"),
        ConnectionError::NotFound | ConnectionError::Remote(Remote::NotFound) => {
            (StatusCode::NOT_FOUND, "CONNECTOR_NOT_FOUND")
        }
        ConnectionError::DuplicateName => (StatusCode::CONFLICT, "CONNECTOR_NAME_DUPLICATE"),
        ConnectionError::EncryptionUnavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "CONNECTOR_ENCRYPTION_UNAVAILABLE",
        ),
        ConnectionError::Remote(Remote::Authentication) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "CONNECTOR_AUTHENTICATION_FAILED",
        ),
        ConnectionError::Remote(Remote::Forbidden) => {
            (StatusCode::FORBIDDEN, "CONNECTOR_FORBIDDEN")
        }
        ConnectionError::Remote(Remote::Unsupported) => {
            (StatusCode::UNPROCESSABLE_ENTITY, "CONNECTOR_UNSUPPORTED")
        }
        ConnectionError::Remote(Remote::RateLimited) => {
            (StatusCode::TOO_MANY_REQUESTS, "CONNECTOR_RATE_LIMITED")
        }
        ConnectionError::Remote(Remote::IndeterminateWrite) => {
            (StatusCode::BAD_GATEWAY, "CONNECTOR_WRITE_UNKNOWN")
        }
        ConnectionError::Remote(Remote::Configuration | Remote::Rejected) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "CONNECTOR_REQUEST_REJECTED",
        ),
        ConnectionError::Remote(Remote::Unavailable | Remote::InvalidResponse) => {
            (StatusCode::BAD_GATEWAY, "CONNECTOR_UNAVAILABLE")
        }
        ConnectionError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONNECTOR_STORAGE_FAILED",
        ),
    };
    let message = match &error {
        ConnectionError::InvalidInput => t!("tron.connector.input_invalid", locale = locale),
        ConnectionError::NotFound | ConnectionError::Remote(Remote::NotFound) => {
            t!("tron.connector.not_found", locale = locale)
        }
        ConnectionError::DuplicateName => {
            t!("tron.connector.name_duplicate", locale = locale)
        }
        ConnectionError::EncryptionUnavailable => {
            t!("tron.connector.encryption_unavailable", locale = locale)
        }
        ConnectionError::Remote(Remote::Authentication) => {
            t!("tron.connector.authentication_failed", locale = locale)
        }
        ConnectionError::Remote(Remote::Forbidden) => {
            t!("tron.connector.forbidden", locale = locale)
        }
        ConnectionError::Remote(Remote::Unsupported) => {
            t!("tron.connector.unsupported", locale = locale)
        }
        ConnectionError::Remote(Remote::RateLimited) => {
            t!("tron.connector.rate_limited", locale = locale)
        }
        ConnectionError::Remote(Remote::IndeterminateWrite) => {
            t!("tron.connector.write_unknown", locale = locale)
        }
        ConnectionError::Remote(Remote::Configuration | Remote::Rejected) => {
            t!("tron.connector.request_rejected", locale = locale)
        }
        ConnectionError::Remote(Remote::Unavailable | Remote::InvalidResponse) => {
            t!("tron.connector.unavailable", locale = locale)
        }
        ConnectionError::StorageFailure => {
            t!("tron.connector.storage_failed", locale = locale)
        }
    };
    (status, Json(ApiResponse::protocol_error(code, message))).into_response()
}
