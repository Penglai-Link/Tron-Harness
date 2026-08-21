use serde::Deserialize;

use crate::domain::{
    agent::{AgentRunLimits, EffectiveTool},
    chat::{
        ChatClientCapabilities, ChatMessageAttachment, ChatSessionSourceType, ChatUserInputAnswer,
    },
    reasoning::ReasoningEffort,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiChatSessionQuery {
    pub workspace_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiCreateChatSessionRequest {
    pub workspace_code: Option<String>,
    pub title: Option<String>,
    pub app_code: Option<String>,
    pub task_code: Option<String>,
    pub source_type: Option<ChatSessionSourceType>,
    pub agent_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiUpdateChatSessionRequest {
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiSendChatMessageRequest {
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
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenApiSteerChatMessageRequest {
    pub session_code: Option<String>,
    pub expected_run_code: Option<String>,
    pub content: Option<String>,
    #[serde(default)]
    pub attachments: Vec<ChatMessageAttachment>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenApiGetActiveChatRunQuery {
    pub session_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenApiStopChatRunRequest {
    pub session_code: Option<String>,
    pub expected_run_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenApiAnswerChatUserInputRequest {
    pub answers: Vec<ChatUserInputAnswer>,
}
