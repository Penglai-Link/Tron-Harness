use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use crate::{
    app::AppState,
    application::profile::{ProfileRegistryError, ProfileRegistryService},
    response::{ApiResponse, EmptyData},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/bootstrap/status", get(bootstrap_status))
        .route(
            "/api/v1/profiles",
            get(list_profiles),
        )
        .route(
            "/api/v1/profiles/{profile_id}/activate",
            post(activate_profile),
        )
        .route("/api/v1/profiles/{profile_id}", delete(remove_profile))
        .route("/api/v1/profiles/pending", post(create_pending_profile))
}

async fn remove_profile(
    State(profiles): State<ProfileRegistryService>,
    Path(profile_id): Path<String>,
) -> Response {
    match profiles.remove(&profile_id) {
        Ok(()) => Json(ApiResponse::success(serde_json::json!({}))).into_response(),
        Err(error) => profile_error_response(error),
    }
}

async fn bootstrap_status(State(state): State<AppState>) -> Response {
    let mut status = match state.profiles.bootstrap_status() {
        Ok(status) => status,
        Err(error) => return profile_error_response(error),
    };

    if let Some(profile) = status.active_profile.as_ref() {
        let username = profile.person_ai_username.clone();
        if let Ok(account) = state.model_providers.get_person_ai_account_info().await {
            if account
                .username
                .as_deref()
                .map(str::trim)
                .is_some_and(|account_username| account_username == username)
            {
                status.avatar_url = account.avatar_url;
            }
        }
    }

    Json(ApiResponse::success(status)).into_response()
}

async fn list_profiles(State(profiles): State<ProfileRegistryService>) -> Response {
    match profiles.list() {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => profile_error_response(error),
    }
}

async fn create_pending_profile(State(profiles): State<ProfileRegistryService>) -> Response {
    match profiles.create_pending() {
        Ok(record) => (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response(),
        Err(error) => profile_error_response(error),
    }
}

async fn activate_profile(
    State(profiles): State<ProfileRegistryService>,
    Path(profile_id): Path<String>,
) -> Response {
    match profiles.activate(&profile_id) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => profile_error_response(error),
    }
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
