use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::graph::GraphError, response::ApiResponse};

pub fn knowledge_graph_request_invalid(locale: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::error(
            "KNOWLEDGE_GRAPH_REQUEST_INVALID",
            tron_i18n::t!("knowledge_graph.request_invalid", locale = locale),
        )),
    )
        .into_response()
}

pub fn knowledge_graph_scope_forbidden(locale: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(ApiResponse::error(
            "KNOWLEDGE_GRAPH_SCOPE_FORBIDDEN",
            tron_i18n::t!("knowledge_graph.scope_forbidden", locale = locale),
        )),
    )
        .into_response()
}

pub fn knowledge_graph_error_response(error: GraphError, locale: &str) -> Response {
    let (status, code, message_key) = match error {
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
            "knowledge_graph.request_invalid",
        ),
        GraphError::MissingNode { .. } | GraphError::MissingEdge { .. } => (
            StatusCode::NOT_FOUND,
            "KNOWLEDGE_GRAPH_NOT_FOUND",
            "knowledge_graph.not_found",
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
            "knowledge_graph.storage_failed",
        ),
    };

    (
        status,
        Json(ApiResponse::error(
            code,
            tron_i18n::t!(message_key, locale = locale),
        )),
    )
        .into_response()
}
