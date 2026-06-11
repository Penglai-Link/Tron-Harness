use axum::{
    Router,
    routing::{get, post, put},
};

use crate::app::AppState;

use super::handler::{
    create_platform_open_app, get_platform_open_app, list_platform_open_app_catalog,
    list_platform_open_apps, patch_platform_open_app, put_platform_open_app_capability_grants,
    put_platform_open_app_user_grants, rotate_platform_open_app_key,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/admin/platform/open-apps",
            get(list_platform_open_apps).post(create_platform_open_app),
        )
        .route(
            "/api/v1/admin/platform/open-apps/catalog",
            get(list_platform_open_app_catalog),
        )
        .route(
            "/api/v1/admin/platform/open-apps/{open_app_code}",
            get(get_platform_open_app).patch(patch_platform_open_app),
        )
        .route(
            "/api/v1/admin/platform/open-apps/{open_app_code}/rotate-key",
            post(rotate_platform_open_app_key),
        )
        .route(
            "/api/v1/admin/platform/open-apps/{open_app_code}/user-grants",
            put(put_platform_open_app_user_grants),
        )
        .route(
            "/api/v1/admin/platform/open-apps/{open_app_code}/capability-grants",
            put(put_platform_open_app_capability_grants),
        )
}
