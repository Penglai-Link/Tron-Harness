use axum::{
    Router,
    routing::{get, patch, post},
};

use crate::app::AppState;

use super::handler::{
    create_variable, delete_variable, list_variables, patch_variable, reveal_workspace_variable,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/variables/workspaces/{workspace_code}",
            get(list_variables).post(create_variable),
        )
        .route(
            "/api/v1/variables/workspaces/{workspace_code}/{variable_key}",
            patch(patch_variable).delete(delete_variable),
        )
        .route(
            "/api/v1/variables/reveal-workspace-variable",
            post(reveal_workspace_variable),
        )
}
