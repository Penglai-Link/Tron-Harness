use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::rag::RagError, response::ApiResponse};

pub fn rag_error_response(error: RagError, locale: &str) -> Response {
    use tron_i18n::t;
    if let RagError::EmbeddingFailure(detail) = &error {
        // The detail is a locale-independent structured message produced by the
        // indexing layer; legacy plain text is rendered unchanged.
        let detail = crate::shared::i18n_message::StoredMessage::decode(detail.as_str())
            .render(locale, None);
        return (
            StatusCode::BAD_GATEWAY,
            Json(ApiResponse::error(
                "RAG_EMBEDDING_FAILED",
                t!(
                    "rag.error.embedding_failure",
                    locale = locale,
                    detail = detail
                ),
            )),
        )
            .into_response();
    }
    let status = match &error {
        RagError::SchemaIncompatible => StatusCode::CONFLICT,
        RagError::IngestRequestInvalid
        | RagError::CorpusNotFound
        | RagError::StorageBindingInvalid => StatusCode::BAD_REQUEST,
        RagError::IngestScopeInvalid | RagError::SearchScopeInvalid => StatusCode::FORBIDDEN,
        RagError::EmbeddingFailed | RagError::EmbeddingFailure(_) => StatusCode::BAD_GATEWAY,
        RagError::SearchRequestInvalid
        | RagError::SearchFilterInvalid
        | RagError::DebugRequestInvalid
        | RagError::TracePayloadInvalid => StatusCode::BAD_REQUEST,
        RagError::SearchRouteUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        RagError::SourceNotFound => StatusCode::NOT_FOUND,
        RagError::StorageFailure => StatusCode::INTERNAL_SERVER_ERROR,
    };
    let code = match &error {
        RagError::SchemaIncompatible => "RAG_SCHEMA_INCOMPATIBLE",
        RagError::IngestRequestInvalid => "RAG_INGEST_REQUEST_INVALID",
        RagError::IngestScopeInvalid => "RAG_INGEST_SCOPE_INVALID",
        RagError::EmbeddingFailed | RagError::EmbeddingFailure(_) => "RAG_EMBEDDING_FAILED",
        RagError::SearchRequestInvalid => "RAG_SEARCH_REQUEST_INVALID",
        RagError::SearchFilterInvalid => "RAG_SEARCH_FILTER_INVALID",
        RagError::SearchScopeInvalid => "RAG_SEARCH_SCOPE_INVALID",
        RagError::SearchRouteUnavailable => "RAG_SEARCH_ROUTE_UNAVAILABLE",
        RagError::DebugRequestInvalid => "RAG_DEBUG_REQUEST_INVALID",
        RagError::TracePayloadInvalid => "RAG_TRACE_PAYLOAD_INVALID",
        RagError::SourceNotFound => "RAG_SOURCE_NOT_FOUND",
        RagError::CorpusNotFound | RagError::StorageBindingInvalid => "RAG_INGEST_REQUEST_INVALID",
        RagError::StorageFailure => "RAG_INGEST_STAGE_FAILED",
    };
    let message = match &error {
        RagError::SchemaIncompatible => {
            t!("rag.error.schema_incompatible", locale = locale).into_owned()
        }
        RagError::IngestRequestInvalid
        | RagError::CorpusNotFound
        | RagError::StorageBindingInvalid => {
            t!("rag.error.ingest_request_invalid", locale = locale).into_owned()
        }
        RagError::IngestScopeInvalid => {
            t!("rag.error.ingest_scope_invalid", locale = locale).into_owned()
        }
        RagError::EmbeddingFailed | RagError::EmbeddingFailure(_) => {
            t!("rag.error.embedding_failed", locale = locale).into_owned()
        }
        RagError::SearchRequestInvalid => {
            t!("rag.error.search_request_invalid", locale = locale).into_owned()
        }
        RagError::SearchFilterInvalid => {
            t!("rag.error.search_filter_invalid", locale = locale).into_owned()
        }
        RagError::SearchScopeInvalid => {
            t!("rag.error.search_scope_invalid", locale = locale).into_owned()
        }
        RagError::SearchRouteUnavailable => {
            t!("rag.error.search_route_unavailable", locale = locale).into_owned()
        }
        RagError::DebugRequestInvalid => {
            t!("rag.error.debug_request_invalid", locale = locale).into_owned()
        }
        RagError::TracePayloadInvalid => {
            t!("rag.error.trace_payload_invalid", locale = locale).into_owned()
        }
        RagError::SourceNotFound => t!("rag.error.source_not_found", locale = locale).into_owned(),
        RagError::StorageFailure => t!("rag.error.storage_failed", locale = locale).into_owned(),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
