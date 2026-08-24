use axum::{
    Router,
    routing::{get, patch, post},
};

use crate::app::AppState;

use super::handler::{
    create_platform_variable, delete_platform_variable, list_platform_variables,
    list_platform_prompt_variable_options, patch_platform_variable, reveal_platform_variable,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/admin/platform/variables",
            get(list_platform_variables).post(create_platform_variable),
        )
        .route(
            "/api/v1/admin/platform/variables/prompt-options",
            get(list_platform_prompt_variable_options),
        )
        .route(
            "/api/v1/admin/platform/variables/reveal-platform-variable",
            post(reveal_platform_variable),
        )
        .route(
            "/api/v1/admin/platform/variables/{variable_key}",
            patch(patch_platform_variable).delete(delete_platform_variable),
        )
}
