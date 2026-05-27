use axum::{
    Json,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::agent::{AgentAguiStreamService, AgentInvocationCommand, AgentInvocationService},
    domain::agent::AgentRunHistoryQuery,
    response::ApiResponse,
};

use super::error_mapper::{
    agent_agui_error_response, agent_error_response, agent_history_error_response,
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
        state.agent_runs.clone(),
        state.event_bus.clone(),
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
    let service = AgentAguiStreamService::new(state.agui_events.clone());
    match service.activate(&stream_key) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => agent_agui_error_response(error),
    }
}

pub async fn deactivate_agui_stream(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(stream_key): Path<String>,
) -> Response {
    let service = AgentAguiStreamService::new(state.agui_events.clone());
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
    let service = AgentAguiStreamService::new(state.agui_events.clone());
    match service.events(&stream_key) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => agent_agui_error_response(error),
    }
}
