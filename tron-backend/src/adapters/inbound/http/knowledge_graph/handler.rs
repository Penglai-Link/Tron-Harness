use axum::{
    Json,
    extract::{Query, State},
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    application::{graph::GraphExplorerService, workspace::WorkspaceService},
    response::ApiResponse,
};

use super::{
    error_mapper::{
        knowledge_graph_error_response, knowledge_graph_request_invalid,
        knowledge_graph_scope_forbidden,
    },
    request::{
        KnowledgeGraphExpandRequest, KnowledgeGraphNamespaceQuery, KnowledgeGraphQueryRequest,
        KnowledgeGraphSnapshotQuery,
    },
    response::KnowledgeGraphExplorerResponse,
};

pub async fn list_namespaces(
    State(workspaces): State<WorkspaceService>,
    State(explorer): State<GraphExplorerService>,
    auth: RequestAuthContext,
    Query(query): Query<KnowledgeGraphNamespaceQuery>,
) -> Response {
    if query.workspace_code.trim().is_empty() {
        return knowledge_graph_request_invalid();
    }
    if !workspace_is_active(&workspaces, &query.workspace_code) {
        return knowledge_graph_scope_forbidden();
    }

    match explorer.list_namespaces(query.workspace_code, auth.user().user_code.clone()) {
        Ok(namespaces) => Json(ApiResponse::success(namespaces)).into_response(),
        Err(error) => knowledge_graph_error_response(error),
    }
}

pub async fn snapshot(
    State(workspaces): State<WorkspaceService>,
    State(explorer): State<GraphExplorerService>,
    auth: RequestAuthContext,
    Query(query): Query<KnowledgeGraphSnapshotQuery>,
) -> Response {
    if query.namespace.trim().is_empty() || query.workspace_code.trim().is_empty() {
        return knowledge_graph_request_invalid();
    }
    if !workspace_is_active(&workspaces, &query.workspace_code) {
        return knowledge_graph_scope_forbidden();
    }

    match explorer.snapshot(
        query.workspace_code,
        query.namespace,
        auth.user().user_code.clone(),
    ) {
        Ok(snapshot) => Json(ApiResponse::success(KnowledgeGraphExplorerResponse::from(
            snapshot,
        )))
        .into_response(),
        Err(error) => knowledge_graph_error_response(error),
    }
}

pub async fn query(
    State(workspaces): State<WorkspaceService>,
    State(explorer): State<GraphExplorerService>,
    auth: RequestAuthContext,
    Json(request): Json<KnowledgeGraphQueryRequest>,
) -> Response {
    let command = match request.into_command(auth.user().user_code.clone()) {
        Ok(command) => command,
        Err(_) => return knowledge_graph_request_invalid(),
    };
    if !workspace_is_active(&workspaces, &command.workspace_code) {
        return knowledge_graph_scope_forbidden();
    }

    match explorer.query(command) {
        Ok(snapshot) => Json(ApiResponse::success(KnowledgeGraphExplorerResponse::from(
            snapshot,
        )))
        .into_response(),
        Err(error) => knowledge_graph_error_response(error),
    }
}

pub async fn expand(
    State(workspaces): State<WorkspaceService>,
    State(explorer): State<GraphExplorerService>,
    auth: RequestAuthContext,
    Json(request): Json<KnowledgeGraphExpandRequest>,
) -> Response {
    let command = match request.into_command(auth.user().user_code.clone()) {
        Ok(command) => command,
        Err(_) => return knowledge_graph_request_invalid(),
    };
    if !workspace_is_active(&workspaces, &command.workspace_code) {
        return knowledge_graph_scope_forbidden();
    }

    match explorer.expand(command) {
        Ok(snapshot) => Json(ApiResponse::success(KnowledgeGraphExplorerResponse::from(
            snapshot,
        )))
        .into_response(),
        Err(error) => knowledge_graph_error_response(error),
    }
}

fn workspace_is_active(workspaces: &WorkspaceService, workspace_code: &str) -> bool {
    workspaces
        .workspace_is_active(workspace_code)
        .unwrap_or(false)
}
