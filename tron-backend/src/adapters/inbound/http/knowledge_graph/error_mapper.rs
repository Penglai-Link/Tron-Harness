use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::graph::GraphError, response::ApiResponse};

pub fn knowledge_graph_request_invalid() -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::error(
            "KNOWLEDGE_GRAPH_REQUEST_INVALID",
            "knowledge graph request is invalid",
        )),
    )
        .into_response()
}

pub fn knowledge_graph_scope_forbidden() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(ApiResponse::error(
            "KNOWLEDGE_GRAPH_SCOPE_FORBIDDEN",
            "knowledge graph scope is forbidden",
        )),
    )
        .into_response()
}

pub fn knowledge_graph_error_response(error: GraphError) -> Response {
    let (status, code, message) = match error {
        GraphError::InvalidGraphCode
        | GraphError::LabelRequired
        | GraphError::RelationshipRequired
        | GraphError::ReservedPropertyName
        | GraphError::EmptySearchQuery
        | GraphError::InvalidTraversalQuery
        | GraphError::RawQueryNotAllowed
        | GraphError::CallerSuppliedVectorNotAllowed
        | GraphError::Scope(_) => (
            StatusCode::BAD_REQUEST,
            "KNOWLEDGE_GRAPH_REQUEST_INVALID",
            "knowledge graph request is invalid",
        ),
        GraphError::MissingNode { .. } | GraphError::MissingEdge { .. } => (
            StatusCode::NOT_FOUND,
            "KNOWLEDGE_GRAPH_NOT_FOUND",
            "knowledge graph resource was not found",
        ),
        GraphError::DuplicateNode { .. }
        | GraphError::DuplicateEdge { .. }
        | GraphError::MissingEndpoint { .. }
        | GraphError::EmbeddingUnavailable
        | GraphError::EmbeddingProviderFailure
        | GraphError::UnsupportedStoreBehavior
        | GraphError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "KNOWLEDGE_GRAPH_STORAGE_FAILED",
            "knowledge graph storage failed",
        ),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
