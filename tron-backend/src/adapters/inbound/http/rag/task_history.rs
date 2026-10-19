use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::rag::{RagConfigRepository, RagTaskHistoryError},
    response::{ApiResponse, EmptyData},
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

// Rust/FE consensus §§2.1,2.2,2.5,2.6: dedicated VO, REST DELETE,
// numeric protocol codes, localized errors, and no deletion of retrieval data.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DeleteTaskView {
    deleted: bool,
}
pub(super) struct TaskHistoryApiError(RagTaskHistoryError, String);
impl IntoResponse for TaskHistoryApiError {
    fn into_response(self) -> Response {
        let descriptor = self.0.descriptor();
        let (status, zh, en) = match descriptor.message_key {
            "tron.rag.task_history_not_found" => (
                StatusCode::NOT_FOUND,
                "任务记录不存在",
                "Task record not found",
            ),
            "tron.rag.task_history_active" => (
                StatusCode::CONFLICT,
                "请先取消运行中的任务，再删除记录",
                "Cancel the active task before deleting its record",
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "任务记录删除失败",
                "Could not delete the task record",
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
pub(super) async fn delete_task(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<DeleteTaskView>>, TaskHistoryApiError> {
    let user = &auth.user().user_code;
    let storage_error =
        |_| TaskHistoryApiError(RagTaskHistoryError::StorageFailure, auth.locale.clone());
    // Ownership is the same as listing, including built-in skill records shared by SYSTEM.
    let mut tasks = state.rag.record_task_views(user).map_err(storage_error)?;
    tasks.extend(state.knowledge_bases.rag_task_views(user).map_err(|_| {
        TaskHistoryApiError(RagTaskHistoryError::StorageFailure, auth.locale.clone())
    })?);
    let task = tasks
        .iter()
        .find(|task| task.id == id)
        .ok_or_else(|| TaskHistoryApiError(RagTaskHistoryError::NotFound, auth.locale.clone()))?;
    if !matches!(task.state.as_str(), "COMPLETED" | "FAILED" | "CANCELLED") {
        return Err(TaskHistoryApiError(
            RagTaskHistoryError::Active,
            auth.locale.clone(),
        ));
    }
    state
        .rag
        .repository_handle()
        .delete_task_history(user, &id)
        .map_err(storage_error)?;
    Ok(Json(ApiResponse::protocol_success(DeleteTaskView {
        deleted: true,
    })))
}
