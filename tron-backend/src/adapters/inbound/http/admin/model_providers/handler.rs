use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    application::model_provider::{
        CompleteProviderAuthorizationCommand, CreateProviderAuthorizationCommand,
        ModelProviderError, ProviderAuthorizationSessionView,
    },
    domain::model_provider::ModelProviderCode,
    response::{ApiResponse, EmptyData},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateAuthorizationRequest {
    pub server_base_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PollAuthorizationRequest {}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthorizationPollResponse {
    completed: bool,
    session: ProviderAuthorizationSessionView,
    connection: Option<crate::domain::model_provider::ProviderConnectionView>,
    profile_id: Option<String>,
}

pub async fn list_provider_connections(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Response {
    let existing = match state.model_providers.list_connections() {
        Ok(records) => records,
        Err(error) => return model_provider_error_response(error),
    };
    let configured = match state.models.list_configured_providers() {
        Ok(records) => records,
        Err(error) => return super::configured::error_response(error, &headers),
    };
    let records = existing
        .into_iter()
        .map(super::configured::ProviderListResponse::from)
        .chain(
            configured
                .into_iter()
                .map(super::configured::ProviderListResponse::from),
        )
        .collect::<Vec<_>>();
    Json(ApiResponse::protocol_success(records)).into_response()
}

pub async fn get_provider_connection(
    State(state): State<AppState>,
    Path(provider_code): Path<String>,
) -> Response {
    let provider_code = match parse_provider_code(&provider_code) {
        Ok(provider_code) => provider_code,
        Err(response) => return response,
    };
    match state.model_providers.get_connection(provider_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_provider_error_response(error),
    }
}

pub async fn list_catalog_models(
    State(state): State<AppState>,
    Path(provider_code): Path<String>,
) -> Response {
    let provider_code = match parse_provider_code(&provider_code) {
        Ok(provider_code) => provider_code,
        Err(response) => return response,
    };
    match state.model_providers.list_catalog_models(provider_code) {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => model_provider_error_response(error),
    }
}

pub async fn get_provider_account_balance(
    State(state): State<AppState>,
    Path(provider_code): Path<String>,
) -> Response {
    let provider_code = match parse_provider_code(&provider_code) {
        Ok(provider_code) => provider_code,
        Err(response) => return response,
    };
    match state
        .model_providers
        .get_account_balance(provider_code)
        .await
    {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_provider_error_response(error),
    }
}

pub async fn list_sync_runs(
    State(state): State<AppState>,
    Path(provider_code): Path<String>,
) -> Response {
    let provider_code = match parse_provider_code(&provider_code) {
        Ok(provider_code) => provider_code,
        Err(response) => return response,
    };
    match state.model_providers.list_sync_runs(provider_code) {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => model_provider_error_response(error),
    }
}

pub async fn create_authorization_session(
    State(state): State<AppState>,
    Path(provider_code): Path<String>,
    Json(request): Json<CreateAuthorizationRequest>,
) -> Response {
    let provider_code = match parse_provider_code(&provider_code) {
        Ok(provider_code) => provider_code,
        Err(response) => return response,
    };
    match state
        .model_providers
        .create_authorization_session(CreateProviderAuthorizationCommand {
            provider_code,
            server_base_url: request.server_base_url,
        })
        .await
    {
        Ok(record) => (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response(),
        Err(error) => model_provider_error_response(error),
    }
}

pub async fn get_authorization_session(
    State(state): State<AppState>,
    Path((provider_code, session_id)): Path<(String, String)>,
) -> Response {
    if let Err(response) = parse_provider_code(&provider_code) {
        return response;
    }
    match state.model_providers.get_authorization_session(&session_id) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_provider_error_response(error),
    }
}

pub async fn poll_authorization_session(
    State(state): State<AppState>,
    Path((provider_code, session_id)): Path<(String, String)>,
    Json(_request): Json<PollAuthorizationRequest>,
) -> Response {
    poll_authorization(state, provider_code, session_id, None, None).await
}

pub async fn poll_reauthorization_session(
    State(state): State<AppState>,
    Path((provider_code, session_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    Json(_request): Json<PollAuthorizationRequest>,
) -> Response {
    let profile = match state.profiles.current_context() {
        Ok(profile) => profile,
        Err(_) => return model_provider_error_response(ModelProviderError::AuthenticationRequired),
    };
    poll_authorization(
        state,
        provider_code,
        session_id,
        Some(profile),
        Some(headers),
    )
    .await
}

async fn poll_authorization(
    state: AppState,
    provider_code: String,
    session_id: String,
    expected_profile: Option<crate::application::profile::CurrentProfileContext>,
    headers: Option<axum::http::HeaderMap>,
) -> Response {
    if let Err(response) = parse_provider_code(&provider_code) {
        return response;
    }
    match state
        .model_providers
        .poll_authorization_session(CompleteProviderAuthorizationCommand {
            session_id: session_id.clone(),
            expected_person_ai_user_id: expected_profile
                .as_ref()
                .map(|profile| profile.person_ai_user_id.clone()),
        })
        .await
    {
        Ok(result) => {
            let mut profile_id = None;
            if let (Some(user_id), Some(username)) = (
                result.connection.person_ai_user_id.as_deref(),
                result.connection.person_ai_username.as_deref(),
            ) {
                if let Some(profile) = expected_profile.as_ref() {
                    // Refreshing a provider's authorization does not register,
                    // activate, rename or switch a local profile.
                    profile_id = Some(profile.profile_id.clone());
                } else {
                    if let Err(error) = state.workspaces.ensure_default_workspace(user_id, username)
                    {
                        return model_provider_profile_error(format!(
                            "failed to initialize default workspace: {error}"
                        ));
                    }
                    let profile = match state.profiles.register_or_update(user_id, username) {
                        Ok(profile) => profile,
                        Err(error) => {
                            return model_provider_profile_error(format!(
                                "failed to register PersonAI profile: {error}"
                            ));
                        }
                    };
                    profile_id = Some(profile.profile_id);
                }
            }
            // The periodic heartbeat maintains device presence. It is not a
            // prerequisite for returning an already committed authorization.
            state.account_sync.request_sync();
            Json(ApiResponse::success(AuthorizationPollResponse {
                completed: true,
                session: result.session,
                connection: Some(result.connection),
                profile_id,
            }))
            .into_response()
        }
        Err(ModelProviderError::AuthorizationPending) => {
            match state.model_providers.get_authorization_session(&session_id) {
                Ok(session) => Json(ApiResponse::success(AuthorizationPollResponse {
                    completed: false,
                    session,
                    connection: None,
                    profile_id: None,
                }))
                .into_response(),
                Err(error) => model_provider_error_response(error),
            }
        }
        Err(ModelProviderError::AuthorizationAccountMismatch) => {
            let locale = crate::shared::locale::resolve_request_locale(
                headers
                    .as_ref()
                    .and_then(|headers| headers.get(axum::http::header::ACCEPT_LANGUAGE))
                    .and_then(|value| value.to_str().ok()),
            );
            (StatusCode::CONFLICT, Json(ApiResponse::<EmptyData>::error(
                "MODEL_PROVIDER_AUTH_ACCOUNT_MISMATCH",
                if locale.starts_with("zh") {
                    "请使用当前 Profile 对应的 PersonAI 账号重新授权；其他账号请从 Profile 页面添加"
                } else {
                    "Authorize with the current profile's PersonAI account; add another account from the profile page"
                },
            ))).into_response()
        }
        Err(error) => model_provider_error_response(error),
    }
}

fn model_provider_profile_error(message: String) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiResponse::<EmptyData>::error(
            "PERSONAI_PROFILE_INITIALIZATION_FAILED",
            message,
        )),
    )
        .into_response()
}

pub async fn sync_provider(
    State(state): State<AppState>,
    Path(provider_code): Path<String>,
) -> Response {
    let provider_code = match parse_provider_code(&provider_code) {
        Ok(provider_code) => provider_code,
        Err(response) => return response,
    };
    match state.model_providers.sync_now(provider_code).await {
        Ok(run) => Json(ApiResponse::success(run)).into_response(),
        Err(error) => model_provider_error_response(error),
    }
}

pub async fn disconnect_provider(
    State(state): State<AppState>,
    Path(provider_code): Path<String>,
) -> Response {
    let provider_code = match parse_provider_code(&provider_code) {
        Ok(provider_code) => provider_code,
        Err(response) => return response,
    };
    match state.model_providers.disconnect(provider_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => model_provider_error_response(error),
    }
}

fn parse_provider_code(value: &str) -> Result<ModelProviderCode, Response> {
    ModelProviderCode::from_storage(value).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<EmptyData>::error(
                "MODEL_PROVIDER_NOT_FOUND",
                "model provider was not found",
            )),
        )
            .into_response()
    })
}

fn model_provider_error_response(error: ModelProviderError) -> Response {
    let (status, code) = match error {
        ModelProviderError::AuthorizationSessionNotFound => (
            StatusCode::NOT_FOUND,
            "MODEL_PROVIDER_AUTH_SESSION_NOT_FOUND",
        ),
        ModelProviderError::AuthorizationPending => {
            (StatusCode::ACCEPTED, "MODEL_PROVIDER_AUTH_PENDING")
        }
        ModelProviderError::AuthorizationEnded(_) => {
            (StatusCode::CONFLICT, "MODEL_PROVIDER_AUTH_ENDED")
        }
        ModelProviderError::AuthorizationAccountMismatch => {
            (StatusCode::CONFLICT, "MODEL_PROVIDER_AUTH_ACCOUNT_MISMATCH")
        }
        ModelProviderError::InvalidServerBaseUrl => {
            (StatusCode::BAD_REQUEST, "MODEL_PROVIDER_SERVER_URL_INVALID")
        }
        ModelProviderError::InvalidEncryptionKey => {
            (StatusCode::BAD_REQUEST, "MODEL_PROVIDER_CRYPTO_INVALID")
        }
        ModelProviderError::ConnectionNotReady => {
            (StatusCode::CONFLICT, "MODEL_PROVIDER_CONNECTION_NOT_READY")
        }
        ModelProviderError::CatalogModelUnavailable(_) => {
            (StatusCode::CONFLICT, "MODEL_PROVIDER_MODEL_UNAVAILABLE")
        }
        ModelProviderError::InvalidSelection(_) => {
            (StatusCode::BAD_REQUEST, "MODEL_SELECTION_INVALID")
        }
        ModelProviderError::AuthenticationRequired => {
            (StatusCode::CONFLICT, "MODEL_PROVIDER_AUTH_REQUIRED")
        }
        ModelProviderError::Unauthorized => (StatusCode::CONFLICT, "MODEL_PROVIDER_AUTH_REQUIRED"),
        ModelProviderError::DeviceUnbound => (StatusCode::UNAUTHORIZED, "PERSONAI_DEVICE_UNBOUND"),
        ModelProviderError::CapabilityPackUnavailable => (StatusCode::NOT_FOUND, "101038005"),
        ModelProviderError::CapabilityReleaseUnavailable => (StatusCode::NOT_FOUND, "101038004"),
        ModelProviderError::Remote(_) => (StatusCode::BAD_GATEWAY, "MODEL_PROVIDER_REMOTE_FAILED"),
        ModelProviderError::Storage => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "MODEL_PROVIDER_STORAGE_FAILED",
        ),
    };
    (
        status,
        Json(ApiResponse::<EmptyData>::error(code, error.to_string())),
    )
        .into_response()
}
