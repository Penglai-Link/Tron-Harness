use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    application::agent::AgentInvocationResult,
    domain::{
        agent::{AgentRunLimits, EffectiveTool},
        chat::{
            ChatClientCapabilities, ChatMessage, ChatMessageAttachment, ChatSession,
            ChatSessionSourceType,
        },
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
    pub project_code: Option<String>,
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
pub struct ChatResumeEntry {
    pub interrupt_id: String,
    pub status: String,
    pub payload: Value,
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
    #[serde(default)]
    pub client_capabilities: Option<ChatClientCapabilities>,
    #[serde(default)]
    pub resume: Vec<ChatResumeEntry>,
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

#[derive(Debug, Clone)]
pub struct ChatSessionAttachmentUploadCommand {
    pub kind: String,
    pub summary: Option<String>,
    pub metadata: Value,
    pub file_name: String,
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSendResult {
    pub session: ChatSession,
    pub run: AgentInvocationResult,
    pub messages: Vec<ChatMessage>,
    pub user_input_request: Option<crate::domain::chat::ChatUserInputRequest>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSteerResult {
    pub session_code: String,
    pub run_code: String,
    pub message: ChatMessage,
}
