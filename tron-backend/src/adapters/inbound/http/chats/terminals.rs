use super::{super::auth::RequestAuthContext, error_mapper::chat_error_response};
use crate::{
    app::AppState,
    application::agent::{TerminalSnapshot, terminal_owner},
    response::{ApiResponse, EmptyData},
};
use axum::{
    Json,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadTerminalQuery {
    #[serde(default)]
    cursor: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalListItem {
    terminal_id: String,
    command: String,
    args: Vec<String>,
    cwd: String,
    running: bool,
    exit_code: Option<u32>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadTerminalResponse {
    terminal_id: String,
    cwd: String,
    running: bool,
    exit_code: Option<u32>,
    output: String,
    next_cursor: u64,
    truncated: bool,
    output_closed: bool,
}
fn owner(
    state: &AppState,
    auth: &RequestAuthContext,
    session_code: &str,
) -> Result<String, Response> {
    let session = state
        .chats
        .get_session(session_code)
        .map_err(chat_error_response)?;
    Ok(terminal_owner(
        &session.workspace_code,
        Some(&auth.user().user_code),
        session_code,
    ))
}
fn unavailable(locale: &str) -> Response {
    let message = if locale.starts_with("en") {
        "This terminal is unavailable in the current conversation. It may have expired or the backend restarted."
    } else {
        "当前会话无法访问此终端，可能已被回收或后端已重启。"
    };
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::protocol_error("101028001", message)),
    )
        .into_response()
}
pub async fn list(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(session): Path<String>,
) -> Result<Json<ApiResponse<Vec<TerminalListItem>>>, Response> {
    let owner = owner(&state, &auth, &session)?;
    let items = state
        .agent_terminal
        .list(&owner)
        .map_err(|_| unavailable(&auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(
        items
            .into_iter()
            .map(|value| TerminalListItem {
                terminal_id: value.terminal_id,
                command: value.command,
                args: value.args,
                cwd: value.cwd,
                running: value.running,
                exit_code: value.exit_code,
            })
            .collect(),
    )))
}
pub async fn read(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path((session, id)): Path<(String, String)>,
    query: Result<Query<ReadTerminalQuery>, QueryRejection>,
) -> Result<Json<ApiResponse<ReadTerminalResponse>>, Response> {
    let Query(query) = query.map_err(|_| {
        let message = if auth.locale.starts_with("en") {
            "The output cursor must be a non-negative integer."
        } else {
            "输出游标必须为非负整数。"
        };
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::protocol_error("101028002", message)),
        )
            .into_response()
    })?;
    let owner = owner(&state, &auth, &session)?;
    let TerminalSnapshot {
        terminal_id,
        cwd,
        running,
        exit_code,
        output,
        next_cursor,
        truncated,
        output_closed,
        ..
    } = state
        .agent_terminal
        .read(&owner, &id, query.cursor)
        .map_err(|_| unavailable(&auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(ReadTerminalResponse {
        terminal_id,
        cwd,
        running,
        exit_code,
        output,
        next_cursor,
        truncated,
        output_closed,
    })))
}
pub async fn stop(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path((session, id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<EmptyData>>, Response> {
    let owner = owner(&state, &auth, &session)?;
    let locale = auth.locale;
    crate::app::telemetry::spawn_blocking_traced(move || state.agent_terminal.stop(&owner, &id))
        .await
        .map_err(|_| unavailable(&locale))?
        .map_err(|_| unavailable(&locale))?;
    Ok(Json(ApiResponse::protocol_success(EmptyData {})))
}
