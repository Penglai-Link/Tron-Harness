use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{
    archive_app, create_app, delete_app, get_app, list_app_options, list_apps, patch_app, restore_app,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/apps/workspaces/{workspace_code}",
            get(list_apps).post(create_app),
        )
        .route(
            "/api/v1/apps/workspaces/{workspace_code}/options",
            get(list_app_options),
        )
        .route("/api/v1/apps/{app_code}", get(get_app).patch(patch_app).delete(delete_app))
        .route("/api/v1/apps/{app_code}/archive", post(archive_app))
        .route("/api/v1/apps/{app_code}/restore", post(restore_app))
}
