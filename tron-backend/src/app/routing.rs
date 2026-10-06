use axum::{Router, middleware};

use crate::adapters::inbound::http::{
    admin::{
        model_providers::{
            profile_routes as profile_model_provider_routes,
            public_routes as public_model_provider_routes,
        },
        models::routes::routes as admin_model_routes,
        navigation::routes as admin_navigation_routes,
        prompts::routes::routes as admin_prompt_routes,
        variables::routes::routes as admin_variable_routes,
        workspaces::routes as workspace_routes,
    },
    agents::routes::routes as agent_routes,
    apps::routes::routes as app_routes,
    auth::require_profile,
    automations::routes as automation_routes,
    capabilities::routes as capability_routes,
    chats::routes::routes as chat_routes,
    knowledge_graph::routes as knowledge_graph_routes,
    mcp::routes::management_routes as mcp_management_routes,
    memory::routes as memory_routes,
    models::routes::routes as model_routes,
    navigation::routes as navigation_routes,
    profiles::routes as profile_routes,
    projects::routes::routes as project_routes,
    prompts::routes::routes as prompt_routes,
    rag::routes as rag_routes,
    resources::routes::routes as resource_routes,
    skills::routes::routes as skill_routes,
    system_config::routes as system_config_routes,
    tasks::routes as task_routes,
    terminals::routes::routes as terminal_routes,
    variables::routes::routes as variable_routes,
};

use super::AppState;

pub(super) fn build_protected_routes(state: &AppState) -> Router<AppState> {
    admin_model_routes()
        .merge(profile_model_provider_routes())
        .merge(admin_navigation_routes())
        .merge(admin_prompt_routes())
        .merge(admin_variable_routes())
        .merge(model_routes())
        .merge(prompt_routes())
        .merge(project_routes())
        .merge(workspace_routes())
        .merge(automation_routes())
        .merge(capability_routes())
        .merge(chat_routes())
        .merge(app_routes())
        .merge(agent_routes())
        .merge(mcp_management_routes())
        .merge(task_routes())
        .merge(variable_routes())
        .merge(memory_routes())
        .merge(rag_routes())
        .merge(crate::adapters::inbound::http::knowledge_bases::routes())
        .merge(resource_routes(&state.resources))
        .merge(knowledge_graph_routes::routes())
        .merge(skill_routes())
        .merge(system_config_routes())
        .merge(crate::adapters::inbound::http::skins::routes())
        .merge(crate::adapters::inbound::http::person_ai::routes())
        .merge(navigation_routes())
        .merge(terminal_routes())
        .route_layer(middleware::from_fn_with_state(
            state.profiles.clone(),
            require_profile,
        ))
}

pub(super) fn build_profile_routes() -> Router<AppState> {
    profile_routes().merge(public_model_provider_routes())
}
