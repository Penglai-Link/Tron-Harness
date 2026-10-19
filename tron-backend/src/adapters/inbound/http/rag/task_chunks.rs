use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::rag::RagTaskChunksError,
    response::{ApiResponse, EmptyData},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

// Rust/FE consensus §§2.1,2.2,2.4-2.6: dedicated DTO/VO, bounded paging, localized errors.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TaskChunksQuery {
    current: usize,
    size: usize,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TaskChunksPage {
    current: usize,
    size: usize,
    total: usize,
    records: Vec<TaskChunkView>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskChunkView {
    text_unit_code: String,
    ordinal: i64,
    text: String,
    heading_path: Vec<String>,
    metadata: serde_json::Value,
}
pub(super) struct TaskChunksApiError(RagTaskChunksError, String);
impl IntoResponse for TaskChunksApiError {
    fn into_response(self) -> Response {
        let descriptor = self.0.descriptor();
        let (status, zh, en) = match self.0 {
            RagTaskChunksError::NotFound => (
                StatusCode::NOT_FOUND,
                "任务记录不存在",
                "Task record not found",
            ),
            RagTaskChunksError::InvalidPagination => (
                StatusCode::BAD_REQUEST,
                "分块分页参数无效",
                "Invalid chunk pagination",
            ),
            RagTaskChunksError::Unavailable => (
                StatusCode::CONFLICT,
                "请查看最新已完成任务的分块",
                "View chunks from the latest completed task",
            ),
            RagTaskChunksError::StorageFailure => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "分块读取失败",
                "Could not load chunks",
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
pub(super) async fn chunks(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
    query: Result<Query<TaskChunksQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<ApiResponse<TaskChunksPage>>, TaskChunksApiError> {
    let error = |cause| TaskChunksApiError(cause, auth.locale.clone());
    let Query(query) = query.map_err(|_| error(RagTaskChunksError::InvalidPagination))?;
    let page = state
        .rag
        .record_task_chunks(&auth.user().user_code, &id, query.current, query.size)
        .map_err(error)?;
    Ok(Json(ApiResponse::protocol_success(TaskChunksPage {
        current: page.page,
        size: page.page_size,
        total: page.total,
        records: page
            .items
            .into_iter()
            .map(|item| TaskChunkView {
                text_unit_code: item.text_unit_code,
                ordinal: item.ordinal,
                text: item.text,
                heading_path: item.heading_path,
                metadata: item.metadata,
            })
            .collect(),
    })))
}
