use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    adapters::inbound::http::auth::{
        RequestAuthContext,
        handler::{auth_grant_response, response_with_refresh_cookie},
    },
    adapters::outbound::setup_transaction::{SetupCompletionCommand, SetupTransactionError},
    app::AppState,
    config::VariableCryptoConfig,
    domain::workspace::{WorkspaceListQuery, WorkspaceStatus},
    domain::{
        auth::AuthError,
        user::{UserError, validate_initial_password},
    },
    response::{ApiResponse, EmptyData},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupStatusDto {
    required: bool,
    initialized: bool,
    initialized_at: Option<String>,
    initialized_by: Option<String>,
    ready: bool,
    admin: SetupAdminDto,
    crypto: SetupCryptoDto,
    workspace: SetupWorkspaceDto,
    models: SetupModelsDto,
}

struct SetupSnapshot {
    status: SetupStatusDto,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapAdminRequest {
    pub new_password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapAdminResponse {
    activated: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupAdminDto {
    user_code: String,
    username: String,
    display_name: String,
    password_reset_required: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupCryptoDto {
    configured: bool,
    algorithm: &'static str,
    key_length: usize,
    fingerprint: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupWorkspaceDto {
    configured: bool,
    active_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupModelsDto {
    configured: bool,
    required_types: [&'static str; 2],
}

fn snapshot(state: &AppState) -> Result<SetupSnapshot, Response> {
    let persisted = state
        .setup_transaction
        .get()
        .map_err(|_| setup_state_error_response())?;
    let admin = match state.users.get_user("USR-ADMIN") {
        Ok(user) => user,
        Err(_) => {
            return Err(Json(ApiResponse::error(
                "SETUP_ADMIN_NOT_FOUND",
                "bootstrap administrator is not available",
            ))
            .into_response());
        }
    };

    let workspaces = state
        .workspaces
        .list_workspaces(WorkspaceListQuery {
            include_archived: Some(false),
            search: None,
            status: Some(WorkspaceStatus::Active),
            root_only: None,
        })
        .unwrap_or_default();
    let actual_crypto = state
        .system_config
        .variable_crypto_key_base64()
        .ok()
        .flatten()
        .map(|key| crypto_status_key(&key))
        .unwrap_or_else(|| crypto_status(&state.config.variable_crypto));
    let workspace_configured = workspaces.len() == 1;
    let manual_model_types = workspace_configured
        && workspaces.first().is_some_and(|workspace| {
            state
                .models
                .list_workspace_models(&workspace.workspace_code)
                .map(|catalog| {
                    ["TEXT", "EMBEDDING"]
                        .into_iter()
                        .all(|model_type| {
                            catalog.defaults.iter().any(|default| {
                                default.model_code.is_some()
                                    && format!("{:?}", default.model_type).to_ascii_uppercase()
                                        == model_type
                            })
                        })
                })
                .unwrap_or(false)
        });
    let person_ai_model_types = state
        .model_providers
        .get_connection(crate::domain::model_provider::ModelProviderCode::PersonAi)
        .ok()
        .is_some_and(|connection| {
            connection.status == crate::domain::model_provider::ProviderConnectionStatus::Connected
                && connection.catalog_model_count > 0
        });
    let configured_model_types = manual_model_types || person_ai_model_types;

    let configuration_ready = !admin.password_reset_required
        && actual_crypto.configured
        && workspace_configured
        && configured_model_types;
    let crypto = if persisted.initialized {
        actual_crypto
    } else {
        SetupCryptoDto {
            configured: false,
            algorithm: "SM4_GCM",
            key_length: 16,
            fingerprint: None,
        }
    };
    Ok(SetupSnapshot {
        status: SetupStatusDto {
            required: !persisted.initialized,
            initialized: persisted.initialized,
            initialized_at: persisted.initialized_at,
            initialized_by: persisted.initialized_by,
            ready: configuration_ready,
            admin: SetupAdminDto {
                user_code: admin.user_code,
                username: admin.username,
                display_name: admin.display_name,
                password_reset_required: admin.password_reset_required,
            },
            crypto,
            workspace: SetupWorkspaceDto {
                configured: workspace_configured,
                active_count: workspaces.len(),
            },
            models: SetupModelsDto {
                configured: configured_model_types,
                required_types: ["TEXT", "EMBEDDING"],
            },
        },
    })
}

pub async fn create_session(
    State(state): State<AppState>,
    Json(request): Json<BootstrapAdminRequest>,
) -> Response {
    let initial_snapshot = match snapshot(&state) {
        Ok(snapshot) => snapshot,
        Err(response) => return response,
    };
    if initial_snapshot.status.initialized {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<EmptyData>::error(
                "SETUP_ALREADY_COMPLETED",
                "initialization has already been completed",
            )),
        )
            .into_response();
    }
    if !initial_snapshot.status.admin.password_reset_required {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<EmptyData>::error(
                "SETUP_ADMIN_ALREADY_ACTIVATED",
                "the bootstrap administrator has already been activated",
            )),
        )
            .into_response();
    }
    if validate_initial_password(&request.new_password).is_err() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<EmptyData>::error(
                "SETUP_ADMIN_PASSWORD_INVALID",
                "new password does not satisfy the password policy",
            )),
        )
            .into_response();
    }

    match state.users.create_bootstrap_setup_session() {
        Ok(session) => response_with_refresh_cookie(
            StatusCode::OK,
            Json(ApiResponse::success(auth_grant_response(
                &state.users,
                &session.access_token,
                &session.session,
            ))),
            &state.users,
            &session.refresh_token,
        ),
        Err(AuthError::InvalidCredentials) => (
            StatusCode::CONFLICT,
            Json(ApiResponse::<EmptyData>::error(
                "SETUP_ADMIN_ALREADY_ACTIVATED",
                "the bootstrap administrator has already been activated",
            )),
        )
            .into_response(),
        Err(_) => setup_state_error_response(),
    }
}

pub async fn status(State(state): State<AppState>) -> Response {
    match snapshot(&state) {
        Ok(snapshot) => Json(ApiResponse::success(snapshot.status)).into_response(),
        Err(response) => response,
    }
}

pub async fn activate_admin(
    State(state): State<AppState>,
    Json(request): Json<BootstrapAdminRequest>,
) -> Response {
    let snapshot = match snapshot(&state) {
        Ok(snapshot) => snapshot,
        Err(response) => return response,
    };
    if snapshot.status.initialized {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<EmptyData>::error(
                "SETUP_ALREADY_COMPLETED",
                "initialization has already been completed",
            )),
        )
            .into_response();
    }
    if !snapshot.status.admin.password_reset_required {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<EmptyData>::error(
                "SETUP_ADMIN_ALREADY_ACTIVATED",
                "the bootstrap administrator has already been activated",
            )),
        )
            .into_response();
    }

    match state
        .users
        .initialize_bootstrap_password(&request.new_password)
    {
        Ok(()) => Json(ApiResponse::success(BootstrapAdminResponse {
            activated: true,
        }))
        .into_response(),
        Err(UserError::WeakInitialPassword) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<EmptyData>::error(
                "SETUP_ADMIN_PASSWORD_INVALID",
                "new password does not satisfy the password policy",
            )),
        )
            .into_response(),
        Err(_) => setup_state_error_response(),
    }
}

pub async fn complete(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Json(request): Json<SetupCompletionCommand>,
) -> Response {
    if !auth.user().role_codes.iter().any(|role| role == "ADMIN") {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<EmptyData>::error(
                "SETUP_ADMIN_REQUIRED",
                "only a platform administrator can complete initialization",
            )),
        )
            .into_response();
    }

    let encryption_key_base64 = request.encryption_key_base64.clone();
    match state
        .setup_transaction
        .complete(&auth.user().user_code, request)
    {
        Ok(completed) => {
            // The transaction has already committed at this point. Loading the
            // new key here keeps the running backend usable without a service
            // restart after the one-shot initialization request succeeds.
            let _ = state.variables.configure_cipher(&encryption_key_base64);
            Json(ApiResponse::success(completed)).into_response()
        }
        Err(error) => setup_transaction_error_response(error),
    }
}

fn setup_transaction_error_response(error: SetupTransactionError) -> Response {
    let (status, code, message): (StatusCode, &'static str, String) = match error {
        SetupTransactionError::AlreadyCompleted => (
            StatusCode::CONFLICT,
            "SETUP_ALREADY_COMPLETED",
            "initialization has already been completed".into(),
        ),
        SetupTransactionError::WorkspaceAlreadyExists => (
            StatusCode::CONFLICT,
            "SETUP_WORKSPACE_ALREADY_EXISTS",
            "initialization allows only one workspace".into(),
        ),
        SetupTransactionError::InvalidPassword => (
            StatusCode::BAD_REQUEST,
            "SETUP_ADMIN_PASSWORD_INVALID",
            "new password does not satisfy the password policy".into(),
        ),
        SetupTransactionError::InvalidEncryptionKey => (
            StatusCode::BAD_REQUEST,
            "SETUP_CRYPTO_INVALID",
            "encryption key must be a 16-byte base64 value".into(),
        ),
        SetupTransactionError::InvalidDisplayName => (
            StatusCode::BAD_REQUEST,
            "SETUP_DISPLAY_NAME_REQUIRED",
            "display name is required".into(),
        ),
        SetupTransactionError::InvalidWorkspaceName => (
            StatusCode::BAD_REQUEST,
            "SETUP_WORKSPACE_NAME_REQUIRED",
            "workspace name is required".into(),
        ),
        SetupTransactionError::InvalidModel(message) => (
            StatusCode::BAD_REQUEST,
            "SETUP_MODEL_INVALID",
            message,
        ),
        SetupTransactionError::PersonAiNotReady => (
            StatusCode::CONFLICT,
            "SETUP_PERSON_AI_NOT_READY",
            "Person-AI authorization, high-quality chat, and embedding verification must complete before initialization".into(),
        ),
        SetupTransactionError::AdminNotFound => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "SETUP_ADMIN_NOT_FOUND",
            "bootstrap administrator is not available".into(),
        ),
        SetupTransactionError::Storage => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "SETUP_TRANSACTION_FAILED",
            "initialization failed and all changes were rolled back".into(),
        ),
    };
    (status, Json(ApiResponse::<EmptyData>::error(code, message))).into_response()
}

fn setup_state_error_response() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiResponse::<EmptyData>::error(
            "SETUP_STATE_UNAVAILABLE",
            "initialization state could not be read or persisted",
        )),
    )
        .into_response()
}

fn crypto_status(config: &VariableCryptoConfig) -> SetupCryptoDto {
    crypto_status_key(&config.key_base64)
}

fn crypto_status_key(key_base64: &str) -> SetupCryptoDto {
    let decoded = BASE64.decode(key_base64.trim()).ok();
    let configured = decoded.as_ref().is_some_and(|value| value.len() == 16);
    let fingerprint = decoded.filter(|value| value.len() == 16).map(|value| {
        let digest = Sha256::digest(value);
        hex::encode(&digest[..6])
    });
    SetupCryptoDto {
        configured,
        algorithm: "SM4_GCM",
        key_length: 16,
        fingerprint,
    }
}
