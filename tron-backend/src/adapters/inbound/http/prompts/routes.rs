use axum::{Router, routing::post};

use crate::app::AppState;

use super::handler::{
    analyze_template, translate_draft, workspace_create, workspace_delete, workspace_detail,
    workspace_duplicate, workspace_export, workspace_page, workspace_revert,
    workspace_set_allow_inheritance, workspace_set_enabled, workspace_update,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/prompts/workspace-page", post(workspace_page))
        .route("/api/v1/prompts/workspace-detail", post(workspace_detail))
        .route("/api/v1/prompts/workspace-create", post(workspace_create))
        .route("/api/v1/prompts/workspace-update", post(workspace_update))
        .route(
            "/api/v1/prompts/workspace-duplicate",
            post(workspace_duplicate),
        )
        .route("/api/v1/prompts/workspace-delete", post(workspace_delete))
        .route("/api/v1/prompts/workspace-revert", post(workspace_revert))
        .route(
            "/api/v1/prompts/workspace-set-enabled",
            post(workspace_set_enabled),
        )
        .route(
            "/api/v1/prompts/workspace-set-allow-inheritance",
            post(workspace_set_allow_inheritance),
        )
        .route("/api/v1/prompts/analyze-template", post(analyze_template))
        .route("/api/v1/prompts/translate-draft", post(translate_draft))
        .route("/api/v1/prompts/workspace-export", post(workspace_export))
}
