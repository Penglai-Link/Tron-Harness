use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{
    archive_workspace, create_workspace, get_workspace, list_workspace_options, list_workspaces,
    move_workspace, patch_workspace, restore_workspace, workspace_switcher,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/admin/workspaces",
            get(list_workspaces).post(create_workspace),
        )
        .route(
            "/api/v1/admin/workspaces/options",
            get(list_workspace_options),
        )
        .route("/api/v1/admin/workspaces/switcher", get(workspace_switcher))
        .route(
            "/api/v1/admin/workspaces/{workspace_code}",
            get(get_workspace).patch(patch_workspace),
        )
        .route(
            "/api/v1/admin/workspaces/{workspace_code}/move",
            post(move_workspace),
        )
        .route(
            "/api/v1/admin/workspaces/{workspace_code}/archive",
            post(archive_workspace),
        )
        .route(
            "/api/v1/admin/workspaces/{workspace_code}/restore",
            post(restore_workspace),
        )
}
