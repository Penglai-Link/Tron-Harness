use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{application::memory::MemoryError, response::ApiResponse};

pub fn memory_error_response(error: MemoryError) -> Response {
    let (status, code, message) = match error {
        MemoryError::Validation(message) => {
            (StatusCode::BAD_REQUEST, "MEMORY_REQUEST_INVALID", message)
        }
        MemoryError::NotFound(message) => (StatusCode::NOT_FOUND, "MEMORY_NOT_FOUND", message),
        MemoryError::Forbidden(message) => (StatusCode::FORBIDDEN, "MEMORY_FORBIDDEN", message),
        MemoryError::Conflict(message) => (StatusCode::CONFLICT, "MEMORY_CONFLICT", message),
        MemoryError::Storage(message) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "MEMORY_STORAGE_FAILED",
            message,
        ),
        MemoryError::Rag(message) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "MEMORY_RAG_FAILED",
            message,
        ),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
