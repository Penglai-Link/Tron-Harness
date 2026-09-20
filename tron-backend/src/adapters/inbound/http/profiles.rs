use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    application::{
        model_provider::{
            CompleteProviderAuthorizationCommand, CreateProviderAuthorizationCommand,
            ModelProviderError, ProviderAuthorizationSessionView,
        },
        profile::{ProfileRegistryError, ProfileRegistryService},
    },
    domain::model_provider::ModelProviderCode,
    response::{ApiResponse, EmptyData},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegisterProfileRequest {
    pub person_ai_user_id: String,
    pub person_ai_username: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreatePersonAiAuthorizationRequest {
    pub server_base_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PollPersonAiAuthorizationRequest {}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PersonAiAuthorizationPollResponse {
    completed: bool,
    session: ProviderAuthorizationSessionView,
    connection: Option<crate::domain::model_provider::ProviderConnectionView>,
    text_verified: Option<bool>,
    embedding_verified: Option<bool>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/bootstrap/status", get(bootstrap_status))
        .route(
            "/api/v1/profiles",
            get(list_profiles).post(register_profile),
        )
        .route(
            "/api/v1/profiles/{profile_id}/activate",
            post(activate_profile),
        )
        .route("/api/v1/profiles/pending", post(create_pending_profile))
        .route(
            "/api/v1/profiles/person-ai/authorization-sessions",
            post(create_person_ai_authorization),
        )
        .route(
            "/api/v1/profiles/person-ai/authorization-sessions/{session_id}/poll",
            post(poll_person_ai_authorization),
        )
}

async fn bootstrap_status(State(profiles): State<ProfileRegistryService>) -> Response {
    match profiles.bootstrap_status() {
        Ok(status) => Json(ApiResponse::success(status)).into_response(),
        Err(error) => profile_error_response(error),
    }
}

async fn list_profiles(State(profiles): State<ProfileRegistryService>) -> Response {
    match profiles.list() {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => profile_error_response(error),
    }
}

async fn register_profile(
    State(profiles): State<ProfileRegistryService>,
    Json(request): Json<RegisterProfileRequest>,
) -> Response {
    match profiles.register_or_update(&request.person_ai_user_id, &request.person_ai_username) {
        Ok(record) => (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response(),
        Err(error) => profile_error_response(error),
    }
}

async fn create_pending_profile(State(profiles): State<ProfileRegistryService>) -> Response {
    match profiles.create_pending() {
        Ok(record) => (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response(),
        Err(error) => profile_error_response(error),
    }
}

async fn create_person_ai_authorization(
    State(state): State<AppState>,
    Json(request): Json<CreatePersonAiAuthorizationRequest>,
) -> Response {
    match state
        .model_providers
        .create_authorization_session(CreateProviderAuthorizationCommand {
            provider_code: ModelProviderCode::PersonAi,
            server_base_url: request.server_base_url,
        })
        .await
    {
        Ok(session) => (StatusCode::CREATED, Json(ApiResponse::success(session))).into_response(),
        Err(error) => person_ai_authorization_error_response(error),
    }
}

async fn poll_person_ai_authorization(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(_request): Json<PollPersonAiAuthorizationRequest>,
) -> Response {
    let encryption_key = match state.setup_transaction.ensure_encryption_key() {
        Ok(key) => key,
        Err(error) => {
            return generic_profile_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "PERSONAI_PROFILE_ENCRYPTION_FAILED",
                format!("failed to initialize profile encryption: {error:?}"),
            );
        }
    };
    if let Err(error) = state.variables.configure_cipher(&encryption_key) {
        return generic_profile_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "PERSONAI_PROFILE_ENCRYPTION_FAILED",
            format!("failed to activate profile encryption: {error}"),
        );
    }
    match state
        .model_providers
        .poll_authorization_session(CompleteProviderAuthorizationCommand {
            session_id: session_id.clone(),
            encryption_key_base64: Some(encryption_key),
        })
        .await
    {
        Ok(result) => {
            if let (Some(user_id), Some(username)) = (
                result.connection.person_ai_user_id.as_deref(),
                result.connection.person_ai_username.as_deref(),
            ) {
                if let Err(error) = state.workspaces.ensure_default_workspace(user_id, username) {
                    return generic_profile_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "PERSONAI_PROFILE_INITIALIZATION_FAILED",
                        format!("failed to initialize default workspace: {error}"),
                    );
                }
                if let Err(error) = state.profiles.register_or_update(user_id, username) {
                    return profile_error_response(error);
                }
            }
            Json(ApiResponse::success(PersonAiAuthorizationPollResponse {
                completed: true,
                session: result.session,
                connection: Some(result.connection),
                text_verified: Some(result.text_verified),
                embedding_verified: Some(result.embedding_verified),
            }))
            .into_response()
        }
        Err(ModelProviderError::AuthorizationPending) => {
            match state.model_providers.get_authorization_session(&session_id) {
                Ok(session) => Json(ApiResponse::success(PersonAiAuthorizationPollResponse {
                    completed: false,
                    session,
                    connection: None,
                    text_verified: None,
                    embedding_verified: None,
                }))
                .into_response(),
                Err(error) => person_ai_authorization_error_response(error),
            }
        }
        Err(error) => person_ai_authorization_error_response(error),
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

fn person_ai_authorization_error_response(error: ModelProviderError) -> Response {
    generic_profile_error(
        StatusCode::BAD_REQUEST,
        "PERSONAI_AUTHORIZATION_FAILED",
        error.to_string(),
    )
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
