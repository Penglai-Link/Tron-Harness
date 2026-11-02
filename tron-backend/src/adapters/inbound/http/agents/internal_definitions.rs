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
        let (status, zh, en) = match descriptor.message_key {
            "tron.agent.builtin_read_only" => (
                StatusCode::FORBIDDEN,
                "内置智能体定义为只读",
                "Built-in agent definitions are read-only",
            ),
            "tron.agent.platform_read_only" => (
                StatusCode::FORBIDDEN,
                "平台智能体只能在平台设置中删除",
                "Platform agents can only be deleted in platform settings",
            ),
            "tron.agent.group_invalid" => (
                StatusCode::BAD_REQUEST,
                "分组名称为 1–60 个字符，且必须选择当前范围内已有的分组",
                "Group names must contain 1–60 characters; select an existing group in this scope",
            ),
            "tron.agent.tool_permission_invalid" => (
                StatusCode::BAD_REQUEST,
                "工具权限配置无效，请检查工具编码和重复项",
                "Invalid tool permission configuration; check tool codes and duplicates",
            ),
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
                "名称为 1–100 个字符，指令不超过 64 KB，头像必须从内置列表选择",
                "Name must contain 1 to 100 characters; role prompt must not exceed 64 KB; select a built-in avatar",
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
        let message = if self.1.starts_with("en") { en } else { zh };
        let message = match &self.0 {
            AgentManagementError::InternalToolUnknown(code)
            | AgentManagementError::InternalToolAssignmentStale(code)
            | AgentManagementError::InternalMcpServerAssignmentStale(code) => {
                format!("{message}: {code}")
            }
            _ => message.to_owned(),
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
