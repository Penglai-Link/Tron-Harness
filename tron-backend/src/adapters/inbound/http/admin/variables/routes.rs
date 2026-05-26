use axum::{
    Router,
    routing::{get, patch},
};

use crate::app::AppState;

use super::handler::{
    create_platform_variable, delete_platform_variable, list_platform_variables,
    patch_platform_variable,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/admin/platform/variables",
            get(list_platform_variables).post(create_platform_variable),
        )
        .route(
            "/api/v1/admin/platform/variables/{variable_key}",
            patch(patch_platform_variable).delete(delete_platform_variable),
        )
}
