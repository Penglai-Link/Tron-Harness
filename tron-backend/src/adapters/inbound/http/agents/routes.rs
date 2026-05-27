use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{
    activate_agui_stream, create_agent_run, deactivate_agui_stream, get_agent_run, list_agent_runs,
    list_agui_stream_events,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/agents/runs",
            get(list_agent_runs).post(create_agent_run),
        )
        .route("/api/v1/agents/runs/{run_code}", get(get_agent_run))
        .route(
            "/api/v1/agents/agui/streams/{stream_key}",
            post(activate_agui_stream).delete(deactivate_agui_stream),
        )
        .route(
            "/api/v1/agents/agui/streams/{stream_key}/events",
            get(list_agui_stream_events),
        )
}
