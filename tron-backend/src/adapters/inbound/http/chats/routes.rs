use axum::{
    Router,
    routing::{get, patch, post},
};

use crate::app::AppState;

use super::handler::{
    archive_session, create_session, delete_session, download_session_attachment, list_messages,
    list_session_groups, patch_session, send_message, steer_message, stream_message,
    upload_session_attachment,
};
use super::run_control::{get_active_run, stop_run};
use super::short_term_memory::{
    cancel_goal, clear_goal, create_compaction, get_goal, get_latest_compaction, get_latest_goal,
    get_todos, pause_goal, resume_goal, set_goal, update_goal,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/chats/session-groups", get(list_session_groups))
        .route("/api/v1/chats/sessions", post(create_session))
        .route("/api/v1/chats/sessions/get-active-run", get(get_active_run))
        .route("/api/v1/chats/sessions/steer-message", post(steer_message))
        .route("/api/v1/chats/sessions/stop-run", post(stop_run))
        .route("/api/v1/chats/sessions/get-todos", get(get_todos))
        .route(
            "/api/v1/chats/sessions/get-latest-compaction",
            get(get_latest_compaction),
        )
        .route(
            "/api/v1/chats/sessions/create-compaction",
            post(create_compaction),
        )
        .route("/api/v1/chats/sessions/get-goal", get(get_goal))
        .route(
            "/api/v1/chats/sessions/get-latest-goal",
            get(get_latest_goal),
        )
        .route("/api/v1/chats/sessions/set-goal", post(set_goal))
        .route("/api/v1/chats/sessions/pause-goal", post(pause_goal))
        .route("/api/v1/chats/sessions/resume-goal", post(resume_goal))
        .route("/api/v1/chats/sessions/cancel-goal", post(cancel_goal))
        .route("/api/v1/chats/sessions/clear-goal", post(clear_goal))
        .route("/api/v1/chats/sessions/update-goal", post(update_goal))
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
        .route(
            "/api/v1/chats/sessions/{session_code}/attachments",
            post(upload_session_attachment).layer(super::handler::chat_attachment_body_limit()),
        )
        .route(
            "/api/v1/chats/sessions/{session_code}/attachments/{attachment_code}",
            get(download_session_attachment),
        )
}
