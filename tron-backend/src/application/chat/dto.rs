use serde::{Deserialize, Serialize};

use crate::{
    application::agent::AgentInvocationResult,
    domain::{
        agent::{AgentRunLimits, EffectiveTool},
        chat::{ChatMessage, ChatMessageAttachment, ChatSession, ChatSessionSourceType},
        reasoning::ReasoningEffort,
    },
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSessionQuery {
    pub workspace_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateChatSessionCommand {
    pub workspace_code: Option<String>,
    pub title: Option<String>,
    pub app_code: Option<String>,
    pub task_code: Option<String>,
    pub source_type: Option<ChatSessionSourceType>,
    pub source_code: Option<String>,
    pub agent_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateChatSessionCommand {
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendChatMessageCommand {
    #[serde(skip)]
    pub run_code: Option<String>,
    pub content: Option<String>,
    pub workspace_code: Option<String>,
    pub app_code: Option<String>,
    pub task_code: Option<String>,
    pub agent_code: Option<String>,
    pub model_code: Option<String>,
    pub system_model_id: Option<String>,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub limits: Option<AgentRunLimits>,
    #[serde(default)]
    pub tools: Vec<EffectiveTool>,
    #[serde(default)]
    pub attachments: Vec<ChatMessageAttachment>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteerChatMessageCommand {
    pub session_code: Option<String>,
    pub expected_run_code: Option<String>,
    pub content: Option<String>,
    #[serde(default)]
    pub attachments: Vec<ChatMessageAttachment>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSendResult {
    pub session: ChatSession,
    pub run: AgentInvocationResult,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSteerResult {
    pub session_code: String,
    pub run_code: String,
    pub message: ChatMessage,
}
