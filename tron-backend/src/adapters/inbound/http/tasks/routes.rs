use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{
    archive_task, create_task, execute_task, get_task, kanban, list_tasks, options, patch_status,
    patch_task, restore_task,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/tasks/options", get(options))
        .route("/api/v1/tasks/kanban", get(kanban))
        .route("/api/v1/tasks", get(list_tasks).post(create_task))
        .route("/api/v1/tasks/{task_code}", get(get_task).patch(patch_task))
        .route("/api/v1/tasks/{task_code}/execute", post(execute_task))
        .route(
            "/api/v1/tasks/{task_code}/status",
            post(patch_status).patch(patch_status),
        )
        .route("/api/v1/tasks/{task_code}/archive", post(archive_task))
        .route("/api/v1/tasks/{task_code}/restore", post(restore_task))
}
