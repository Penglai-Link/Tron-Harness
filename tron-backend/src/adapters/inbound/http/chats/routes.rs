use axum::{
    Router,
    routing::{get, patch, post},
};

use crate::app::AppState;

use super::handler::{
    answer_user_input, archive_session, archived_session_detail, batch_delete_archived_sessions,
    create_session, delete_archived_session, delete_session, download_archived_session_attachment,
    download_session_attachment, list_archived_sessions, list_messages, list_session_groups,
    list_user_input_requests, patch_session, restore_session, send_message, steer_message,
    stream_message, upload_session_attachment,
};
use super::run_control::{get_active_run, stop_run};
use super::short_term_memory::{
    cancel_goal, clear_goal, create_compaction, get_context_detail, get_goal,
    get_latest_compaction, get_latest_goal, get_todos, pause_goal, resume_goal, set_goal,
    update_goal,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/chats/input-suggestions", get(super::composer::suggestions))
        .route("/api/v1/chats/sessions/{session_code}/subagents", get(super::subagents::list))
        .route("/api/v1/chats/sessions/{session_code}/subagents/{run_id}", get(super::subagents::read))
        .route("/api/v1/chats/sessions/{session_code}/a2ui-state",get(super::a2ui::state))
        .route("/api/v1/chats/sessions/{session_code}/a2ui/actions/stream",post(super::a2ui::action))
        .route("/api/v1/chats/sessions/{session_code}/history/turns/{root_run_id}", axum::routing::delete(super::history_deletion::delete_turn))
        .route("/api/v1/chats/sessions/{session_code}/history/query", post(super::history::read_history))
        .route("/api/v1/chats/sessions/{session_code}/terminals", get(super::terminals::list))
        .route("/api/v1/chats/sessions/{session_code}/terminals/{terminal_id}", get(super::terminals::read))
        .route("/api/v1/chats/sessions/{session_code}/terminals/{terminal_id}/stop", post(super::terminals::stop))
        .route("/api/v1/chats/sessions/applications", get(super::application_context::get_session_applications))
        .route("/api/v1/chats/session-groups", get(list_session_groups))
        .route("/api/v1/admin/archived-sessions", get(list_archived_sessions))
        .route("/api/v1/admin/archived-sessions/batch-delete", post(batch_delete_archived_sessions))
        .route("/api/v1/admin/archived-sessions/{session_code}/restore", post(restore_session))
        .route("/api/v1/admin/archived-sessions/{session_code}", get(archived_session_detail).delete(delete_archived_session))
        .route("/api/v1/admin/archived-sessions/{session_code}/attachments/{attachment_code}", get(download_archived_session_attachment))
        .route("/api/v1/chats/sessions", post(create_session))
        .route("/api/v1/chats/sessions/get-active-run", get(get_active_run))
        .route("/api/v1/chats/sessions/steer-message", post(steer_message))
        .route("/api/v1/chats/sessions/stop-run", post(stop_run))
        .route("/api/v1/chats/sessions/get-todos", get(get_todos))
        .route("/api/v1/chats/sessions/get-context-detail", get(get_context_detail))
        .route("/api/v1/chats/sessions/get-context-assembly", get(super::context_assembly::read))
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
            "/api/v1/chats/sessions/{session_code}/user-input-requests",
            get(list_user_input_requests),
        )
        .route(
            "/api/v1/chats/sessions/{session_code}/user-input-requests/{request_code}/answers/stream",
            post(answer_user_input),
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
