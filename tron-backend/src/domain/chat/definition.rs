use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChatSessionStatus {
    Active,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChatMessageRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChatMessageChannel {
    MainAgent,
    TaskAgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChatSessionSourceType {
    Manual,
    Task,
    Automation,
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSession {
    pub id: i64,
    pub session_code: String,
    pub workspace_code: String,
    pub project_code: Option<String>,
    pub user_code: Option<String>,
    pub app_code: Option<String>,
    pub task_code: Option<String>,
    pub source_type: ChatSessionSourceType,
    pub source_code: Option<String>,
    pub agent_code: Option<String>,
    pub system_agent_session_id: Option<String>,
    pub title: String,
    pub status: ChatSessionStatus,
    pub last_run_code: Option<String>,
    pub last_run_status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: i64,
    pub message_code: String,
    pub session_code: String,
    pub run_code: Option<String>,
    pub role: ChatMessageRole,
    pub channel: ChatMessageChannel,
    pub actor: String,
    pub content: String,
    #[serde(default)]
    pub attachments: Vec<ChatMessageAttachment>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageAttachment {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub title: Option<String>,
    pub summary: String,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub resource_code: Option<String>,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub byte_size: Option<u64>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSessionGroup {
    pub task_code: Option<String>,
    pub task_title: String,
    pub source_type: ChatSessionSourceType,
    pub source_code: Option<String>,
    pub pending_user_input_count: u32,
    pub pending_user_input_session_codes: Vec<String>,
    pub sessions: Vec<ChatSession>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatClientCapabilities {
    #[serde(default)]
    pub user_input_requests: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ChatUserInputRequestStatus {
    Pending,
    Answered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ChatUserInputAnswerKind {
    Option,
    Other,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatUserInputOption {
    pub option_code: String,
    pub label: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatUserInputQuestion {
    pub question_code: String,
    pub question: String,
    pub options: Vec<ChatUserInputOption>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatUserInputAnswer {
    pub question_code: String,
    pub answer_kind: ChatUserInputAnswerKind,
    pub option_code: Option<String>,
    pub other_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatUserInputRequest {
    pub request_code: String,
    pub session_code: String,
    pub run_code: String,
    pub assistant_message_code: Option<String>,
    pub intro: String,
    pub questions: Vec<ChatUserInputQuestion>,
    pub answers: Option<Vec<ChatUserInputAnswer>>,
    pub model_code: Option<String>,
    pub reasoning_effort: Option<crate::domain::reasoning::ReasoningEffort>,
    pub status: ChatUserInputRequestStatus,
    pub requested_at: String,
    pub answered_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatError {
    AttachmentInvalid(String),
    AttachmentNotFound,
    MessageRequired,
    SessionNotFound,
    SessionTitleRequired,
    StorageFailure,
    WorkspaceForbidden,
    WorkspaceRequired,
    ProjectRequired,
    ProjectInvalid,
    UserInputAlreadyAnswered,
    UserInputAnswerInvalid(String),
    UserInputPending,
    UserInputRequestNotFound,
}

impl Display for ChatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::AttachmentInvalid(message) => write!(f, "chat attachment is invalid: {message}"),
            Self::AttachmentNotFound => f.write_str("chat attachment was not found"),
            Self::MessageRequired => f.write_str("chat message content is required"),
            Self::SessionNotFound => f.write_str("chat session was not found"),
            Self::SessionTitleRequired => f.write_str("chat session title is required"),
            Self::StorageFailure => f.write_str("chat storage failure"),
            Self::WorkspaceForbidden => f.write_str("workspace access is forbidden"),
            Self::WorkspaceRequired => f.write_str("workspaceCode is required"),
            Self::ProjectRequired => {
                f.write_str("projectCode is required for a manual chat session")
            }
            Self::ProjectInvalid => f.write_str("chat project is invalid for this workspace"),
            Self::UserInputAlreadyAnswered => {
                f.write_str("chat user input request is already answered")
            }
            Self::UserInputAnswerInvalid(message) => {
                write!(f, "chat user input answer is invalid: {message}")
            }
            Self::UserInputPending => f.write_str("chat session is waiting for a user answer"),
            Self::UserInputRequestNotFound => f.write_str("chat user input request was not found"),
        }
    }
}

impl Error for ChatError {}
