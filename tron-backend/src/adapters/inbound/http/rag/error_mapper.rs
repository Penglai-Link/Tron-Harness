use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::rag::RagError, response::ApiResponse};

pub fn rag_error_response(error: RagError) -> Response {
    let (status, code, message) = match error {
        RagError::IngestRequestInvalid => (
            StatusCode::BAD_REQUEST,
            "RAG_INGEST_REQUEST_INVALID",
            "rag ingest request is invalid",
        ),
        RagError::IngestProfileInvalid => (
            StatusCode::BAD_REQUEST,
            "RAG_INGEST_PROFILE_INVALID",
            "rag ingest profile is invalid",
        ),
        RagError::IngestScopeInvalid => (
            StatusCode::FORBIDDEN,
            "RAG_INGEST_SCOPE_INVALID",
            "rag ingest target scope is invalid",
        ),
        RagError::IngestStageFailed => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "RAG_INGEST_STAGE_FAILED",
            "rag ingest stage failed",
        ),
        RagError::EmbeddingFailed => (
            StatusCode::BAD_GATEWAY,
            "RAG_EMBEDDING_FAILED",
            "rag embedding failed",
        ),
        RagError::SearchRequestInvalid => (
            StatusCode::BAD_REQUEST,
            "RAG_SEARCH_REQUEST_INVALID",
            "rag search request is invalid",
        ),
        RagError::RetrievalProfileInvalid => (
            StatusCode::BAD_REQUEST,
            "RAG_RETRIEVAL_PROFILE_INVALID",
            "rag retrieval profile is invalid",
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
        RagError::HydrateSourceInactive => (
            StatusCode::CONFLICT,
            "RAG_HYDRATE_SOURCE_INACTIVE",
            "rag hydrate source is inactive",
        ),
        RagError::RerankFailed => (
            StatusCode::BAD_GATEWAY,
            "RAG_RERANK_FAILED",
            "rag rerank failed",
        ),
        RagError::CorpusReserved => (
            StatusCode::BAD_REQUEST,
            "RAG_CORPUS_RESERVED",
            "rag corpus is reserved",
        ),
        RagError::ProfileNotFound => (
            StatusCode::NOT_FOUND,
            "RAG_INGEST_PROFILE_NOT_FOUND",
            "rag ingest profile was not found",
        ),
        RagError::RetrievalProfileNotFound => (
            StatusCode::NOT_FOUND,
            "RAG_RETRIEVAL_PROFILE_NOT_FOUND",
            "rag retrieval profile was not found",
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
