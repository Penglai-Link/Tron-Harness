use serde::Serialize;

use crate::{
    application::chat::{
        ActiveChatRunSnapshot, ChatSteerResult, GetActiveChatRunResult, StopChatRunResult,
    },
    domain::chat::{ChatMessageAttachment, ChatMessageChannel, ChatMessageRole},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetActiveChatRunResponse {
    pub session_code: String,
    pub active_run: Option<ActiveChatRunResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveChatRunResponse {
    pub run_code: String,
    pub runtime_adapter_key: String,
    pub supports_steer: bool,
    pub state: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopChatRunResponse {
    pub session_code: String,
    pub run_code: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSteerResultResponse {
    pub session_code: String,
    pub run_code: String,
    pub message: SteeredChatMessageResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteeredChatMessageResponse {
    pub id: String,
    pub message_code: String,
    pub session_code: String,
    pub run_code: String,
    pub role: ChatMessageRole,
    pub channel: ChatMessageChannel,
    pub actor: String,
    pub content: String,
    pub attachments: Vec<ChatMessageAttachment>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
    pub created_at: String,
}

impl From<ChatSteerResult> for ChatSteerResultResponse {
    fn from(value: ChatSteerResult) -> Self {
        let message = value.message;
        Self {
            session_code: value.session_code,
            run_code: value.run_code,
            message: SteeredChatMessageResponse {
                id: message.id.to_string(),
                message_code: message.message_code,
                session_code: message.session_code,
                run_code: message.run_code.unwrap_or_default(),
                role: message.role,
                channel: message.channel,
                actor: message.actor,
                content: message.content,
                attachments: message.attachments,
                input_tokens: message.input_tokens,
                output_tokens: message.output_tokens,
                total_tokens: message.total_tokens,
                created_at: message.created_at,
            },
        }
    }
}

impl From<GetActiveChatRunResult> for GetActiveChatRunResponse {
    fn from(value: GetActiveChatRunResult) -> Self {
        Self {
            session_code: value.session_code,
            active_run: value.active_run.map(ActiveChatRunResponse::from),
        }
    }
}

impl From<ActiveChatRunSnapshot> for ActiveChatRunResponse {
    fn from(value: ActiveChatRunSnapshot) -> Self {
        Self {
            run_code: value.run_code,
            runtime_adapter_key: value.runtime_adapter_key,
            supports_steer: value.supports_steer,
            state: value.state.code().to_string(),
        }
    }
}

impl From<StopChatRunResult> for StopChatRunResponse {
    fn from(value: StopChatRunResult) -> Self {
        Self {
            session_code: value.session_code,
            run_code: value.run_code,
        }
    }
}
