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
        let detail = match &self.0 {
            SkinError::CssRejected(detail) | SkinError::Incompatible(detail) => {
                Some(detail.render(&self.1))
            }
            _ => None,
        };
        let status = match self.0 {
            SkinError::Invalid => StatusCode::BAD_REQUEST,
            SkinError::CssRejected(_) => StatusCode::BAD_REQUEST,
            SkinError::Incompatible(_) => StatusCode::BAD_REQUEST,
            SkinError::NotFound => StatusCode::NOT_FOUND,
            SkinError::Conflict => StatusCode::CONFLICT,
            SkinError::Protected => StatusCode::FORBIDDEN,
            SkinError::Storage => StatusCode::INTERNAL_SERVER_ERROR,
            SkinError::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        };
        use tron_i18n::t;
        let message: String = if let Some(detail) = detail {
            // Structured reason rendered with the request locale.
            detail
        } else {
            match descriptor.message_key {
                "tron.skin.invalid" => t!("tron.skin.invalid", locale = &self.1).into_owned(),
                "tron.skin.not_found" => t!("tron.skin.not_found", locale = &self.1).into_owned(),
                "tron.skin.conflict" => t!("tron.skin.conflict", locale = &self.1).into_owned(),
                "tron.skin.protected" => t!("tron.skin.protected", locale = &self.1).into_owned(),
                "tron.skin.too_large" => t!("tron.skin.too_large", locale = &self.1).into_owned(),
                _ => t!("tron.skin.storage_failed", locale = &self.1).into_owned(),
            }
        };
        (
            status,
            Json(ApiResponse::<EmptyData>::protocol_error(
                descriptor.code,
                message,
            )),
        )
            .into_response()
    }
}
