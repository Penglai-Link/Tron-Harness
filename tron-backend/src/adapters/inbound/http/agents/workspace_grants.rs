use super::internal_definitions::InternalAgentApiError;
use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    response::{ApiResponse, EmptyData},
};
use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignWorkspacesRequest {
    workspace_codes: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkspacesVo {
    workspace_codes: Vec<String>,
}
pub async fn list(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(agent): Path<String>,
) -> Result<Json<ApiResponse<AgentWorkspacesVo>>, InternalAgentApiError> {
    let workspace_codes = state
        .agent_management
        .agent_workspace_grants(&agent)
        .map_err(|error| InternalAgentApiError(error, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(AgentWorkspacesVo {
        workspace_codes,
    })))
}
pub async fn replace(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(agent): Path<String>,
    Json(q): Json<AssignWorkspacesRequest>,
) -> Result<Json<ApiResponse<EmptyData>>, InternalAgentApiError> {
    state
        .agent_management
        .assign_agent_workspaces(&agent, q.workspace_codes)
        .map_err(|error| InternalAgentApiError(error, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(EmptyData {})))
}
