use axum::{middleware, routing::get, Router};

use crate::{
    adapters::inbound::http::{
        admin::{
            models::routes::routes as admin_model_routes, roles::routes as role_routes,
            users::routes as user_routes, variables::routes::routes as admin_variable_routes,
            workspaces::routes as workspace_routes,
        },
        agents::routes::routes as agent_routes,
        apps::routes::routes as app_routes,
        auth::{require_auth, routes as auth_routes},
        automations::routes as automation_routes,
        chats::routes::routes as chat_routes,
        frontend::routes as frontend_routes,
        mcp::routes::{
            management_routes as mcp_management_routes,
            transport_routes as mcp_transport_routes,
        },
        models::routes::routes as model_routes,
        rag::routes as rag_routes,
        skills::routes::routes as skill_routes,
        tasks::routes as task_routes,
        variables::routes::routes as variable_routes,
    },
    config::AppConfig,
    health::health,
};

use super::AppState;

pub fn build_router(config: AppConfig) -> Router {
    let state = AppState::new(config);
    let protected_routes = user_routes()
        .merge(admin_model_routes())
        .merge(admin_variable_routes())
        .merge(role_routes())
        .merge(model_routes())
        .merge(workspace_routes())
        .merge(automation_routes())
        .merge(chat_routes())
        .merge(app_routes())
        .merge(agent_routes())
        .merge(mcp_management_routes())
        .merge(task_routes())
        .merge(variable_routes())
        .merge(rag_routes())
        .merge(skill_routes())
        .route_layer(middleware::from_fn_with_state(
            state.users.clone(),
            require_auth,
        ));

    Router::new()
        .route("/api/v1/health", get(health))
        .merge(auth_routes())
        .merge(mcp_transport_routes())
        .merge(protected_routes)
        .merge(frontend_routes())
        .with_state(state)
}
