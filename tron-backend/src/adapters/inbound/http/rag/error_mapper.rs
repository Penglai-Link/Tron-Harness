use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::rag::RagError, response::ApiResponse};

pub fn rag_error_response(error: RagError) -> Response {
    if let RagError::EmbeddingFailure(_) = &error {
        return (
            StatusCode::BAD_GATEWAY,
            Json(ApiResponse::error(
                "RAG_EMBEDDING_FAILED",
                error.to_string(),
            )),
        )
            .into_response();
    }
    let (status, code, message) = match error {
        RagError::SchemaIncompatible => (
            StatusCode::CONFLICT,
            "RAG_SCHEMA_INCOMPATIBLE",
            "旧 RAG 数据结构不兼容，请使用新数据库；现有数据未删除",
        ),
        RagError::IngestRequestInvalid => (
            StatusCode::BAD_REQUEST,
            "RAG_INGEST_REQUEST_INVALID",
            "rag ingest request is invalid",
        ),
        RagError::IngestScopeInvalid => (
            StatusCode::FORBIDDEN,
            "RAG_INGEST_SCOPE_INVALID",
            "rag ingest target scope is invalid",
        ),
        RagError::EmbeddingFailed | RagError::EmbeddingFailure(_) => (
            StatusCode::BAD_GATEWAY,
            "RAG_EMBEDDING_FAILED",
            "rag embedding failed",
        ),
        RagError::SearchRequestInvalid => (
            StatusCode::BAD_REQUEST,
            "RAG_SEARCH_REQUEST_INVALID",
            "rag search request is invalid",
        ),
        RagError::SearchFilterInvalid => (
            StatusCode::BAD_REQUEST,
            "RAG_SEARCH_FILTER_INVALID",
            "rag search filter is invalid",
        ),
        RagError::SearchScopeInvalid => (
            StatusCode::FORBIDDEN,
            "RAG_SEARCH_SCOPE_INVALID",
            "rag search scope is invalid",
        ),
        RagError::SearchRouteUnavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "RAG_SEARCH_ROUTE_UNAVAILABLE",
            "rag search route is unavailable",
        ),
        RagError::DebugRequestInvalid => (
            StatusCode::BAD_REQUEST,
            "RAG_DEBUG_REQUEST_INVALID",
            "rag debug request is invalid",
        ),
        RagError::TracePayloadInvalid => (
            StatusCode::BAD_REQUEST,
            "RAG_TRACE_PAYLOAD_INVALID",
            "rag trace payload is invalid",
        ),
        RagError::SourceNotFound => (
            StatusCode::NOT_FOUND,
            "RAG_SOURCE_NOT_FOUND",
            "rag source was not found",
        ),
        RagError::CorpusNotFound | RagError::StorageBindingInvalid => (
            StatusCode::BAD_REQUEST,
            "RAG_INGEST_REQUEST_INVALID",
            "rag ingest request is invalid",
        ),
        RagError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "RAG_INGEST_STAGE_FAILED",
            "rag ingest storage failed",
        ),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
