use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{list_models, remove_model, restore_model};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/models/workspaces/{workspace_code}", get(list_models))
        .route(
            "/api/v1/models/workspaces/{workspace_code}/{model_code}/remove",
            post(remove_model),
        )
        .route(
            "/api/v1/models/workspaces/{workspace_code}/{model_code}/restore",
            post(restore_model),
        )
}
