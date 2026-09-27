use crate::{
    app::AppState,
    application::profile::{PersonAiAccountSummary, ProfileRecord, ProfileRegistryError},
    response::{ApiResponse, EmptyData},
};
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::{delete, get, post},
};
use serde::Deserialize;
use std::{convert::Infallible, time::Duration};

const MAX_PROFILE_AVATAR_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapStatusQuery {
    refresh_account: Option<bool>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/bootstrap/status", get(bootstrap_status))
        .route("/api/v1/profiles/events", get(server_auth_events))
        .route("/api/v1/profiles/active/token", get(active_profile_token))
        .route("/api/v1/profiles", get(list_profiles))
        .route("/api/v1/profiles/{profile_id}/avatar", get(profile_avatar))
        .route(
            "/api/v1/profiles/{profile_id}/activate",
            post(activate_profile),
        )
        .route("/api/v1/profiles/{profile_id}", delete(remove_profile))
        .route("/api/v1/profiles/pending", post(create_pending_profile))
}

async fn server_auth_events(State(state): State<AppState>) -> Response {
    let mut receiver = state.model_providers.subscribe_server_auth_events();
    let stream = async_stream::stream! {
        if state.model_providers.device_unbound() {
            yield Ok::<_, Infallible>(Event::default().event("server-auth-device-unbound").data("{}"));
        }
        loop {
            match receiver.recv().await {
                Ok(()) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    yield Ok(Event::default().event("server-auth-device-unbound").data("{}"));
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(20)))
        .into_response()
}

async fn active_profile_token(State(state): State<AppState>) -> Response {
    match state.model_providers.get_person_ai_access_token().await {
        Ok(token) => {
            Json(ApiResponse::success(serde_json::json!({ "token": token }))).into_response()
        }
        Err(_) => generic_profile_error(
            StatusCode::UNAUTHORIZED,
            "PERSONAI_PROFILE_AUTH_REQUIRED",
            "The active PersonAI profile is not authorized",
        ),
    }
}

async fn remove_profile(State(state): State<AppState>, Path(profile_id): Path<String>) -> Response {
    match state.profiles.remove(&profile_id) {
        Ok(()) => Json(ApiResponse::success(serde_json::json!({}))).into_response(),
        Err(error) => profile_error_response(error),
    }
}

async fn bootstrap_status(
    State(state): State<AppState>,
    Query(query): Query<BootstrapStatusQuery>,
) -> Response {
    let mut status = match state.profiles.bootstrap_status() {
        Ok(status) => status,
        Err(error) => return profile_error_response(error),
    };

    if query.refresh_account.unwrap_or(true)
        && let Some(profile) = status.active_profile.clone()
    {
        let username = profile.person_ai_username.clone();
        if state
            .model_providers
            .activate_person_ai_profile(&profile.person_ai_user_id, &profile.person_ai_username)
            .is_ok()
            && let Ok(account) = state.model_providers.get_person_ai_account_info().await
        {
            if account
                .username
                .as_deref()
                .map(str::trim)
                .is_some_and(|account_username| account_username == username)
            {
                let avatar_url =
                    refresh_cached_profile_avatar(&state, &profile, account.avatar_url.as_deref())
                        .await;
                status.avatar_url = avatar_url.clone();
                if let Some(active_profile) = status.active_profile.as_mut() {
                    active_profile.avatar_url = avatar_url;
                }
                status.account = Some(PersonAiAccountSummary {
                    vip_level: account.vip_level,
                    remaining_points: account.remaining_tokens,
                });
            }
        }
    }

    Json(ApiResponse::success(status)).into_response()
}

async fn profile_avatar(State(state): State<AppState>, Path(profile_id): Path<String>) -> Response {
    let avatar = match state.profiles.cached_avatar(&profile_id) {
        Ok(Some(avatar)) => avatar,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => return profile_error_response(error),
    };
    let content_type = match HeaderValue::from_str(&avatar.content_type) {
        Ok(content_type) => content_type,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let mut response = Response::new(Body::from(avatar.image_bytes));
    *response.status_mut() = StatusCode::OK;
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, content_type);
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=3600"),
    );
    response
}

async fn refresh_cached_profile_avatar(
    state: &AppState,
    profile: &ProfileRecord,
    source_avatar_url: Option<&str>,
) -> Option<String> {
    let Some(source_avatar_url) = source_avatar_url.filter(|url| !url.trim().is_empty()) else {
        let _ = state
            .profiles
            .clear_cached_avatar(&profile.person_ai_user_id);
        return None;
    };
    let Ok(source_url) = url::Url::parse(source_avatar_url) else {
        return profile.avatar_url.clone();
    };
    if source_url.scheme() != "https" {
        return profile.avatar_url.clone();
    }

    let Ok(mut response) = state.runtime_http_client.get(source_url).send().await else {
        return profile.avatar_url.clone();
    };
    if response.url().scheme() != "https"
        || !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|length| length > MAX_PROFILE_AVATAR_BYTES as u64)
    {
        return profile.avatar_url.clone();
    }
    let Some(content_type) = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .filter(|value| value.starts_with("image/"))
        .map(str::to_owned)
    else {
        return profile.avatar_url.clone();
    };

    let mut image_bytes = Vec::new();
    loop {
        let chunk = match response.chunk().await {
            Ok(chunk) => chunk,
            Err(_) => return profile.avatar_url.clone(),
        };
        let Some(chunk) = chunk else {
            break;
        };
        if image_bytes.len().saturating_add(chunk.len()) > MAX_PROFILE_AVATAR_BYTES {
            return profile.avatar_url.clone();
        }
        image_bytes.extend_from_slice(&chunk);
    }
    if image_bytes.is_empty()
        || state
            .profiles
            .cache_avatar(&profile.person_ai_user_id, &content_type, &image_bytes)
            .is_err()
    {
        return profile.avatar_url.clone();
    }
    state
        .profiles
        .cached_avatar_url(&profile.profile_id)
        .ok()
        .flatten()
        .or_else(|| profile.avatar_url.clone())
}

async fn list_profiles(State(state): State<AppState>) -> Response {
    match state.profiles.list() {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => profile_error_response(error),
    }
}

async fn create_pending_profile(State(state): State<AppState>) -> Response {
    match state.profiles.create_pending() {
        Ok(record) => (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response(),
        Err(error) => profile_error_response(error),
    }
}

async fn activate_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Response {
    match state.profiles.activate(&profile_id) {
        Ok(record) => match state
            .model_providers
            .activate_person_ai_profile(&record.person_ai_user_id, &record.person_ai_username)
        {
            Ok(()) => Json(ApiResponse::success(record)).into_response(),
            Err(error) => provider_activation_error_response(error),
        },
        Err(error) => profile_error_response(error),
    }
}

fn provider_activation_error_response(
    error: crate::application::model_provider::ModelProviderError,
) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiResponse::<EmptyData>::error(
            "PERSONAI_PROFILE_CREDENTIALS_FAILED",
            format!("failed to activate PersonAI profile credentials: {error}"),
        )),
    )
        .into_response()
}

fn profile_error_response(error: ProfileRegistryError) -> Response {
    let (status, code, message) = match error {
        ProfileRegistryError::ProfileNotFound(profile_id) => (
            StatusCode::NOT_FOUND,
            "PERSONAI_PROFILE_NOT_FOUND",
            format!("PersonAI profile `{profile_id}` was not found"),
        ),
        ProfileRegistryError::InvalidInput(field) => (
            StatusCode::BAD_REQUEST,
            "PERSONAI_PROFILE_INVALID",
            format!("{field} is required"),
        ),
        ProfileRegistryError::Storage(_) | ProfileRegistryError::LockPoisoned => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "PERSONAI_PROFILE_REGISTRY_FAILED",
            "PersonAI profile registry operation failed".to_owned(),
        ),
    };
    generic_profile_error(status, code, message)
}

fn generic_profile_error(
    status: StatusCode,
    code: &'static str,
    message: impl Into<String>,
) -> Response {
    (
        status,
        Json(ApiResponse::<EmptyData>::error(code, message.into())),
    )
        .into_response()
}
