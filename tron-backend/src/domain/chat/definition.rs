use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSession {
    pub id: i64,
    pub session_code: String,
    pub workspace_code: String,
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
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSessionGroup {
    pub task_code: Option<String>,
    pub task_title: String,
    pub source_type: ChatSessionSourceType,
    pub source_code: Option<String>,
    pub sessions: Vec<ChatSession>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatError {
    MessageRequired,
    SessionNotFound,
    SessionTitleRequired,
    StorageFailure,
    WorkspaceForbidden,
    WorkspaceRequired,
}

impl Display for ChatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MessageRequired => f.write_str("chat message content is required"),
            Self::SessionNotFound => f.write_str("chat session was not found"),
            Self::SessionTitleRequired => f.write_str("chat session title is required"),
            Self::StorageFailure => f.write_str("chat storage failure"),
            Self::WorkspaceForbidden => f.write_str("workspace access is forbidden"),
            Self::WorkspaceRequired => f.write_str("workspaceCode is required"),
        }
    }
}

impl Error for ChatError {}
