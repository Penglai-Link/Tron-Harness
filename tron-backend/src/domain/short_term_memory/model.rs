use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl TodoStatus {
    pub fn code(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_code(value: &str) -> Result<Self, ShortTermMemoryError> {
        match value {
            "pending" => Ok(Self::Pending),
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(ShortTermMemoryError::TodoInvalid),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoPriority {
    High,
    Medium,
    Low,
}

impl TodoPriority {
    pub fn code(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }

    pub fn from_code(value: &str) -> Result<Self, ShortTermMemoryError> {
        match value {
            "high" => Ok(Self::High),
            "medium" => Ok(Self::Medium),
            "low" => Ok(Self::Low),
            _ => Err(ShortTermMemoryError::TodoInvalid),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortTermMemoryTodo {
    pub id: i64,
    pub todo_code: String,
    pub session_code: String,
    pub workspace_code: String,
    pub position: i64,
    pub content: String,
    pub status: TodoStatus,
    pub priority: TodoPriority,
    pub created_by_run_code: Option<String>,
    pub updated_by_run_code: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortTermMemoryGoal {
    pub id: i64,
    pub goal_code: String,
    pub session_code: String,
    pub workspace_code: String,
    pub user_code: Option<String>,
    pub task_code: Option<String>,
    pub active_family_code: Option<String>,
    pub objective_markdown: String,
    pub status: String,
    pub revision: i64,
    pub runs_used: u32,
    pub max_runs: u32,
    pub last_run_code: Option<String>,
    pub last_input_code: Option<String>,
    pub last_verdict: Option<String>,
    pub last_reason: Option<String>,
    pub blocked_reason: Option<String>,
    pub required_user_input: Option<String>,
    pub consecutive_judge_failures: u32,
    pub created_by_run_code: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub paused_at: Option<String>,
    pub blocked_at: Option<String>,
    pub completed_at: Option<String>,
    pub cancelled_at: Option<String>,
    pub cleared_at: Option<String>,
    pub metadata_json: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortTermMemoryGoalCriterion {
    pub id: i64,
    pub criterion_code: String,
    pub goal_code: String,
    pub session_code: String,
    pub position: i64,
    pub text: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub removed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortTermMemoryGoalView {
    pub goal: ShortTermMemoryGoal,
    #[serde(default, skip_serializing)]
    pub criteria: Vec<ShortTermMemoryGoalCriterion>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShortTermMemoryError {
    SessionNotFound,
    CompactionConflict,
    RunConflict,
    CompactionInputTooLarge,
    CompactionModelFailed,
    ContextBudgetExceeded,
    TodoInvalid,
    GoalNotFound,
    GoalConflict,
    GoalStateConflict,
    GoalInvalid,
    StorageFailure,
}

impl Display for ShortTermMemoryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::SessionNotFound => f.write_str("chat session was not found"),
            Self::CompactionConflict => f.write_str("session has an active agent run"),
            Self::RunConflict => f.write_str("session already has an active agent run"),
            Self::CompactionInputTooLarge => f.write_str("compaction input is too large"),
            Self::CompactionModelFailed => f.write_str("compaction model failed"),
            Self::ContextBudgetExceeded => f.write_str("short-term memory context exceeded budget"),
            Self::TodoInvalid => f.write_str("todo update is invalid"),
            Self::GoalNotFound => f.write_str("session goal was not found"),
            Self::GoalConflict => f.write_str("session already has an active goal"),
            Self::GoalStateConflict => f.write_str("session goal was concurrently modified"),
            Self::GoalInvalid => f.write_str("goal command is invalid"),
            Self::StorageFailure => f.write_str("short-term memory storage failure"),
        }
    }
}

impl Error for ShortTermMemoryError {}
