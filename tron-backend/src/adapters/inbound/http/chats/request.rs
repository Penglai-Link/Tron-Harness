use serde::Deserialize;

use crate::{
    application::chat::ChatResumeEntry,
    domain::chat::{ChatMessageAttachment, ChatUserInputAnswer},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetActiveChatRunQuery {
    pub workspace_code: Option<String>,
    pub session_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StopChatRunRequest {
    pub session_code: Option<String>,
    pub expected_run_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SteerChatMessageRequest {
    #[serde(default)]
    pub annotations: Vec<eliteforge_agent_core::MessageAnnotation>,
    pub session_code: Option<String>,
    pub expected_run_code: Option<String>,
    pub content: Option<String>,
    #[serde(default)]
    pub attachments: Vec<ChatMessageAttachment>,
    #[serde(default)]
    pub requested_skills: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnswerChatUserInputRequest {
    pub client_capabilities: Option<crate::domain::chat::ChatClientCapabilities>,
    pub a2ui_metadata: Option<serde_json::Value>,
    pub answers: Option<Vec<ChatUserInputAnswer>>,
    #[serde(default)]
    pub resume: Vec<ChatResumeEntry>,
}
