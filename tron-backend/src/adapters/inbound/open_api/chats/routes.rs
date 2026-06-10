use axum::{
    Router,
    routing::{get, patch, post},
};

use crate::app::AppState;

use super::handler::{
    create_session, list_agui_stream_events, list_agui_stream_protocol_events, list_messages,
    list_session_groups, patch_session, send_message, stream_message,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/open-api/v1/chats/session-groups", get(list_session_groups))
        .route("/open-api/v1/chats/sessions", post(create_session))
        .route(
            "/open-api/v1/chats/sessions/{session_code}",
            patch(patch_session),
        )
        .route(
            "/open-api/v1/chats/sessions/{session_code}/messages",
            get(list_messages).post(send_message),
        )
        .route(
            "/open-api/v1/chats/sessions/{session_code}/messages/stream",
            post(stream_message),
        )
        .route(
            "/open-api/v1/agents/agui/streams/{stream_key}/events",
            get(list_agui_stream_events),
        )
        .route(
            "/open-api/v1/agents/agui/streams/{stream_key}/protocol-events",
            get(list_agui_stream_protocol_events),
        )
}
