use axum::{
    Router,
    routing::{get, patch, post, put},
};

use crate::app::AppState;

use super::handler::{
    activate_agui_stream, create_agent_run, deactivate_agui_stream, disable_platform_system_agent,
    disable_workspace_system_agent, enable_platform_system_agent, enable_workspace_system_agent,
    get_agent_run, get_main_agent_config, get_runtime_capability, get_tool,
    get_workspace_internal_agent, list_agent_runs, list_agui_stream_events,
    list_platform_system_agents, list_runtime_capabilities, list_tool_domains, list_tools,
    list_workspace_internal_agents, list_workspace_system_agents,
    replace_workspace_internal_agent_tool_assignments, rescan_platform_system_agents,
    reset_workspace_internal_agent_policy, update_main_agent_config,
    update_workspace_internal_agent_policy,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/agents/runs",
            get(list_agent_runs).post(create_agent_run),
        )
        .route("/api/v1/agents/runs/{run_code}", get(get_agent_run))
        .route(
            "/api/v1/agents/registry/main-agent",
            get(get_main_agent_config).patch(update_main_agent_config),
        )
        .route(
            "/api/v1/agents/registry/system",
            get(list_platform_system_agents),
        )
        .route(
            "/api/v1/agents/registry/system/rescan",
            post(rescan_platform_system_agents),
        )
        .route(
            "/api/v1/agents/registry/system/{agent_code}/enable",
            post(enable_platform_system_agent),
        )
        .route(
            "/api/v1/agents/registry/system/{agent_code}/disable",
            post(disable_platform_system_agent),
        )
        .route("/api/v1/agents/runtimes", get(list_runtime_capabilities))
        .route(
            "/api/v1/agents/runtimes/{adapter_key}/capabilities",
            get(get_runtime_capability),
        )
        .route(
            "/api/v1/agents/workspaces/{workspace_code}/system",
            get(list_workspace_system_agents),
        )
        .route(
            "/api/v1/agents/workspaces/{workspace_code}/system/{agent_code}/enable",
            post(enable_workspace_system_agent),
        )
        .route(
            "/api/v1/agents/workspaces/{workspace_code}/system/{agent_code}/disable",
            post(disable_workspace_system_agent),
        )
        .route("/api/v1/tools/domains", get(list_tool_domains))
        .route("/api/v1/tools", get(list_tools))
        .route("/api/v1/tools/{tool_code}", get(get_tool))
        .route(
            "/api/v1/agents/workspaces/{workspace_code}/internal",
            get(list_workspace_internal_agents),
        )
        .route(
            "/api/v1/agents/workspaces/{workspace_code}/internal/{agent_code}",
            get(get_workspace_internal_agent),
        )
        .route(
            "/api/v1/agents/workspaces/{workspace_code}/internal/{agent_code}/policy",
            patch(update_workspace_internal_agent_policy),
        )
        .route(
            "/api/v1/agents/workspaces/{workspace_code}/internal/{agent_code}/tool-assignments",
            put(replace_workspace_internal_agent_tool_assignments),
        )
        .route(
            "/api/v1/agents/workspaces/{workspace_code}/internal/{agent_code}/reset-policy",
            post(reset_workspace_internal_agent_policy),
        )
        .route(
            "/api/v1/agents/agui/streams/{stream_key}",
            post(activate_agui_stream).delete(deactivate_agui_stream),
        )
        .route(
            "/api/v1/agents/agui/streams/{stream_key}/events",
            get(list_agui_stream_events),
        )
}
