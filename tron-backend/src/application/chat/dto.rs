use serde::{Deserialize, Serialize};

use crate::{
    application::agent::AgentInvocationResult,
    domain::{
        agent::{AgentRunLimits, EffectiveTool},
        chat::{ChatMessage, ChatSession},
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
    pub limits: Option<AgentRunLimits>,
    #[serde(default)]
    pub tools: Vec<EffectiveTool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSendResult {
    pub session: ChatSession,
    pub run: AgentInvocationResult,
    pub messages: Vec<ChatMessage>,
}
