use crate::{
    adapters::inbound::http::auth::RequestAuthContext, app::AppState, response::ApiResponse,
    shared::locale::SupportedLocale,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use eliteforge_agent_core as fw;
use serde::{Deserialize, Serialize};

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
    _auth: RequestAuthContext,
    Path((session, root)): Path<(String, String)>,
    headers: HeaderMap,
    body: Result<Json<DeleteHistoryTurnRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let locale =
        SupportedLocale::from_request(headers.get("accept-language").and_then(|v| v.to_str().ok()));
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
            let (status, zh, en) = match descriptor.message_key {
                "tron.chat.history_delete_unsupported" => (
                    StatusCode::BAD_REQUEST,
                    "外部 Agent 会话不支持删除历史轮次",
                    "External agent sessions do not support deleting turns",
                ),
                "tron.chat.history_delete_busy" => (
                    StatusCode::CONFLICT,
                    "请先结束当前运行及待处理的用户交互，再删除历史轮次",
                    "Finish the active run and pending interactions before deleting a turn",
                ),
                "tron.chat.history_version_changed" => (
                    StatusCode::CONFLICT,
                    "会话历史已更新，请刷新后重试",
                    "History changed; refresh and retry",
                ),
                "tron.chat.history_not_found" => (
                    StatusCode::NOT_FOUND,
                    "会话或历史轮次不存在",
                    "Session or turn not found",
                ),
                "tron.chat.history_delete_invalid" => (
                    StatusCode::BAD_REQUEST,
                    "无法删除该历史轮次，请检查请求",
                    "This history turn cannot be deleted; check the request",
                ),
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "删除历史轮次失败",
                    "Could not delete the history turn",
                ),
            };
            (
                status,
                Json(ApiResponse::error(
                    descriptor.code,
                    if locale == SupportedLocale::EnUs {
                        en
                    } else {
                        zh
                    },
                )),
            )
                .into_response()
        }
    }
}
