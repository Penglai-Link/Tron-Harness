//! Read-only child-session access, authorized through the owning product chat.
use crate::{
    adapters::inbound::http::auth::RequestAuthContext, app::AppState,
    application::agent::SubagentView, domain::chat::ChatError, response::ApiResponse,
    shared::locale::SupportedLocale,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use eliteforge_agent_core as fw;
use serde::Serialize;
use tron_i18n::t;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentListItem {
    run_id: fw::RunId,
    session_id: String,
    parent_run_id: fw::RunId,
    agent_code: String,
    agent_name: String,
    avatar_id: Option<String>,
    task: String,
    status: fw::HistoryStatus,
    started_at_ms: u64,
    finished_at_ms: Option<u64>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentDetail {
    run_id: fw::RunId,
    session_id: String,
    status: fw::HistoryStatus,
    records: fw::HistoryRecords,
    events: Vec<serde_json::Value>,
}
fn authorize(state: &AppState, auth: &RequestAuthContext, session: &str) -> Result<(), Response> {
    let parent = state
        .chats
        .get_session(session)
        .map_err(super::error_mapper::chat_error_response)?;
    if parent
        .user_code
        .as_deref()
        .is_some_and(|user| user != auth.user().user_code)
    {
        return Err(super::error_mapper::chat_error_response(
            ChatError::SessionNotFound,
        ));
    }
    Ok(())
}
fn error(error: fw::StateError, locale: SupportedLocale) -> Response {
    let descriptor = crate::application::chat::subagent_error::descriptor(&error);
    let missing = matches!(error, fw::StateError::NotFound);
    let message = match descriptor.message_key {
        "tron.chat.subagent_not_found" => {
            t!("tron.chat.subagent_not_found", locale = locale)
        }
        _ => t!("tron.chat.subagent_read_failed", locale = locale),
    };
    (
        if missing {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        },
        Json(ApiResponse::error(descriptor.code, message)),
    )
        .into_response()
}
pub async fn list(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(session): Path<String>,
) -> Result<Json<ApiResponse<Vec<SubagentListItem>>>, Response> {
    authorize(&state, &auth, &session)?;
    let locale = SupportedLocale::from_raw(&auth.locale);
    let items = state
        .agent_framework
        .subagents(&session, &auth.user().user_code)
        .await
        .map_err(|e| error(e, locale))?;
    Ok(Json(ApiResponse::success(
        items
            .into_iter()
            .map(|SubagentView { record, run }| SubagentListItem {
                run_id: record.run_id,
                session_id: record.session_id,
                parent_run_id: record.parent_run_id,
                agent_code: record.agent_code,
                agent_name: record.agent_name,
                avatar_id: record.avatar_id,
                task: record.task,
                status: run.status,
                started_at_ms: run.started_at_ms,
                finished_at_ms: run.finished_at_ms,
            })
            .collect(),
    )))
}
pub async fn read(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path((session, id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<SubagentDetail>>, Response> {
    authorize(&state, &auth, &session)?;
    let locale = SupportedLocale::from_raw(&auth.locale);
    let id = fw::RunId::parse_str(&id).map_err(|_| error(fw::StateError::NotFound, locale))?;
    let (item, history) = state
        .agent_framework
        .subagent_history(&session, id, &auth.user().user_code)
        .await
        .map_err(|e| error(e, locale))?;
    let mut records = history.records;
    for message in &mut records.messages {
        message.content = message
            .content
            .drain(..)
            .filter_map(super::history::public_history_block)
            .collect();
    }
    let events = state
        .agent_presentation
        .protocol_events(&item.record.session_id)
        .map_err(|_| {
            error(
                fw::StateError::Backend("presentation unavailable".into()),
                locale,
            )
        })?
        .into_iter()
        .map(|event| event.data)
        .collect();
    Ok(Json(ApiResponse::success(SubagentDetail {
        run_id: id,
        session_id: item.record.session_id,
        status: item.run.status,
        records,
        events,
    })))
}
