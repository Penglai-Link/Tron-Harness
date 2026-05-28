use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use serde::{Deserialize, Serialize};

use super::AgentRunStatus;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunHistoryRecord {
    pub id: i64,
    pub run_code: String,
    pub conversation_code: Option<String>,
    pub workspace_code: String,
    pub user_code: Option<String>,
    pub app_code: Option<String>,
    pub task_code: Option<String>,
    pub model_code: String,
    pub status: AgentRunStatus,
    pub outcome: String,
    pub step_count: u32,
    pub observation_count: u32,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
    pub output_preview: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub finished_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedAgentRunHistoryRecords<T> {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<T>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunHistoryQuery {
    pub workspace_code: Option<String>,
    pub conversation_code: Option<String>,
    pub current: Option<usize>,
    pub size: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentRunHistoryError {
    StorageFailure,
    RunNotFound,
}

impl Display for AgentRunHistoryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::StorageFailure => f.write_str("agent run history storage failure"),
            Self::RunNotFound => f.write_str("agent run was not found"),
        }
    }
}

impl Error for AgentRunHistoryError {}
