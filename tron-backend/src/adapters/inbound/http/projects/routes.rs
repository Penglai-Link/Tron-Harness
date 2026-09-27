use super::handler::{archive_project, get_project, list_projects, save_project};
use crate::app::AppState;
use axum::{Router, routing::post};
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/projects/list", post(list_projects))
        .route("/api/v1/projects/get", post(get_project))
        .route("/api/v1/projects/save", post(save_project))
        .route("/api/v1/projects/archive", post(archive_project))
}
