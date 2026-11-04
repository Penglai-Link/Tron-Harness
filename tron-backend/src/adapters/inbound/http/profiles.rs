use crate::{
    app::AppState,
    application::{model_provider::ModelProviderError, profile::ProfileRegistryError},
    response::{ApiResponse, EmptyData},
};
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header},
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::{delete, get, patch, post},
};
use serde::Deserialize;
use std::{convert::Infallible, time::Duration};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/bootstrap/status", get(bootstrap_status))
        .route("/api/v1/profiles/events", get(server_auth_events))
        .route("/api/v1/profiles/active/token", get(active_profile_token))
        .route(
            "/api/v1/profiles/active/preferences",
            patch(update_preferences),
        )
        .route("/api/v1/profiles", get(list_profiles))
        .route("/api/v1/profiles/{profile_id}/avatar", get(profile_avatar))
        .route(
            "/api/v1/profiles/{profile_id}/activate",
            post(activate_profile),
        )
        .route("/api/v1/profiles/{profile_id}", delete(remove_profile))
        .route("/api/v1/profiles/pending", post(create_pending_profile))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePreferencesRequest {
    time_zone: String,
}

#[utoipa::path(patch, path = "/api/v1/profiles/active/preferences", request_body = UpdatePreferencesRequest,
    responses((status = 200, description = "User time zone saved"), (status = 400, description = "Invalid IANA time zone")), tag = "Profiles")]
pub async fn update_preferences(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<UpdatePreferencesRequest>,
) -> Response {
    let context = match state.profiles.current_context() {
        Ok(context) => context,
        Err(error) => return profile_error_response(error),
    };
    if request.time_zone.trim().parse::<chrono_tz::Tz>().is_err() {
        let locale = crate::shared::locale::resolve_request_locale(
            headers
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|v| v.to_str().ok()),
        );
        return generic_profile_error(
            StatusCode::BAD_REQUEST,
            "PERSONAI_PROFILE_INVALID_TIME_ZONE",
            if locale.starts_with("zh") {
                "请选择有效的 IANA 时区"
            } else {
                "Select a valid IANA time zone"
            },
        );
    }
    match state
        .profiles
        .set_time_zone(&context.person_ai_user_id, &request.time_zone)
    {
        Ok(time_zone) => Json(ApiResponse::success(
            serde_json::json!({"timeZone": time_zone}),
        ))
        .into_response(),
        Err(error) => profile_error_response(error),
    }
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
    let profile = match state.profiles.current_context() {
        Ok(profile) => profile,
        Err(error) => return profile_error_response(error),
    };
    if let Err(error) = state
        .model_providers
        .activate_person_ai_profile(&profile.person_ai_user_id, &profile.person_ai_username)
    {
        return active_profile_token_error(error);
    }
    match state.model_providers.get_person_ai_access_token().await {
        Ok(token) => {
            if state
                .profiles
                .cached_account(&profile.person_ai_user_id)
                .ok()
                .flatten()
                .is_none()
            {
                state.account_sync.request_sync();
            }
            Json(ApiResponse::success(serde_json::json!({ "token": token }))).into_response()
        }
        Err(error) => active_profile_token_error(error),
    }
}

fn active_profile_token_error(error: ModelProviderError) -> Response {
    match error {
        ModelProviderError::AuthenticationRequired
        | ModelProviderError::Unauthorized
        | ModelProviderError::DeviceUnbound
        | ModelProviderError::ConnectionNotReady => generic_profile_error(
            StatusCode::UNAUTHORIZED,
            "PERSONAI_PROFILE_AUTH_REQUIRED",
            "The active PersonAI profile is not authorized",
        ),
        ModelProviderError::Remote(_) => generic_profile_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "PERSONAI_PROFILE_TEMPORARILY_UNAVAILABLE",
            "PersonAI is temporarily unavailable; please retry",
        ),
        _ => generic_profile_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "PERSONAI_PROFILE_TOKEN_FAILED",
            "Could not load the active PersonAI profile token",
        ),
    }
}

async fn remove_profile(State(state): State<AppState>, Path(profile_id): Path<String>) -> Response {
    match state.profiles.remove(&profile_id) {
        Ok(()) => {
            if let Ok(Some(profile)) = state.profiles.active() {
                if let Err(error) = state.model_providers.activate_person_ai_profile(
                    &profile.person_ai_user_id,
                    &profile.person_ai_username,
                ) {
                    return provider_activation_error_response(error);
                }
            }
            state.account_sync.request_sync();
            Json(ApiResponse::success(serde_json::json!({}))).into_response()
        }
        Err(error) => profile_error_response(error),
    }
}

async fn bootstrap_status(State(state): State<AppState>) -> Response {
    match state.profiles.bootstrap_status() {
        Ok(status) => Json(ApiResponse::success(status)).into_response(),
        Err(error) => profile_error_response(error),
    }
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
            Ok(()) => {
                state.account_sync.request_sync();
                Json(ApiResponse::success(record)).into_response()
            }
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
