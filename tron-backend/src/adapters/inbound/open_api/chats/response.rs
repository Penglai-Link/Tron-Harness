use serde::Serialize;
use serde_json::Value;

use crate::{
    application::{agent::AgentInvocationResult, chat::ChatSendResult},
    domain::{
        agent::{AgentAguiEvent, AgentAguiEventSnapshot, AgentAguiProtocolEvent},
        chat::{ChatMessage, ChatSession},
    },
};

fn upper_snake_case(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len() + 4);
    let mut previous_was_lower_or_digit = false;
    for character in value.chars() {
        if character.is_ascii_uppercase() && previous_was_lower_or_digit {
            normalized.push('_');
        }
        if matches!(character, '-' | ' ') {
            normalized.push('_');
            previous_was_lower_or_digit = false;
            continue;
        }
        normalized.push(character.to_ascii_uppercase());
        previous_was_lower_or_digit = character.is_ascii_lowercase() || character.is_ascii_digit();
    }
    normalized
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiChatSessionResponse {
    pub id: i64,
    pub session_code: String,
    pub workspace_code: String,
    pub user_code: Option<String>,
    pub app_code: Option<String>,
    pub task_code: Option<String>,
    pub agent_code: Option<String>,
    pub system_agent_session_id: Option<String>,
    pub title: String,
    pub status: String,
    pub last_run_code: Option<String>,
    pub last_run_status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ChatSession> for OpenApiChatSessionResponse {
    fn from(value: ChatSession) -> Self {
        Self {
            id: value.id,
            session_code: value.session_code,
            workspace_code: value.workspace_code,
            user_code: value.user_code,
            app_code: value.app_code,
            task_code: value.task_code,
            agent_code: value.agent_code,
            system_agent_session_id: value.system_agent_session_id,
            title: value.title,
            status: upper_snake_case(&format!("{:?}", value.status)),
            last_run_code: value.last_run_code,
            last_run_status: value.last_run_status,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiChatSessionGroupResponse {
    pub task_code: Option<String>,
    pub task_title: String,
    pub sessions: Vec<OpenApiChatSessionResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiChatMessageResponse {
    pub id: i64,
    pub message_code: String,
    pub session_code: String,
    pub run_code: Option<String>,
    pub role: String,
    pub channel: String,
    pub actor: String,
    pub content: String,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
    pub created_at: String,
}

impl From<ChatMessage> for OpenApiChatMessageResponse {
    fn from(value: ChatMessage) -> Self {
        Self {
            id: value.id,
            message_code: value.message_code,
            session_code: value.session_code,
            run_code: value.run_code,
            role: upper_snake_case(&format!("{:?}", value.role)),
            channel: upper_snake_case(&format!("{:?}", value.channel)),
            actor: value.actor,
            content: value.content,
            input_tokens: value.input_tokens,
            output_tokens: value.output_tokens,
            total_tokens: value.total_tokens,
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiChatRunSummaryResponse {
    pub run_code: String,
    pub conversation_code: Option<String>,
    pub workspace_code: String,
    pub runtime_adapter_key: String,
    pub status: String,
    pub final_output: String,
    pub step_count: u32,
    pub token_usage: Option<crate::domain::agent::ModelTokenUsage>,
    pub system_agent_session_id: Option<String>,
}

impl From<AgentInvocationResult> for OpenApiChatRunSummaryResponse {
    fn from(value: AgentInvocationResult) -> Self {
        Self {
            run_code: value.run_code,
            conversation_code: value.conversation_code,
            workspace_code: value.workspace_code,
            runtime_adapter_key: value.runtime_adapter_key,
            status: upper_snake_case(&value.status.to_string()),
            final_output: value.final_output,
            step_count: value.step_count,
            token_usage: value.token_usage,
            system_agent_session_id: value.system_agent_session_id,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiChatSendResultResponse {
    pub session: OpenApiChatSessionResponse,
    pub run: OpenApiChatRunSummaryResponse,
    pub messages: Vec<OpenApiChatMessageResponse>,
}

impl From<ChatSendResult> for OpenApiChatSendResultResponse {
    fn from(value: ChatSendResult) -> Self {
        Self {
            session: value.session.into(),
            run: value.run.into(),
            messages: value.messages.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiAguiEventResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub event_name: String,
    pub run_code: Option<String>,
    pub stream_key: String,
    pub occurred_at: String,
    pub outcome: String,
    pub metadata: std::collections::BTreeMap<String, String>,
}

impl From<AgentAguiEvent> for OpenApiAguiEventResponse {
    fn from(value: AgentAguiEvent) -> Self {
        Self {
            id: value.id,
            event_type: value.event_type,
            event_name: value.event_name,
            run_code: value.run_code,
            stream_key: value.stream_key,
            occurred_at: value.occurred_at,
            outcome: value.outcome,
            metadata: value.metadata,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiAguiEventSnapshotResponse {
    pub stream_key: String,
    pub active_streams: usize,
    pub total_streams: usize,
    pub events: Vec<OpenApiAguiEventResponse>,
}

impl From<AgentAguiEventSnapshot> for OpenApiAguiEventSnapshotResponse {
    fn from(value: AgentAguiEventSnapshot) -> Self {
        Self {
            stream_key: value.stream_key,
            active_streams: value.active_streams,
            total_streams: value.total_streams,
            events: value.events.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiAguiProtocolEventResponse {
    pub sse_event: String,
    pub data: Value,
}

impl From<AgentAguiProtocolEvent> for OpenApiAguiProtocolEventResponse {
    fn from(value: AgentAguiProtocolEvent) -> Self {
        Self {
            sse_event: value.sse_event,
            data: value.data,
        }
    }
}
