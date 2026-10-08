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
    role_prompt: String,
    loop_code: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInternalAgentResponse {
    agent_code: String,
    name: String,
    role_prompt: String,
    loop_code: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateInternalAgentDefinitionRequest {
    name: Option<String>,
    role_prompt: Option<String>,
    loop_code: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInternalAgentDefinitionResponse {
    agent_code: String,
    name: String,
    role_prompt: String,
    loop_code: String,
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
                role_prompt: request.role_prompt,
                loop_code: request.loop_code,
            },
        )
        .map_err(|error| InternalAgentApiError(error, auth.locale.clone()))?;
    Ok(Json(ApiResponse::success(CreateInternalAgentResponse {
        agent_code: record.agent_code,
        name: record.name,
        role_prompt: record.role_prompt,
        loop_code: record.loop_code,
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
                name: request.name,
                role_prompt: request.role_prompt,
                loop_code: request.loop_code,
            },
        )
        .map_err(|error| InternalAgentApiError(error, auth.locale.clone()))?;
    Ok(Json(ApiResponse::success(
        UpdateInternalAgentDefinitionResponse {
            agent_code: record.agent_code,
            name: record.name,
            role_prompt: record.role_prompt,
            loop_code: record.loop_code,
        },
    )))
}

pub(super) struct InternalAgentApiError(pub(super) AgentManagementError, pub(super) String);
impl IntoResponse for InternalAgentApiError {
    fn into_response(self) -> Response {
        let descriptor = self.0.internal_definition_descriptor();
        let (status, zh, en) = match descriptor.message_key {
            "tron.agent.loop_invalid" => (
                StatusCode::BAD_REQUEST,
                "Loop 配置无效，请检查节点、依赖、分支和并行限制",
                "Invalid loop nodes, dependencies, branch conditions or concurrency",
            ),
            "tron.agent.loop_not_found" => (StatusCode::NOT_FOUND, "Loop 不存在", "Loop not found"),
            "tron.agent.loop_conflict" => (
                StatusCode::CONFLICT,
                "Loop 已存在或版本已变化，请刷新后重试",
                "Loop already exists or version changed; refresh and retry",
            ),
            "tron.agent.loop_in_use" => (
                StatusCode::CONFLICT,
                "Loop 已被智能体引用，无法删除",
                "Loop is referenced by an agent and cannot be deleted",
            ),
            "tron.agent.code_invalid" => (
                StatusCode::BAD_REQUEST,
                "编码为 1–64 位，以小写字母开头，仅支持小写字母、数字、连字符和下划线",
                "Code must contain 1–64 lowercase letters, numbers, hyphens or underscores, starting with a letter",
            ),
            "tron.agent.code_conflict" => (
                StatusCode::CONFLICT,
                "智能体编码已被使用，请更换编码",
                "Agent code is already in use; choose another code",
            ),
            "tron.agent.definition_invalid" => (
                StatusCode::BAD_REQUEST,
                "名称不能为空且不能超过 100 个字符，角色提示词不能超过 64 KB",
                "Name must contain 1 to 100 characters; role prompt must not exceed 64 KB",
            ),
            "tron.agent.internal_not_found" => (
                StatusCode::NOT_FOUND,
                "内部智能体不存在",
                "Internal agent not found",
            ),
            "tron.agent.workspace_invalid" => (
                StatusCode::BAD_REQUEST,
                "请选择有效的工作区",
                "Select an active workspace",
            ),
            "tron.agent.policy_invalid" => (
                StatusCode::BAD_REQUEST,
                "智能体策略或运行限制无效",
                "Invalid agent policy or execution limits",
            ),
            "tron.agent.tool_unknown" => (StatusCode::BAD_REQUEST, "工具不存在", "Unknown tool"),
            "tron.agent.tool_unavailable" => (
                StatusCode::BAD_REQUEST,
                "工具已不可用，请重新选择",
                "Selected tool is no longer available",
            ),
            "tron.agent.mcp_unavailable" => (
                StatusCode::BAD_REQUEST,
                "MCP 服务已不可用，请重新选择",
                "Selected MCP server is no longer available",
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "智能体配置保存失败",
                "Could not save agent configuration",
            ),
        };
        (
            status,
            Json(ApiResponse::<EmptyData>::protocol_error(
                descriptor.code,
                if self.1.starts_with("en") { en } else { zh },
            )),
        )
            .into_response()
    }
}
