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
        let (zh, en) = match d.message_key {
            "tron.memory.invalid" => (
                "记忆内容、范围或分页参数无效",
                "Invalid memory content, scope or pagination",
            ),
            "tron.memory.not_found" => ("记忆不存在", "Memory not found"),
            "tron.memory.forbidden" => ("无权访问此记忆范围", "Memory scope is not accessible"),
            "tron.memory.conflict" => (
                "记忆已变更或存在不同内容，请刷新后重新编辑",
                "Memory changed or contains conflicting content; refresh before editing",
            ),
            "tron.memory.index" => ("记忆索引暂不可用", "Memory index is unavailable"),
            _ => ("记忆存储失败", "Could not access memory storage"),
        };
        (
            status,
            Json(ApiResponse::protocol_error(
                d.code,
                if self.1 == SupportedLocale::EnUs {
                    en
                } else {
                    zh
                },
            )),
        )
            .into_response()
    }
}
