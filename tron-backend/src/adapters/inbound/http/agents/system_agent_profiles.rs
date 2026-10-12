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
    agent_code: String,
    name: String,
    base_agent_code: String,
    model_id: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSystemAgentResponse {
    agent_code: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateSystemAgentRequest {
    name: String,
    model_id: String,
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
    match state.agent_management.create_system_agent_profile(
        &request.agent_code,
        &request.name,
        &request.base_agent_code,
        &request.model_id,
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
    match state.agent_management.update_system_agent_profile(
        &code,
        &request.name,
        &request.model_id,
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
    let zh = locale.starts_with("zh");
    let (status, code, message) = match error {
        AgentManagementError::InternalAgentCodeInvalid => (
            StatusCode::BAD_REQUEST,
            "SYSTEM_AGENT_CODE_INVALID",
            if zh {
                "标识须为 1–64 位小写字母、数字、连字符或下划线，并以字母开头"
            } else {
                "Use 1–64 lowercase letters, numbers, hyphens or underscores, starting with a letter"
            },
        ),
        AgentManagementError::InternalAgentCodeConflict => (
            StatusCode::CONFLICT,
            "SYSTEM_AGENT_CODE_CONFLICT",
            if zh {
                "Agent 标识已被使用"
            } else {
                "Agent code is already in use"
            },
        ),
        AgentManagementError::InternalAgentDefinitionInvalid => (
            StatusCode::BAD_REQUEST,
            "SYSTEM_AGENT_CONFIG_INVALID",
            if zh {
                "请提供有效的名称、基础 Agent 和模型 ID"
            } else {
                "Provide a valid name, base Agent and model ID"
            },
        ),
        AgentManagementError::BuiltinAgentReadOnly => (
            StatusCode::BAD_REQUEST,
            "SYSTEM_AGENT_BASE_READ_ONLY",
            if zh {
                "扫描得到的基础 Agent 不可修改或删除，请新建独立配置"
            } else {
                "Scanned base Agents cannot be edited or deleted; create a separate configuration"
            },
        ),
        AgentManagementError::SystemAgentNotFound => (
            StatusCode::NOT_FOUND,
            "SYSTEM_AGENT_NOT_FOUND",
            if zh {
                "系统 Agent 不存在"
            } else {
                "System Agent was not found"
            },
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "SYSTEM_AGENT_STORAGE_FAILED",
            if zh {
                "无法保存系统 Agent 配置"
            } else {
                "Could not save system Agent configuration"
            },
        ),
    };
    (status, Json(ApiResponse::<EmptyData>::error(code, message))).into_response()
}
