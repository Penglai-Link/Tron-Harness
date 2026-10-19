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
        let descriptor = self.0.descriptor();
        let (status, zh, en) = match descriptor.message_key {
            "tron.rag.graph.invalid_request" => (
                StatusCode::BAD_REQUEST,
                "图谱查询参数无效",
                "Invalid graph query parameters",
            ),
            "tron.rag.graph.scope_denied" => (
                StatusCode::FORBIDDEN,
                "无权查询该语料图谱",
                "Access to this corpus graph is denied",
            ),
            "tron.rag.graph.node_not_found" => (
                StatusCode::NOT_FOUND,
                "节点不存在或不在可访问范围内",
                "Node not found in the accessible scope",
            ),
            "tron.rag.graph.snapshot_changed" => (
                StatusCode::CONFLICT,
                "授权、发布版本或查询游标已变化，请重新检索",
                "Authorization, publication or cursor has changed; search again",
            ),
            "tron.rag.graph.channel_unavailable" => (
                StatusCode::BAD_GATEWAY,
                "图谱检索通道暂不可用，请稍后重试",
                "Graph retrieval is unavailable; try again later",
            ),
            "tron.rag.graph.disabled" => (
                StatusCode::CONFLICT,
                "该语料尚未启用图谱",
                "Graph indexing is disabled for this corpus",
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "图谱读取失败",
                "Could not read graph data",
            ),
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
