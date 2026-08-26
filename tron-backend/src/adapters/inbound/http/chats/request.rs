use serde::Deserialize;

use crate::{
    application::chat::ChatResumeEntry,
    domain::chat::{ChatMessageAttachment, ChatUserInputAnswer},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetActiveChatRunQuery {
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
    pub session_code: Option<String>,
    pub expected_run_code: Option<String>,
    pub content: Option<String>,
    #[serde(default)]
    pub attachments: Vec<ChatMessageAttachment>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnswerChatUserInputRequest {
    pub answers: Option<Vec<ChatUserInputAnswer>>,
    #[serde(default)]
    pub resume: Vec<ChatResumeEntry>,
}
