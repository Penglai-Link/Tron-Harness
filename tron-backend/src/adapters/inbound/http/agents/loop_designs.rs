use super::internal_definitions::InternalAgentApiError;
use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    domain::agent::{AgentManagementError, loop_design::*},
    response::{ApiResponse, EmptyData},
};
use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateLoopRequest {
    loop_code: String,
    name: String,
    kind: LoopKind,
    nodes: Vec<LoopNode>,
    edges: Vec<LoopEdge>,
    max_parallel_nodes: u32,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateLoopRequest {
    name: String,
    kind: LoopKind,
    nodes: Vec<LoopNode>,
    edges: Vec<LoopEdge>,
    max_parallel_nodes: u32,
    expected_version: u32,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopView {
    loop_code: String,
    name: String,
    kind: LoopKind,
    version: u32,
    nodes: Vec<LoopNode>,
    edges: Vec<LoopEdge>,
    max_parallel_nodes: u32,
    updated_at: i64,
}
impl From<LoopDesign> for LoopView {
    fn from(d: LoopDesign) -> Self {
        Self {
            loop_code: d.loop_code,
            name: d.name,
            kind: d.kind,
            version: d.version,
            nodes: d.nodes,
            edges: d.edges,
            max_parallel_nodes: d.max_parallel_nodes,
            updated_at: d.updated_at,
        }
    }
}
#[derive(Serialize)]
#[serde(transparent)]
pub struct CreatedLoopView(LoopView);
#[derive(Serialize)]
#[serde(transparent)]
pub struct UpdatedLoopView(LoopView);
pub async fn list(
    State(state): State<AppState>,
    auth: RequestAuthContext,
) -> Result<Json<ApiResponse<Vec<LoopView>>>, InternalAgentApiError> {
    let records = state
        .agent_management
        .list_loop_designs()
        .map_err(|e| InternalAgentApiError(e, auth.locale))?;
    Ok(Json(ApiResponse::success(
        records.into_iter().map(Into::into).collect(),
    )))
}
pub async fn create(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Json(r): Json<CreateLoopRequest>,
) -> Result<Json<ApiResponse<CreatedLoopView>>, InternalAgentApiError> {
    let d = LoopDesign {
        loop_code: r.loop_code,
        name: r.name,
        kind: r.kind,
        version: 0,
        nodes: r.nodes,
        edges: r.edges,
        max_parallel_nodes: r.max_parallel_nodes,
        updated_at: 0,
    };
    let d = state
        .agent_management
        .save_loop_design(d, 0)
        .map_err(|e| InternalAgentApiError(e, auth.locale))?;
    Ok(Json(ApiResponse::success(CreatedLoopView(d.into()))))
}
pub async fn update(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(code): Path<String>,
    Json(r): Json<UpdateLoopRequest>,
) -> Result<Json<ApiResponse<UpdatedLoopView>>, InternalAgentApiError> {
    if r.expected_version == 0 {
        return Err(InternalAgentApiError(
            AgentManagementError::LoopInvalid,
            auth.locale,
        ));
    }
    let d = LoopDesign {
        loop_code: code,
        name: r.name,
        kind: r.kind,
        version: 0,
        nodes: r.nodes,
        edges: r.edges,
        max_parallel_nodes: r.max_parallel_nodes,
        updated_at: 0,
    };
    let d = state
        .agent_management
        .save_loop_design(d, r.expected_version)
        .map_err(|e| InternalAgentApiError(e, auth.locale))?;
    Ok(Json(ApiResponse::success(UpdatedLoopView(d.into()))))
}
pub async fn delete(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<EmptyData>>, InternalAgentApiError> {
    state
        .agent_management
        .delete_loop_design(&code)
        .map_err(|e| InternalAgentApiError(e, auth.locale))?;
    Ok(Json(ApiResponse::success(EmptyData {})))
}
