use axum::{Router, routing::post};

use crate::app::AppState;

use super::handler::{
    platform_create, platform_delete, platform_detail, platform_duplicate, platform_page,
    platform_set_enabled, platform_update,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/platform/prompts/page", post(platform_page))
        .route(
            "/api/v1/admin/platform/prompts/detail",
            post(platform_detail),
        )
        .route(
            "/api/v1/admin/platform/prompts/create",
            post(platform_create),
        )
        .route(
            "/api/v1/admin/platform/prompts/update",
            post(platform_update),
        )
        .route(
            "/api/v1/admin/platform/prompts/duplicate",
            post(platform_duplicate),
        )
        .route(
            "/api/v1/admin/platform/prompts/delete",
            post(platform_delete),
        )
        .route(
            "/api/v1/admin/platform/prompts/set-enabled",
            post(platform_set_enabled),
        )
}
