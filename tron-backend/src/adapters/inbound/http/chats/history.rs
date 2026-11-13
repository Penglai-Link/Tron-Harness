//! Transport and workspace authorization only; Framework owns history storage.
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
pub struct ReadHistoryRequest {
    pub size: usize,
    pub cursor: Option<fw::HistoryCursor>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadHistoryResponse {
    pub compaction_details: std::collections::BTreeMap<String, Vec<CompactionBlockResponse>>,
    pub can_delete_turns: bool,
    pub session_id: String,
    pub version: String,
    pub next_cursor: Option<fw::HistoryCursor>,
    pub runs: Vec<fw::HistoryRun>,
    pub records: fw::HistoryRecords,
    pub pending_interactions: Vec<fw::HistoryInteraction>,
    pub active_runs: Vec<fw::HistoryRun>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactionBlockResponse {
    id: String,
    before_tokens: usize,
    after_tokens: usize,
    kind: &'static str,
}
impl From<fw::SessionHistory> for ReadHistoryResponse {
    fn from(mut h: fw::SessionHistory) -> Self {
        for message in &mut h.records.messages {
            message.content = message
                .content
                .drain(..)
                .filter_map(public_history_block)
                .collect();
        }
        Self {
            compaction_details: Default::default(),
            can_delete_turns: h.can_delete_turns,
            session_id: h.session_id,
            version: h.version,
            next_cursor: h.next_cursor,
            runs: h.runs,
            records: h.records,
            pending_interactions: h.pending_interactions,
            active_runs: h.active_runs,
        }
    }
}

// The lossless conversation codec is for model resumption. Only its public
// projection belongs in the transcript; genuine user JSON stays untouched.
pub(crate) fn public_history_block(
    block: fw::HistoryContentBlock,
) -> Option<fw::HistoryContentBlock> {
    let fw::HistoryContentBlock::Json { value } = &block else {
        return Some(block);
    };
    match value
        .pointer("/tronMessagePart/type")
        .and_then(serde_json::Value::as_str)
    {
        Some("reasoningSummary") => value
            .pointer("/tronMessagePart/text")
            .and_then(serde_json::Value::as_str)
            .map(|text| fw::HistoryContentBlock::Reasoning {
                text: text.to_owned(),
            }),
        Some("context" | "reasoning" | "compactionCheckpoint" | "toolCall" | "toolResult") => None,
        _ => Some(block),
    }
}
pub async fn read_history(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(session): Path<String>,
    headers: HeaderMap,
    request: Result<Json<ReadHistoryRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let locale =
        SupportedLocale::from_request(headers.get("accept-language").and_then(|h| h.to_str().ok()));
    let Json(query) = match request {
        Ok(q) => q,
        Err(_) => return history_error(fw::StateError::Invalid("request".into()), locale),
    };
    if let Err(error) = state.chats.get_session(&session) {
        return super::error_mapper::chat_error_response(error);
    }
    let result = async {
        let framework = state
            .agent_framework
            .storage_framework()
            .await
            .map_err(|_| fw::StateError::Backend("framework unavailable".into()))?;
        let history = framework
            .history()
            .map_err(|_| fw::StateError::Backend("history unavailable".into()))?
            .read_session(
                &session,
                fw::HistoryQuery {
                    limit: query.size,
                    cursor: query.cursor,
                },
            )
            .await?;
        let details = crate::adapters::outbound::persistence::sqlite::compaction_details::read_compaction_details(
            &state.agent_framework, &session, &history.records.steps,
        ).await?;
        let mut response = ReadHistoryResponse::from(history);
        response.compaction_details = details.into_iter().map(|(id, blocks)| (id,
            blocks.into_iter().map(|block| CompactionBlockResponse {
                id: block.id, before_tokens: block.before_tokens,
                after_tokens: block.after_tokens, kind: block.kind,
            }).collect()
        )).collect();
        Ok::<_, fw::StateError>(response)
    }
    .await;
    match result {
        Ok(history) => Json(ApiResponse::success(history)).into_response(),
        Err(error) => history_error(error, locale),
    }
}
fn history_error(error: fw::StateError, locale: SupportedLocale) -> Response {
    let descriptor = crate::application::chat::history_error::descriptor(&error);
    let (status, zh, en) = match descriptor.message_key {
        "tron.chat.history_query_invalid" => (
            StatusCode::BAD_REQUEST,
            "历史查询参数无效，每页支持 1 至 100 轮",
            "Invalid history query; page size must be 1 to 100",
        ),
        "tron.chat.history_version_changed" => (
            StatusCode::CONFLICT,
            "会话历史已更新，请重新加载",
            "History changed; reload the newest page",
        ),
        "tron.chat.history_not_found" => (StatusCode::NOT_FOUND, "会话不存在", "Session not found"),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "读取会话历史失败",
            "Could not read session history",
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
