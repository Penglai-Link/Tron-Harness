use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    domain::agent::AgentManagementError,
    response::{ApiResponse, EmptyData},
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateSystemAgentRequest {
    #[serde(default)]
    description: String,
    #[serde(default)]
    fast_mode: bool,
    agent_code: String,
    name: String,
    base_agent_code: String,
    model_id: String,
    reasoning_effort: Option<crate::domain::reasoning::ReasoningEffort>,
    avatar_id: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSystemAgentResponse {
    agent_code: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateSystemAgentRequest {
    description: Option<String>,
    #[serde(default)]
    fast_mode: bool,
    name: String,
    model_id: String,
    reasoning_effort: Option<crate::domain::reasoning::ReasoningEffort>,
    avatar_id: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSystemAgentResponse {
    agent_code: String,
}
#[derive(Serialize)]
pub struct DeleteSystemAgentResponse {
    deleted: bool,
}

pub async fn create(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Json(request): Json<CreateSystemAgentRequest>,
) -> Response {
    match state
        .agent_management
        .create_system_agent_profile_with_settings(
            &request.agent_code,
            &request.name,
            &request.base_agent_code,
            &request.model_id,
            request.reasoning_effort,
            request.avatar_id,
            request.fast_mode,
            Some(request.description),
        ) {
        Ok(record) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(CreateSystemAgentResponse {
                agent_code: record.agent_code,
            })),
        )
            .into_response(),
        Err(error) => error_response(error, &auth.locale),
    }
}
pub async fn update(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(code): Path<String>,
    Json(request): Json<UpdateSystemAgentRequest>,
) -> Response {
    match state
        .agent_management
        .update_system_agent_profile_with_settings(
            &code,
            &request.name,
            &request.model_id,
            request.reasoning_effort,
            request.avatar_id,
            request.fast_mode,
            request.description,
        ) {
        Ok(record) => Json(ApiResponse::success(UpdateSystemAgentResponse {
            agent_code: record.agent_code,
        }))
        .into_response(),
        Err(error) => error_response(error, &auth.locale),
    }
}
pub async fn delete(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(code): Path<String>,
) -> Response {
    match state.agent_management.delete_system_agent_profile(&code) {
        Ok(()) => Json(ApiResponse::success(DeleteSystemAgentResponse {
            deleted: true,
        }))
        .into_response(),
        Err(error) => error_response(error, &auth.locale),
    }
}
fn error_response(error: AgentManagementError, locale: &str) -> Response {
    let (status, code, message_key) = match error {
        AgentManagementError::InternalAgentCodeInvalid => (
            StatusCode::BAD_REQUEST,
            "SYSTEM_AGENT_CODE_INVALID",
            "agents.system_agent.code_invalid",
        ),
        AgentManagementError::InternalAgentCodeConflict => (
            StatusCode::CONFLICT,
            "SYSTEM_AGENT_CODE_CONFLICT",
            "agents.system_agent.code_conflict",
        ),
        AgentManagementError::InternalAgentDefinitionInvalid => (
            StatusCode::BAD_REQUEST,
            "SYSTEM_AGENT_CONFIG_INVALID",
            "agents.system_agent.config_invalid",
        ),
        AgentManagementError::BuiltinAgentReadOnly => (
            StatusCode::BAD_REQUEST,
            "SYSTEM_AGENT_BASE_READ_ONLY",
            "agents.system_agent.base_read_only",
        ),
        AgentManagementError::SystemAgentNotFound => (
            StatusCode::NOT_FOUND,
            "SYSTEM_AGENT_NOT_FOUND",
            "agents.system_agent.not_found",
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "SYSTEM_AGENT_STORAGE_FAILED",
            "agents.system_agent.storage_failed",
        ),
    };
    (
        status,
        Json(ApiResponse::<EmptyData>::error(
            code,
            tron_i18n::t!(message_key, locale = locale),
        )),
    )
        .into_response()
}
