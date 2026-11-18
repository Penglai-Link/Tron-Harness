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
        use tron_i18n::t;
        let descriptor = self.0.descriptor();
        let status = match self.0 {
            RagTaskChunksError::NotFound => StatusCode::NOT_FOUND,
            RagTaskChunksError::InvalidPagination => StatusCode::BAD_REQUEST,
            RagTaskChunksError::Unavailable => StatusCode::CONFLICT,
            RagTaskChunksError::StorageFailure => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let message = match descriptor.message_key {
            "tron.rag.task_chunks_not_found" => {
                t!("tron.rag.task_chunks_not_found", locale = &self.1)
            }
            "tron.rag.task_chunks_pagination_invalid" => {
                t!("tron.rag.task_chunks_pagination_invalid", locale = &self.1)
            }
            "tron.rag.task_chunks_unavailable" => {
                t!("tron.rag.task_chunks_unavailable", locale = &self.1)
            }
            _ => t!("tron.rag.task_chunks_storage_failed", locale = &self.1),
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
