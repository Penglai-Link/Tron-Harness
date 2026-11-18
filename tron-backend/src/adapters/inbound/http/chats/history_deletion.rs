use crate::{
    adapters::inbound::http::auth::RequestAuthContext, app::AppState, response::ApiResponse,
    shared::locale::SupportedLocale,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use eliteforge_agent_core as fw;
use serde::{Deserialize, Serialize};
use tron_i18n::t;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteHistoryTurnRequest {
    pub expected_version: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteHistoryTurnResponse {
    pub session_id: String,
    pub root_run_id: String,
    pub context_revision: String,
}

pub async fn delete_turn(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path((session, root)): Path<(String, String)>,
    body: Result<Json<DeleteHistoryTurnRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let locale = SupportedLocale::from_raw(&auth.locale);
    if let Err(error) = state.chats.get_session(&session) {
        return super::error_mapper::chat_error_response(error);
    }
    let result = async {
        let Json(body) = body.map_err(|_| fw::StateError::Invalid("request".into()))?;
        if body.expected_version.is_empty() {
            return Err(fw::StateError::Invalid("version".into()));
        }
        let root_id =
            fw::RunId::parse_str(&root).map_err(|_| fw::StateError::Invalid("run".into()))?;
        state
            .chats
            .delete_history_turn(
                &state.agent_framework,
                &session,
                root_id,
                &body.expected_version,
            )
            .await
    }
    .await;
    match result {
        Ok(context_revision) => Json(ApiResponse::success(DeleteHistoryTurnResponse {
            session_id: session,
            root_run_id: root,
            context_revision,
        }))
        .into_response(),
        Err(error) => {
            let descriptor = crate::application::chat::history_error::deletion_descriptor(&error);
            let (status, message) = match descriptor.message_key {
                "tron.chat.history_delete_unsupported" => (
                    StatusCode::BAD_REQUEST,
                    t!("tron.chat.history_delete_unsupported", locale = locale),
                ),
                "tron.chat.history_delete_busy" => (
                    StatusCode::CONFLICT,
                    t!("tron.chat.history_delete_busy", locale = locale),
                ),
                "tron.chat.history_version_changed" => (
                    StatusCode::CONFLICT,
                    t!("tron.chat.history_version_changed", locale = locale),
                ),
                "tron.chat.history_not_found" => (
                    StatusCode::NOT_FOUND,
                    t!("tron.chat.history_turn_not_found", locale = locale),
                ),
                "tron.chat.history_delete_invalid" => (
                    StatusCode::BAD_REQUEST,
                    t!("tron.chat.history_delete_invalid", locale = locale),
                ),
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    t!("tron.chat.history_delete_failed", locale = locale),
                ),
            };
            (status, Json(ApiResponse::error(descriptor.code, message))).into_response()
        }
    }
}
