use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::*;

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
            "/api/v1/skills/registry/{skill_name}/agent-grants",
            get(get_skill_agents).put(put_skill_agents),
        )
        .route(
            "/api/v1/agents/registry/internal/{agent_code}/skills",
            get(get_agent_skills).put(put_agent_skills),
        )
}
