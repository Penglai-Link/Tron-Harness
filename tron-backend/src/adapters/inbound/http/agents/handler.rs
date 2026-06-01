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
        AgentManagementService,
    },
    domain::agent::AgentRunHistoryQuery,
    response::ApiResponse,
};

use super::error_mapper::{
    agent_agui_error_response, agent_error_response, agent_history_error_response,
    agent_management_error_response,
};
use super::request::UpdateMainAgentConfigRequest;

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
        state.event_bus.clone(),
        state.agent_model_client.clone(),
        state.agent_tool_registry.clone(),
        state.runtime_http_client.clone(),
    );
    match service
        .invoke(request, Some(auth.user().user_code.clone()))
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
