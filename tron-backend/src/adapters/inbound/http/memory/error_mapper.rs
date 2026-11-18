use crate::{
    application::memory::MemoryError, response::ApiResponse, shared::locale::SupportedLocale,
};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
pub struct ApiError(pub MemoryError, pub SupportedLocale);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let d = self.0.descriptor();
        let status = match self.0 {
            MemoryError::Validation(_) => StatusCode::BAD_REQUEST,
            MemoryError::NotFound(_) => StatusCode::NOT_FOUND,
            MemoryError::Forbidden(_) => StatusCode::FORBIDDEN,
            MemoryError::Conflict(_) => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let message_key = match d.message_key {
            "tron.memory.invalid" => "tron.memory.invalid",
            "tron.memory.not_found" => "tron.memory.not_found",
            "tron.memory.forbidden" => "tron.memory.forbidden",
            "tron.memory.conflict" => "tron.memory.conflict",
            "tron.memory.index" => "tron.memory.index",
            _ => "tron.memory.storage",
        };
        (
            status,
            Json(ApiResponse::protocol_error(
                d.code,
                tron_i18n::t!(message_key, locale = self.1),
            )),
        )
            .into_response()
    }
}
