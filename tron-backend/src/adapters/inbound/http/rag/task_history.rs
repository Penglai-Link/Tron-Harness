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
        use tron_i18n::t;
        let descriptor = self.0.descriptor();
        let status = match descriptor.message_key {
            "tron.rag.task_history_not_found" => StatusCode::NOT_FOUND,
            "tron.rag.task_history_active" => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let message = match descriptor.message_key {
            "tron.rag.task_history_not_found" => {
                t!("tron.rag.task_history_not_found", locale = &self.1)
            }
            "tron.rag.task_history_active" => {
                t!("tron.rag.task_history_active", locale = &self.1)
            }
            _ => t!("tron.rag.task_history_storage_failed", locale = &self.1),
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
