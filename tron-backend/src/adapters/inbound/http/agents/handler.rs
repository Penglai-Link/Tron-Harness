use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::agent::{
        AgentAguiStreamService, AgentInvocationCommand, AgentInvocationService,
        AgentManagementService, UpdateWorkspaceInternalAgentMcpPolicyCommand,
        UpdateWorkspaceInternalAgentPolicyCommand,
    },
    domain::agent::{
        AgentRunHistoryQuery, ExpandedDomainPersistenceMode, McpAccessMode, ToolAssignmentMode,
        ToolExposureMode,
    },
    domain::mcp::WorkspaceMcpServerListQuery,
    response::ApiResponse,
};

use super::error_mapper::{
    agent_agui_error_response, agent_error_response, agent_history_error_response,
    agent_management_error_response,
};
use super::request::{
    ListToolsQuery, ReplaceInternalAgentMcpServerAssignmentsRequest,
    ReplaceInternalAgentToolAssignmentsRequest, UpdateInternalAgentMcpPolicyRequest,
    UpdateInternalAgentPolicyRequest, UpdateMainAgentConfigRequest,
};

pub async fn create_agent_run(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Json(request): Json<AgentInvocationCommand>,
) -> Response {
    let service = AgentInvocationService::new(
        state.workspaces.clone(),
        state.apps.clone(),
        state.variables.clone(),
        state.skills.clone(),
        state.agent_management.clone(),
        state.agent_runs.clone(),
        state.mcp.clone(),
        state.event_bus.clone(),
        state.agent_model_client.clone(),
        state.agent_tool_registry.clone(),
        state.runtime_http_client.clone(),
    )
    .with_active_runs(state.active_runs.clone())
    .with_prompt_service(state.prompts.clone());
    match service
        .invoke(
            request,
            Some(auth.user().user_code.clone()),
            Some(auth.locale.clone()),
        )
        .await
    {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => agent_error_response(error),
    }
}

pub async fn list_agent_runs(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Query(query): Query<AgentRunHistoryQuery>,
) -> Response {
    match state.agent_runs.list_runs(query) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => agent_history_error_response(error),
    }
}

pub async fn get_agent_run(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(run_code): Path<String>,
) -> Response {
    match state.agent_runs.get_run(&run_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => agent_history_error_response(error),
    }
}

pub async fn activate_agui_stream(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(stream_key): Path<String>,
) -> Response {
    let service = AgentAguiStreamService::with_replay_store(
        state.agui_events.clone(),
        state.agui_replay_store.clone(),
    );
    match service.activate(&stream_key) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => agent_agui_error_response(error),
    }
}

pub async fn get_main_agent_config(
    State(agent_management): State<AgentManagementService>,
    _auth: RequestAuthContext,
) -> Response {
    match agent_management.get_main_agent_config() {
        Ok(config) => Json(ApiResponse::success(config)).into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn update_main_agent_config(
    State(agent_management): State<AgentManagementService>,
    _auth: RequestAuthContext,
    Json(request): Json<UpdateMainAgentConfigRequest>,
) -> Response {
    match agent_management.update_main_agent_config(request.into()) {
        Ok(config) => Json(ApiResponse::success(config)).into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn list_tool_domains(
    State(agent_management): State<AgentManagementService>,
    auth: RequestAuthContext,
) -> Response {
    Json(ApiResponse::success(
        agent_management.list_tool_domains(&auth.locale),
    ))
    .into_response()
}

pub async fn list_tools(
    State(agent_management): State<AgentManagementService>,
    auth: RequestAuthContext,
    Query(query): Query<ListToolsQuery>,
) -> Response {
    let tools = agent_management.list_tools(
        query.domain_code.as_deref(),
        query.search.as_deref(),
        &auth.locale,
    );
    Json(ApiResponse::success(tools)).into_response()
}

pub async fn get_tool(
    State(agent_management): State<AgentManagementService>,
    auth: RequestAuthContext,
    Path(tool_code): Path<String>,
) -> Response {
    match agent_management.tool(&tool_code, &auth.locale) {
        Some(tool) => Json(ApiResponse::success(tool)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(
                "AGENT_TOOL_NOT_FOUND",
                "tool was not found in platform catalog",
            )),
        )
            .into_response(),
    }
}

pub async fn list_workspace_internal_agents(
    State(agent_management): State<AgentManagementService>,
    _auth: RequestAuthContext,
    Path(workspace_code): Path<String>,
) -> Response {
    match agent_management.list_workspace_internal_agents(&workspace_code) {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn get_workspace_internal_agent(
    State(agent_management): State<AgentManagementService>,
    _auth: RequestAuthContext,
    Path((workspace_code, agent_code)): Path<(String, String)>,
) -> Response {
    match agent_management.workspace_internal_agent_policy(&workspace_code, &agent_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn update_workspace_internal_agent_policy(
    State(agent_management): State<AgentManagementService>,
    _auth: RequestAuthContext,
    Path((workspace_code, agent_code)): Path<(String, String)>,
    Json(request): Json<UpdateInternalAgentPolicyRequest>,
) -> Response {
    let command = match to_internal_agent_policy_command(request) {
        Ok(command) => command,
        Err((code, message)) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(code, message)),
            )
                .into_response();
        }
    };
    match agent_management.update_internal_agent_policy(&workspace_code, &agent_code, command) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn replace_workspace_internal_agent_tool_assignments(
    State(agent_management): State<AgentManagementService>,
    _auth: RequestAuthContext,
    Path((workspace_code, agent_code)): Path<(String, String)>,
    Json(request): Json<ReplaceInternalAgentToolAssignmentsRequest>,
) -> Response {
    let tool_codes = normalize_tool_codes(request.tool_codes);
    match agent_management.replace_internal_agent_tool_assignments(
        &workspace_code,
        &agent_code,
        tool_codes,
    ) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn reset_workspace_internal_agent_policy(
    State(agent_management): State<AgentManagementService>,
    _auth: RequestAuthContext,
    Path((workspace_code, agent_code)): Path<(String, String)>,
) -> Response {
    match agent_management.reset_internal_agent_policy(&workspace_code, &agent_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn update_workspace_internal_agent_mcp_policy(
    State(agent_management): State<AgentManagementService>,
    _auth: RequestAuthContext,
    Path((workspace_code, agent_code)): Path<(String, String)>,
    Json(request): Json<UpdateInternalAgentMcpPolicyRequest>,
) -> Response {
    let command = match to_internal_agent_mcp_policy_command(request) {
        Ok(command) => command,
        Err((code, message)) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(code, message)),
            )
                .into_response();
        }
    };
    match agent_management.update_internal_agent_mcp_policy(&workspace_code, &agent_code, command) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn replace_workspace_internal_agent_mcp_server_assignments(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path((workspace_code, agent_code)): Path<(String, String)>,
    Json(request): Json<ReplaceInternalAgentMcpServerAssignmentsRequest>,
) -> Response {
    let server_codes = normalize_tool_codes(request.server_codes);
    if let Err(error) =
        validate_workspace_mcp_server_assignments(&state, &workspace_code, &server_codes)
    {
        return agent_management_error_response(error);
    }
    match state
        .agent_management
        .replace_internal_agent_mcp_server_assignments(&workspace_code, &agent_code, server_codes)
    {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn list_platform_system_agents(
    State(agent_management): State<AgentManagementService>,
    _auth: RequestAuthContext,
) -> Response {
    match agent_management.list_platform_system_agents() {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn list_runtime_capabilities(
    State(agent_management): State<AgentManagementService>,
    _auth: RequestAuthContext,
) -> Response {
    match agent_management.list_runtime_capabilities() {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn get_runtime_capability(
    State(agent_management): State<AgentManagementService>,
    _auth: RequestAuthContext,
    Path(adapter_key): Path<String>,
) -> Response {
    match agent_management.runtime_capability(&adapter_key) {
        Ok(Some(record)) => Json(ApiResponse::success(record)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(
                "AGENT_RUNTIME_NOT_FOUND",
                "runtime adapter capability was not found",
            )),
        )
            .into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn rescan_platform_system_agents(
    State(agent_management): State<AgentManagementService>,
    _auth: RequestAuthContext,
) -> Response {
    match agent_management.rescan_system_agents() {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn enable_platform_system_agent(
    State(agent_management): State<AgentManagementService>,
    _auth: RequestAuthContext,
    Path(agent_code): Path<String>,
) -> Response {
    match agent_management.enable_platform_system_agent(&agent_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn disable_platform_system_agent(
    State(agent_management): State<AgentManagementService>,
    _auth: RequestAuthContext,
    Path(agent_code): Path<String>,
) -> Response {
    match agent_management.disable_platform_system_agent(&agent_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn list_workspace_system_agents(
    State(agent_management): State<AgentManagementService>,
    _auth: RequestAuthContext,
    Path(workspace_code): Path<String>,
) -> Response {
    match agent_management.list_workspace_system_agents(&workspace_code) {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn enable_workspace_system_agent(
    State(agent_management): State<AgentManagementService>,
    _auth: RequestAuthContext,
    Path((workspace_code, agent_code)): Path<(String, String)>,
) -> Response {
    match agent_management.enable_workspace_system_agent(&workspace_code, &agent_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn disable_workspace_system_agent(
    State(agent_management): State<AgentManagementService>,
    _auth: RequestAuthContext,
    Path((workspace_code, agent_code)): Path<(String, String)>,
) -> Response {
    match agent_management.disable_workspace_system_agent(&workspace_code, &agent_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => agent_management_error_response(error),
    }
}

pub async fn deactivate_agui_stream(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(stream_key): Path<String>,
) -> Response {
    let service = AgentAguiStreamService::with_replay_store(
        state.agui_events.clone(),
        state.agui_replay_store.clone(),
    );
    match service.deactivate(&stream_key) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => agent_agui_error_response(error),
    }
}

fn to_internal_agent_policy_command(
    request: UpdateInternalAgentPolicyRequest,
) -> Result<UpdateWorkspaceInternalAgentPolicyCommand, (&'static str, &'static str)> {
    let tool_assignment_mode = match request.tool_assignment_mode {
        Some(value) => Some(parse_tool_assignment_mode(&value)?),
        None => None,
    };
    let tool_exposure_mode = match request.tool_exposure_mode {
        Some(value) => Some(parse_tool_exposure_mode(&value)?),
        None => None,
    };
    let expanded_domain_persistence_mode = match request.expanded_domain_persistence_mode {
        Some(value) => Some(parse_expanded_domain_persistence_mode(&value)?),
        None => None,
    };

    Ok(UpdateWorkspaceInternalAgentPolicyCommand {
        tool_assignment_mode,
        tool_exposure_mode,
        expanded_domain_persistence_mode,
        max_steps: request.max_steps,
        max_tool_calls: request.max_tool_calls,
        max_parallel_tool_calls: request.max_parallel_tool_calls,
    })
}

fn to_internal_agent_mcp_policy_command(
    request: UpdateInternalAgentMcpPolicyRequest,
) -> Result<UpdateWorkspaceInternalAgentMcpPolicyCommand, (&'static str, &'static str)> {
    let mcp_access_mode = match request.mcp_access_mode {
        Some(value) => Some(parse_mcp_access_mode(&value)?),
        None => None,
    };

    Ok(UpdateWorkspaceInternalAgentMcpPolicyCommand { mcp_access_mode })
}

fn parse_tool_assignment_mode(
    raw: &str,
) -> Result<ToolAssignmentMode, (&'static str, &'static str)> {
    match normalize_mode_token(raw).as_str() {
        "all_platform_tools" => Ok(ToolAssignmentMode::AllPlatformTools),
        "selected_tools" => Ok(ToolAssignmentMode::SelectedTools),
        _ => Err((
            "AGENT_TOOL_ASSIGNMENT_MODE_INVALID",
            "invalid tool assignment mode",
        )),
    }
}

fn parse_tool_exposure_mode(raw: &str) -> Result<ToolExposureMode, (&'static str, &'static str)> {
    match normalize_mode_token(raw).as_str() {
        "direct_list" => Ok(ToolExposureMode::DirectList),
        "domain_discovery" => Ok(ToolExposureMode::DomainDiscovery),
        _ => Err((
            "AGENT_TOOL_EXPOSURE_MODE_INVALID",
            "invalid tool exposure mode",
        )),
    }
}

fn parse_expanded_domain_persistence_mode(
    raw: &str,
) -> Result<ExpandedDomainPersistenceMode, (&'static str, &'static str)> {
    match normalize_mode_token(raw).as_str() {
        "sticky_expanded" => Ok(ExpandedDomainPersistenceMode::StickyExpanded),
        "ephemeral_expanded" => Ok(ExpandedDomainPersistenceMode::EphemeralExpanded),
        _ => Err((
            "AGENT_TOOL_EXPANDED_DOMAIN_PERSISTENCE_MODE_INVALID",
            "invalid expanded domain persistence mode",
        )),
    }
}

fn parse_mcp_access_mode(raw: &str) -> Result<McpAccessMode, (&'static str, &'static str)> {
    match normalize_mode_token(raw).as_str() {
        "disabled" => Ok(McpAccessMode::Disabled),
        "all_enabled_servers" => Ok(McpAccessMode::AllEnabledServers),
        "selected_servers" => Ok(McpAccessMode::SelectedServers),
        _ => Err(("AGENT_MCP_ACCESS_MODE_INVALID", "invalid MCP access mode")),
    }
}

fn normalize_tool_codes(values: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for value in values {
        let normalized = value.trim().to_string();
        if normalized.is_empty() {
            continue;
        }
        if !deduped
            .iter()
            .any(|item: &String| item.eq_ignore_ascii_case(&normalized))
        {
            deduped.push(normalized);
        }
    }
    deduped
}

fn validate_workspace_mcp_server_assignments(
    state: &AppState,
    workspace_code: &str,
    server_codes: &[String],
) -> Result<(), crate::domain::agent::AgentManagementError> {
    let effective_servers = state
        .mcp
        .list_workspace_effective_servers(
            workspace_code,
            WorkspaceMcpServerListQuery { search: None },
        )
        .map_err(map_mcp_assignment_error)?;
    let effective_codes = effective_servers
        .into_iter()
        .filter(|server| server.effective_enabled)
        .map(|server| server.server_code)
        .collect::<std::collections::HashSet<_>>();
    for server_code in server_codes {
        if !effective_codes.contains(server_code) {
            return Err(
                crate::domain::agent::AgentManagementError::InternalMcpServerAssignmentStale(
                    server_code.clone(),
                ),
            );
        }
    }
    Ok(())
}

fn map_mcp_assignment_error(
    error: crate::domain::mcp::McpError,
) -> crate::domain::agent::AgentManagementError {
    match error {
        crate::domain::mcp::McpError::InvalidWorkspace
        | crate::domain::mcp::McpError::WorkspaceRequired => {
            crate::domain::agent::AgentManagementError::InvalidWorkspace
        }
        _ => crate::domain::agent::AgentManagementError::StorageFailure,
    }
}

fn normalize_mode_token(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace('-', "_")
}

pub async fn list_agui_stream_events(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(stream_key): Path<String>,
) -> Response {
    let service = AgentAguiStreamService::with_replay_store(
        state.agui_events.clone(),
        state.agui_replay_store.clone(),
    );
    match service.events(&stream_key) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => agent_agui_error_response(error),
    }
}

pub async fn list_agui_stream_protocol_events(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(stream_key): Path<String>,
) -> Response {
    let service = AgentAguiStreamService::with_replay_store(
        state.agui_events.clone(),
        state.agui_replay_store.clone(),
    );
    match service.protocol_events(&stream_key) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => agent_agui_error_response(error),
    }
}
