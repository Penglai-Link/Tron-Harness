use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{
    create_platform_model, delete_platform_model, disable_platform_model, duplicate_platform_model,
    enable_platform_model, get_platform_model, list_platform_models, patch_platform_model,
    set_platform_default, verify_platform_model,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/admin/platform/models",
            get(list_platform_models).post(create_platform_model),
        )
        .route(
            "/api/v1/admin/platform/models/{model_code}",
            get(get_platform_model)
                .patch(patch_platform_model)
                .delete(delete_platform_model),
        )
        .route(
            "/api/v1/admin/platform/models/{model_code}/verify",
            post(verify_platform_model),
        )
        .route(
            "/api/v1/admin/platform/models/{model_code}/duplicate",
            post(duplicate_platform_model),
        )
        .route(
            "/api/v1/admin/platform/models/{model_code}/enable",
            post(enable_platform_model),
        )
        .route(
            "/api/v1/admin/platform/models/{model_code}/disable",
            post(disable_platform_model),
        )
        .route(
            "/api/v1/admin/platform/models/{model_code}/set-default",
            post(set_platform_default),
        )
}
