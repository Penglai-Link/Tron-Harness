use axum::{
    Router,
    routing::{get, patch, post},
};

use crate::app::AppState;

use super::handler::{
    create_session, delete_session, list_agui_stream_events, list_agui_stream_protocol_events, list_messages,
    list_session_groups, patch_session, send_message, steer_message, stream_message,
};
use crate::adapters::inbound::http::chats::handler::{
    chat_attachment_body_limit, download_session_attachment, upload_session_attachment,
};
use super::run_control::{get_active_run, stop_run};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/open-api/v1/chats/session-groups",
            get(list_session_groups),
        )
        .route("/open-api/v1/chats/sessions", post(create_session))
        .route(
            "/open-api/v1/chats/sessions/get-active-run",
            get(get_active_run),
        )
        .route(
            "/open-api/v1/chats/sessions/steer-message",
            post(steer_message),
        )
        .route("/open-api/v1/chats/sessions/stop-run", post(stop_run))
        .route(
            "/open-api/v1/chats/sessions/{session_code}",
            patch(patch_session).delete(delete_session),
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
            "/open-api/v1/chats/sessions/{session_code}/attachments",
            post(upload_session_attachment).layer(chat_attachment_body_limit()),
        )
        .route(
            "/open-api/v1/chats/sessions/{session_code}/attachments/{attachment_code}",
            get(download_session_attachment),
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
