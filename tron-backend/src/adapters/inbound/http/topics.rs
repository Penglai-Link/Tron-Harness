use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get},
};
use serde::Deserialize;

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::topic::AddTopicReference,
    domain::topic::{TopicError, TopicReferenceKind},
    response::ApiResponse,
    shared::locale::SupportedLocale,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/topics/{project_code}/memo",
            get(get_memo).put(save_memo),
        )
        .route(
            "/api/v1/topics/{project_code}/references",
            axum::routing::post(add_reference),
        )
        .route(
            "/api/v1/topics/{project_code}/references/{reference_code}",
            delete(remove_reference),
        )
        .route(
            "/api/v1/topics/{project_code}/references/{reference_code}/checkout",
            get(checkout_options).post(checkout),
        )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveMemoRequest {
    revision: u64,
    notes: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AddReferenceRequest {
    revision: u64,
    kind: TopicReferenceKind,
    target_code: String,
    connection_code: Option<String>,
    label: String,
    url: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevisionQuery {
    revision: u64,
}

#[derive(Deserialize)]
#[serde(tag = "mode", rename_all_fields = "camelCase", deny_unknown_fields)]
enum CheckoutRequest {
    Existing { path: String },
    Clone { path: String, username: String },
}

async fn get_memo(
    State(state): State<AppState>,
    Path(code): Path<String>,
    auth: RequestAuthContext,
    headers: HeaderMap,
) -> Response {
    match state.topics.get(&auth.user().user_code, &code) {
        Ok(memo) => Json(ApiResponse::protocol_success(memo)).into_response(),
        Err(error) => error_response(error, &headers),
    }
}

async fn save_memo(
    State(state): State<AppState>,
    Path(code): Path<String>,
    auth: RequestAuthContext,
    headers: HeaderMap,
    Json(request): Json<SaveMemoRequest>,
) -> Response {
    match state.topics.save_notes(
        &auth.user().user_code,
        &code,
        request.revision,
        &request.notes,
    ) {
        Ok(memo) => Json(ApiResponse::protocol_success(memo)).into_response(),
        Err(error) => error_response(error, &headers),
    }
}

async fn add_reference(
    State(state): State<AppState>,
    Path(code): Path<String>,
    auth: RequestAuthContext,
    headers: HeaderMap,
    Json(request): Json<AddReferenceRequest>,
) -> Response {
    let input = AddTopicReference {
        kind: request.kind,
        target_code: request.target_code,
        connection_code: request.connection_code,
        label: request.label,
        url: request.url,
    };
    match state
        .topics
        .add_reference(&auth.user().user_code, &code, request.revision, input)
    {
        Ok(memo) => Json(ApiResponse::protocol_success(memo)).into_response(),
        Err(error) => error_response(error, &headers),
    }
}

async fn remove_reference(
    State(state): State<AppState>,
    Path((code, reference)): Path<(String, String)>,
    Query(query): Query<RevisionQuery>,
    auth: RequestAuthContext,
    headers: HeaderMap,
) -> Response {
    match state
        .topics
        .remove_reference(&auth.user().user_code, &code, query.revision, &reference)
    {
        Ok(memo) => Json(ApiResponse::protocol_success(memo)).into_response(),
        Err(error) => error_response(error, &headers),
    }
}

async fn checkout(
    State(state): State<AppState>,
    Path((project, reference)): Path<(String, String)>,
    auth: RequestAuthContext,
    headers: HeaderMap,
    Json(request): Json<CheckoutRequest>,
) -> Response {
    use crate::application::repository_checkout::{self, CheckoutChoice};
    let workspace = match state.projects.get(&project) {
        Ok(project) => project.workspace_code,
        Err(_) => return error_response(TopicError::NotFound, &headers),
    };
    let choice = match request {
        CheckoutRequest::Existing { path } => CheckoutChoice::Existing { path },
        CheckoutRequest::Clone { path, username } => CheckoutChoice::Clone { path, username },
    };
    match repository_checkout::prepare(
        &state.integrations,
        &state.directories,
        &state.topics,
        &workspace,
        &auth.user().user_code,
        &project,
        &reference,
        choice,
    )
    .await
    {
        Ok(memo) => Json(ApiResponse::protocol_success(memo)).into_response(),
        Err(error) => checkout_error_response(error, &headers),
    }
}

async fn checkout_options(
    State(state): State<AppState>,
    Path((project, reference)): Path<(String, String)>,
    auth: RequestAuthContext,
    headers: HeaderMap,
) -> Response {
    let workspace = match state.projects.get(&project) {
        Ok(project) => project.workspace_code,
        Err(_) => return error_response(TopicError::NotFound, &headers),
    };
    match crate::application::repository_checkout::options(
        &state.integrations,
        &state.directories,
        &state.topics,
        &workspace,
        &auth.user().user_code,
        &project,
        &reference,
    )
    .await
    {
        Ok(options) => Json(ApiResponse::protocol_success(options)).into_response(),
        Err(error) => checkout_error_response(error, &headers),
    }
}

fn checkout_error_response(
    error: crate::application::repository_checkout::CheckoutError,
    headers: &HeaderMap,
) -> Response {
    use crate::application::repository_checkout::CheckoutError;
    match error {
        CheckoutError::Connection(error) => super::connectors::error_response(error, headers),
        CheckoutError::Topic(error) => error_response(error, headers),
        CheckoutError::Directory(error) => super::directories::error_response(error, headers),
        error => {
            use tron_i18n::t;
            let code = match error {
                CheckoutError::InvalidPath => "CHECKOUT_PATH_INVALID",
                CheckoutError::DifferentRepository => "CHECKOUT_REPOSITORY_MISMATCH",
                CheckoutError::UnsupportedRemote => "CHECKOUT_REMOTE_UNSUPPORTED",
                CheckoutError::GitFailed => "CHECKOUT_GIT_FAILED",
                CheckoutError::TimedOut => "CHECKOUT_TIMEOUT",
                _ => unreachable!(),
            };
            let locale = SupportedLocale::from_request(
                headers
                    .get("accept-language")
                    .and_then(|value| value.to_str().ok()),
            );
            let message = match code {
                "CHECKOUT_PATH_INVALID" => {
                    t!("tron.checkout.path_invalid", locale = locale)
                }
                "CHECKOUT_REPOSITORY_MISMATCH" => {
                    t!("tron.checkout.repository_mismatch", locale = locale)
                }
                "CHECKOUT_REMOTE_UNSUPPORTED" => {
                    t!("tron.checkout.remote_unsupported", locale = locale)
                }
                "CHECKOUT_GIT_FAILED" => {
                    t!("tron.checkout.git_failed", locale = locale)
                }
                _ => t!("tron.checkout.timeout", locale = locale),
            };
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ApiResponse::protocol_error(code, message)),
            )
                .into_response()
        }
    }
}

fn error_response(error: TopicError, headers: &HeaderMap) -> Response {
    use tron_i18n::t;
    let locale = SupportedLocale::from_request(
        headers
            .get("accept-language")
            .and_then(|value| value.to_str().ok()),
    );
    let (status, code) = match error {
        TopicError::NotFound => (StatusCode::NOT_FOUND, "TOPIC_NOT_FOUND"),
        TopicError::InvalidReference => (StatusCode::BAD_REQUEST, "TOPIC_REFERENCE_INVALID"),
        TopicError::InvalidNotes => (StatusCode::BAD_REQUEST, "TOPIC_NOTES_INVALID"),
        TopicError::Conflict => (StatusCode::CONFLICT, "TOPIC_REVISION_CONFLICT"),
        TopicError::StorageFailure => (StatusCode::INTERNAL_SERVER_ERROR, "TOPIC_STORAGE_FAILED"),
    };
    let message = match code {
        "TOPIC_NOT_FOUND" => t!("tron.topic.not_found", locale = locale),
        "TOPIC_REFERENCE_INVALID" => t!("tron.topic.reference_invalid", locale = locale),
        "TOPIC_NOTES_INVALID" => t!("tron.topic.notes_invalid", locale = locale),
        "TOPIC_REVISION_CONFLICT" => t!("tron.topic.conflict", locale = locale),
        _ => t!("tron.topic.storage_failed", locale = locale),
    };
    (status, Json(ApiResponse::protocol_error(code, message))).into_response()
}
