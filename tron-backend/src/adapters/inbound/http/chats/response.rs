use serde::Serialize;

use crate::{
    application::chat::{
        ActiveChatRunSnapshot, ChatSteerResult, GetActiveChatRunResult, StopChatRunResult,
    },
    domain::chat::ChatUserInputRequest,
    domain::chat::{ChatMessageAttachment, ChatMessageChannel, ChatMessageRole},
    shared::time::{option_timestamp_to_millis, timestamp_to_millis},
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
    pub operation_kind: Option<String>,
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
pub struct ChatUserInputRequestResponse {
    pub request_code: String,
    pub session_code: String,
    pub run_code: String,
    pub assistant_message_code: Option<String>,
    pub intro: String,
    pub questions: Vec<crate::domain::chat::ChatUserInputQuestion>,
    pub answers: Option<Vec<crate::domain::chat::ChatUserInputAnswer>>,
    pub status: crate::domain::chat::ChatUserInputRequestStatus,
    pub requested_at: i64,
    pub answered_at: Option<i64>,
}

impl From<ChatUserInputRequest> for ChatUserInputRequestResponse {
    fn from(value: ChatUserInputRequest) -> Self {
        Self {
            request_code: value.request_code,
            session_code: value.session_code,
            run_code: value.run_code,
            assistant_message_code: value.assistant_message_code,
            intro: value.intro,
            questions: value.questions,
            answers: value.answers,
            status: value.status,
            requested_at: timestamp_to_millis(&value.requested_at),
            answered_at: option_timestamp_to_millis(value.answered_at.as_deref()),
        }
    }
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
    pub annotations: Vec<eliteforge_agent_core::MessageAnnotation>,
    pub id: String,
    pub message_code: String,
    pub session_code: String,
    pub run_code: String,
    pub role: ChatMessageRole,
    pub channel: ChatMessageChannel,
    pub actor: String,
    pub content: String,
    pub attachments: Vec<ChatMessageAttachment>,
    pub requested_skills: Vec<String>,
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
                annotations: message.annotations,
                id: message.id.to_string(),
                message_code: message.message_code,
                session_code: message.session_code,
                run_code: message.run_code.unwrap_or_default(),
                role: message.role,
                channel: message.channel,
                actor: message.actor,
                content: message.content,
                attachments: message.attachments,
                requested_skills: message.requested_skills,
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
            operation_kind: value.operation_kind,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedSessionResponse {
    pub session_code: String,
    pub workspace_code: String,
    pub project_code: Option<String>,
    pub title: String,
    pub status: eliteforge_agent_core::SessionStatus,
    pub created_at: i64,
    pub updated_at: i64,
}
impl From<crate::domain::chat::ChatSession> for ArchivedSessionResponse {
    fn from(s: crate::domain::chat::ChatSession) -> Self {
        Self {
            session_code: s.session_code,
            workspace_code: s.workspace_code,
            project_code: s.project_code,
            title: s.title,
            status: match s.status {
                crate::domain::chat::ChatSessionStatus::Active => {
                    eliteforge_agent_core::SessionStatus::Active
                }
                crate::domain::chat::ChatSessionStatus::Archived => {
                    eliteforge_agent_core::SessionStatus::Archived
                }
            },
            created_at: timestamp_to_millis(&s.created_at),
            updated_at: timestamp_to_millis(&s.updated_at),
        }
    }
}
