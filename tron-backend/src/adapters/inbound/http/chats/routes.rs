use axum::{
    Router,
    routing::{get, patch, post},
};

use crate::app::AppState;

use super::handler::{
    archive_session, create_session, delete_session, list_messages, list_session_groups,
    patch_session, send_message, stream_message,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/chats/session-groups", get(list_session_groups))
        .route("/api/v1/chats/sessions", post(create_session))
        .route(
            "/api/v1/chats/sessions/{session_code}",
            patch(patch_session).delete(delete_session),
        )
        .route(
            "/api/v1/chats/sessions/{session_code}/archive",
            post(archive_session),
        )
        .route(
            "/api/v1/chats/sessions/{session_code}/messages",
            get(list_messages).post(send_message),
        )
        .route(
            "/api/v1/chats/sessions/{session_code}/messages/stream",
            post(stream_message),
        )
}
