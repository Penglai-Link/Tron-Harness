use crate::{
    application::rag::graph::GraphQueryError,
    response::{ApiResponse, EmptyData},
};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
pub(super) struct GraphApiError(pub GraphQueryError, pub String);
impl GraphApiError {
    pub(super) fn invalid(locale: &str) -> Self {
        Self(GraphQueryError::Invalid, locale.into())
    }
}
impl IntoResponse for GraphApiError {
    fn into_response(self) -> Response {
        use tron_i18n::t;
        let descriptor = self.0.descriptor();
        let status = match descriptor.message_key {
            "tron.rag.graph.invalid_request" => StatusCode::BAD_REQUEST,
            "tron.rag.graph.scope_denied" => StatusCode::FORBIDDEN,
            "tron.rag.graph.node_not_found" => StatusCode::NOT_FOUND,
            "tron.rag.graph.snapshot_changed" | "tron.rag.graph.disabled" => StatusCode::CONFLICT,
            "tron.rag.graph.channel_unavailable" => StatusCode::BAD_GATEWAY,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let message = match descriptor.message_key {
            "tron.rag.graph.invalid_request" => {
                t!("tron.rag.graph.invalid_request", locale = &self.1)
            }
            "tron.rag.graph.scope_denied" => {
                t!("tron.rag.graph.scope_denied", locale = &self.1)
            }
            "tron.rag.graph.node_not_found" => {
                t!("tron.rag.graph.node_not_found", locale = &self.1)
            }
            "tron.rag.graph.snapshot_changed" => {
                t!("tron.rag.graph.snapshot_changed", locale = &self.1)
            }
            "tron.rag.graph.channel_unavailable" => {
                t!("tron.rag.graph.channel_unavailable", locale = &self.1)
            }
            "tron.rag.graph.disabled" => {
                t!("tron.rag.graph.disabled", locale = &self.1)
            }
            _ => t!("tron.rag.graph.storage_failed", locale = &self.1),
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
