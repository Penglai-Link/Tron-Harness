use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{
    create_workspace_model, delete_workspace_model, disable_workspace_model,
    duplicate_workspace_model, enable_workspace_model, get_workspace_model, list_select_options,
    list_workspace_models, patch_workspace_model, restore_workspace_model_from_platform_deprecated,
    set_allow_inheritance, set_workspace_default, verify_workspace_model,
    verify_workspace_model_create_draft,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/models/workspaces/{workspace_code}",
            get(list_workspace_models).post(create_workspace_model),
        )
        .route(
            "/api/v1/models/workspaces/{workspace_code}/verify",
            post(verify_workspace_model_create_draft),
        )
        .route(
            "/api/v1/models/workspaces/{workspace_code}/select-options",
            get(list_select_options),
        )
        .route(
            "/api/v1/models/workspaces/{workspace_code}/from-platform",
            post(restore_workspace_model_from_platform_deprecated),
        )
        .route(
            "/api/v1/models/workspaces/{workspace_code}/{model_code}",
            get(get_workspace_model)
                .patch(patch_workspace_model)
                .delete(delete_workspace_model),
        )
        .route(
            "/api/v1/models/workspaces/{workspace_code}/{model_code}/verify",
            post(verify_workspace_model),
        )
        .route(
            "/api/v1/models/workspaces/{workspace_code}/{model_code}/duplicate",
            post(duplicate_workspace_model),
        )
        .route(
            "/api/v1/models/workspaces/{workspace_code}/{model_code}/enable",
            post(enable_workspace_model),
        )
        .route(
            "/api/v1/models/workspaces/{workspace_code}/{model_code}/disable",
            post(disable_workspace_model),
        )
        .route(
            "/api/v1/models/workspaces/{workspace_code}/{model_code}/set-default",
            post(set_workspace_default),
        )
        .route(
            "/api/v1/models/workspaces/{workspace_code}/{model_code}/allow-inheritance",
            post(set_allow_inheritance),
        )
}
