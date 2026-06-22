use axum::{
    Router,
    http::{Method, header},
    middleware,
};
use tower_http::cors::{Any, CorsLayer};

use crate::adapters::{
    inbound::http::{
        admin::{
            models::routes::routes as admin_model_routes,
            open_apps::routes::routes as admin_open_app_routes, roles::routes as role_routes,
            users::routes as user_routes, variables::routes::routes as admin_variable_routes,
            workspaces::routes as workspace_routes,
        },
        agents::routes::routes as agent_routes,
        apps::routes::routes as app_routes,
        auth::require_auth,
        automations::routes as automation_routes,
        chats::routes::routes as chat_routes,
        dashboard::routes as dashboard_routes,
        mcp::routes::management_routes as mcp_management_routes,
        memory::routes as memory_routes,
        models::routes::routes as model_routes,
        rag::routes as rag_routes,
        skills::routes::routes as skill_routes,
        tasks::routes as task_routes,
        variables::routes::routes as variable_routes,
    },
    inbound::open_api::{
        auth::require_open_api_auth, chats::routes::routes as open_api_chat_routes,
    },
};

use super::AppState;

pub(super) fn build_protected_routes(state: &AppState) -> Router<AppState> {
    user_routes()
        .merge(admin_model_routes())
        .merge(admin_open_app_routes())
        .merge(admin_variable_routes())
        .merge(role_routes())
        .merge(model_routes())
        .merge(workspace_routes())
        .merge(automation_routes())
        .merge(chat_routes())
        .merge(dashboard_routes())
        .merge(app_routes())
        .merge(agent_routes())
        .merge(mcp_management_routes())
        .merge(task_routes())
        .merge(variable_routes())
        .merge(memory_routes())
        .merge(rag_routes())
        .merge(skill_routes())
        .route_layer(middleware::from_fn_with_state(
            state.users.clone(),
            require_auth,
        ))
}

pub(super) fn build_open_api_routes(state: &AppState) -> Router<AppState> {
    open_api_chat_routes()
        .route_layer(middleware::from_fn_with_state(
            state.open_apps.clone(),
            require_open_api_auth,
        ))
        .layer(open_api_cors_layer())
}

fn open_api_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::OPTIONS])
        .allow_headers([
            header::ACCEPT,
            header::ACCEPT_LANGUAGE,
            header::CONTENT_TYPE,
            header::HeaderName::from_static("x-tron-app-code"),
            header::HeaderName::from_static("x-tron-user-code"),
            header::HeaderName::from_static("x-tron-timestamp"),
            header::HeaderName::from_static("x-tron-nonce"),
            header::HeaderName::from_static("x-tron-signature-version"),
            header::HeaderName::from_static("x-tron-signature"),
        ])
}
