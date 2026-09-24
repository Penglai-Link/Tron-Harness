use crate::{
    application::skin::SkinError,
    response::{ApiResponse, EmptyData},
};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub struct SkinApiError(pub SkinError, pub String);
impl IntoResponse for SkinApiError {
    fn into_response(self) -> Response {
        let descriptor = self.0.descriptor();
        let status = match self.0 {
            SkinError::Invalid => StatusCode::BAD_REQUEST,
            SkinError::NotFound => StatusCode::NOT_FOUND,
            SkinError::Conflict => StatusCode::CONFLICT,
            SkinError::Storage => StatusCode::INTERNAL_SERVER_ERROR,
            SkinError::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        };
        let (zh, en) = match descriptor.message_key {
            "tron.skin.invalid" => ("皮肤包格式或资源无效", "Invalid skin package or resource"),
            "tron.skin.not_found" => ("皮肤或资源不存在", "Skin or resource not found"),
            "tron.skin.conflict" => (
                "皮肤 ID 已存在，请修改 ID 后导入",
                "Skin ID already exists; use a different ID",
            ),
            "tron.skin.too_large" => (
                "皮肤包不能超过 20 MB",
                "Skin packages must be smaller than 20 MB",
            ),
            _ => ("皮肤文件读写失败", "Could not read or write skin files"),
        };
        (
            status,
            Json(ApiResponse::<EmptyData>::protocol_error(
                descriptor.code,
                if self.1.starts_with("en") { en } else { zh },
            )),
        )
            .into_response()
    }
}
