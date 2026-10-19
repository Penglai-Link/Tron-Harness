use super::{handler::*, task_history::delete_task};
use crate::app::AppState;
use axum::{
    Router,
    routing::{delete, get, post, put},
};
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(super::graph::routes())
        .route("/api/v1/rag/corpora", get(corpora))
        .route("/api/v1/rag/corpora/{id}/settings", put(configure))
        .route("/api/v1/rag/tasks", get(tasks).post(submit))
        .route("/api/v1/rag/tasks/{id}", delete(delete_task))
        .route(
            "/api/v1/rag/tasks/{id}/chunks",
            get(super::task_chunks::chunks),
        )
        .route("/api/v1/rag/tasks/{id}/cancel", post(cancel))
        .route("/api/v1/rag/tasks/{id}/retry", post(retry))
        .route("/api/v1/rag/tasks/{id}/rebuild", post(rebuild))
        .route("/api/v1/rag/search", post(search))
        .route("/api/v1/rag/recall", post(recall))
}
