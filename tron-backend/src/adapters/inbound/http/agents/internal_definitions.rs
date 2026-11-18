use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::agent::{
        CreateWorkspaceInternalAgentCommand, UpdateWorkspaceInternalAgentDefinitionCommand,
    },
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
pub struct CreateInternalAgentRequest {
    agent_code: String,
    name: String,
    #[serde(default)]
    description: String,
    role_prompt: String,
    avatar_id: Option<String>,
    group_name: Option<String>,
    #[serde(default)]
    candidate_model_codes: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInternalAgentResponse {
    agent_code: String,
    name: String,
    #[serde(default)]
    description: String,
    role_prompt: String,
    avatar_id: String,
    group_name: Option<String>,
    #[serde(default)]
    candidate_model_codes: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateInternalAgentDefinitionRequest {
    name: Option<String>,
    description: Option<String>,
    role_prompt: Option<String>,
    avatar_id: Option<String>,
    group_name: Option<String>,
    #[serde(default)]
    candidate_model_codes: Option<Vec<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInternalAgentDefinitionResponse {
    agent_code: String,
    name: String,
    #[serde(default)]
    description: String,
    role_prompt: String,
    avatar_id: String,
    group_name: Option<String>,
    #[serde(default)]
    candidate_model_codes: Vec<String>,
}

pub async fn create_internal_agent(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(workspace): Path<String>,
    Json(request): Json<CreateInternalAgentRequest>,
) -> Result<Json<ApiResponse<CreateInternalAgentResponse>>, InternalAgentApiError> {
    let record = state
        .agent_management
        .create_internal_agent(
            &workspace,
            CreateWorkspaceInternalAgentCommand {
                agent_code: request.agent_code,
                name: request.name,
                description: request.description,
                role_prompt: request.role_prompt,
                avatar_id: request.avatar_id,
                group_name: request.group_name,
                candidate_model_codes: request.candidate_model_codes,
            },
        )
        .map_err(|error| InternalAgentApiError(error, auth.locale.clone()))?;
    Ok(Json(ApiResponse::success(CreateInternalAgentResponse {
        agent_code: record.agent_code,
        name: record.name,
        description: record.description,
        role_prompt: record.role_prompt,
        avatar_id: record.avatar_id,
        group_name: record.group_name,
        candidate_model_codes: record.candidate_model_codes,
    })))
}

pub async fn update_internal_agent_definition(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path((workspace, agent)): Path<(String, String)>,
    Json(request): Json<UpdateInternalAgentDefinitionRequest>,
) -> Result<Json<ApiResponse<UpdateInternalAgentDefinitionResponse>>, InternalAgentApiError> {
    let record = state
        .agent_management
        .update_internal_agent_definition(
            &workspace,
            &agent,
            UpdateWorkspaceInternalAgentDefinitionCommand {
                enabled: None,
                name: request.name,
                description: request.description,
                role_prompt: request.role_prompt,
                avatar_id: request.avatar_id,
                group_name: request.group_name,
                candidate_model_codes: request.candidate_model_codes,
            },
        )
        .map_err(|error| InternalAgentApiError(error, auth.locale.clone()))?;
    Ok(Json(ApiResponse::success(
        UpdateInternalAgentDefinitionResponse {
            agent_code: record.agent_code,
            name: record.name,
            description: record.description,
            role_prompt: record.role_prompt,
            avatar_id: record.avatar_id,
            group_name: record.group_name,
            candidate_model_codes: record.candidate_model_codes,
        },
    )))
}

pub async fn delete_internal_agent(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path((workspace, agent)): Path<(String, String)>,
) -> Result<Json<ApiResponse<EmptyData>>, InternalAgentApiError> {
    state
        .agent_management
        .delete_internal_agent(&workspace, &agent)
        .map(|()| Json(ApiResponse::success(EmptyData {})))
        .map_err(|error| InternalAgentApiError(error, auth.locale))
}

pub(super) struct InternalAgentApiError(pub(super) AgentManagementError, pub(super) String);
impl IntoResponse for InternalAgentApiError {
    fn into_response(self) -> Response {
        let descriptor = self.0.internal_definition_descriptor();
        let (status, message_key) = match descriptor.message_key {
            "tron.agent.builtin_read_only" => {
                (StatusCode::FORBIDDEN, "tron.agent.builtin_read_only")
            }
            "tron.agent.platform_read_only" => {
                (StatusCode::FORBIDDEN, "tron.agent.platform_read_only")
            }
            "tron.agent.group_invalid" => (StatusCode::BAD_REQUEST, "tron.agent.group_invalid"),
            "tron.agent.tool_permission_invalid" => (
                StatusCode::BAD_REQUEST,
                "tron.agent.tool_permission_invalid",
            ),
            "tron.agent.loop_invalid" => (StatusCode::BAD_REQUEST, "tron.agent.loop_invalid"),
            "tron.agent.loop_not_found" => (StatusCode::NOT_FOUND, "tron.agent.loop_not_found"),
            "tron.agent.loop_conflict" => (StatusCode::CONFLICT, "tron.agent.loop_conflict"),
            "tron.agent.loop_in_use" => (StatusCode::CONFLICT, "tron.agent.loop_in_use"),
            "tron.agent.code_invalid" => (StatusCode::BAD_REQUEST, "tron.agent.code_invalid"),
            "tron.agent.code_conflict" => (StatusCode::CONFLICT, "tron.agent.code_conflict"),
            "tron.agent.definition_invalid" => {
                (StatusCode::BAD_REQUEST, "tron.agent.definition_invalid")
            }
            "tron.agent.internal_not_found" => {
                (StatusCode::NOT_FOUND, "tron.agent.internal_not_found")
            }
            "tron.agent.workspace_invalid" => {
                (StatusCode::BAD_REQUEST, "tron.agent.workspace_invalid")
            }
            "tron.agent.policy_invalid" => (StatusCode::BAD_REQUEST, "tron.agent.policy_invalid"),
            "tron.agent.tool_unknown" => (StatusCode::BAD_REQUEST, "tron.agent.tool_unknown"),
            "tron.agent.tool_unavailable" => {
                (StatusCode::BAD_REQUEST, "tron.agent.tool_unavailable")
            }
            "tron.agent.mcp_unavailable" => (StatusCode::BAD_REQUEST, "tron.agent.mcp_unavailable"),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "tron.agent.storage_failed",
            ),
        };
        let base = tron_i18n::t!(message_key, locale = &self.1).into_owned();
        let message = match &self.0 {
            AgentManagementError::InternalToolUnknown(code)
            | AgentManagementError::InternalToolAssignmentStale(code)
            | AgentManagementError::InternalMcpServerAssignmentStale(code) => {
                format!("{base}: {code}")
            }
            _ => base,
        };
        (
            status,
            Json(ApiResponse::<EmptyData>::protocol_error(
                descriptor.code,
                message,
            )),
        )
            .into_response()
    }
}
