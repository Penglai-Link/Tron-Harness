use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{
    bind_workspace_skill, check_registry_skill_updates, get_registry_skill, get_workspace_skill,
    install_registry_skill, list_registry_skills, list_workspace_skills, refresh_workspace_skills,
    rescan_registry_skills, retry_workspace_skill_rag_index, search_skills, unbind_workspace_skill,
    uninstall_registry_skill, update_registry_skills,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/skills/search", post(search_skills))
        .route("/api/v1/skills/registry", get(list_registry_skills))
        .route(
            "/api/v1/skills/registry/install",
            post(install_registry_skill),
        )
        .route(
            "/api/v1/skills/registry/rescan",
            post(rescan_registry_skills),
        )
        .route(
            "/api/v1/skills/registry/check-updates",
            post(check_registry_skill_updates),
        )
        .route(
            "/api/v1/skills/registry/update",
            post(update_registry_skills),
        )
        .route(
            "/api/v1/skills/registry/{skill_name}",
            get(get_registry_skill).delete(uninstall_registry_skill),
        )
        .route(
            "/api/v1/skills/workspaces/{workspace_code}/skills",
            get(list_workspace_skills),
        )
        .route(
            "/api/v1/skills/workspaces/{workspace_code}/skills/{skill_name}",
            get(get_workspace_skill),
        )
        .route(
            "/api/v1/skills/workspaces/{workspace_code}/skills/{skill_name}/rag-index/retry",
            post(retry_workspace_skill_rag_index),
        )
        .route(
            "/api/v1/skills/workspaces/{workspace_code}/refresh",
            post(refresh_workspace_skills),
        )
        .route(
            "/api/v1/skills/workspaces/{workspace_code}/bindings",
            post(bind_workspace_skill),
        )
        .route(
            "/api/v1/skills/workspaces/{workspace_code}/bindings/{skill_name}",
            axum::routing::delete(unbind_workspace_skill),
        )
}
