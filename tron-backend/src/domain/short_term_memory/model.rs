use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortTermMemoryInput {
    pub id: i64,
    pub input_code: String,
    pub session_code: String,
    pub workspace_code: String,
    pub run_code: Option<String>,
    pub input_kind: String,
    pub delivery: String,
    pub status: String,
    pub priority: i64,
    pub idempotency_key: Option<String>,
    pub prompt_json: Value,
    pub metadata_json: Option<Value>,
    pub admitted_seq: Option<i64>,
    pub promoted_seq: Option<i64>,
    pub created_at: String,
    pub promoted_at: Option<String>,
    pub consumed_at: Option<String>,
    pub cancelled_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortTermMemoryEvent {
    pub id: i64,
    pub event_code: String,
    pub session_code: String,
    pub workspace_code: String,
    pub seq: i64,
    pub run_code: Option<String>,
    pub input_code: Option<String>,
    pub message_code: Option<String>,
    pub part_code: Option<String>,
    pub event_type: String,
    pub event_source: String,
    pub payload_json: Value,
    pub metadata_json: Option<Value>,
    pub occurred_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortTermMemoryMessage {
    pub id: i64,
    pub message_code: String,
    pub session_code: String,
    pub workspace_code: String,
    pub run_code: Option<String>,
    pub seq: i64,
    pub role: String,
    pub message_type: String,
    pub status: String,
    pub agent_code: Option<String>,
    pub model_code: Option<String>,
    pub model_provider: Option<String>,
    pub model_name: Option<String>,
    pub model_protocol: Option<String>,
    pub finish_reason: Option<String>,
    pub error_json: Option<Value>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub reasoning_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
    pub metadata_json: Option<Value>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortTermMemoryPart {
    pub id: i64,
    pub part_code: String,
    pub session_code: String,
    pub workspace_code: String,
    pub message_code: String,
    pub seq: i64,
    pub part_type: String,
    pub status: String,
    pub content_text: Option<String>,
    pub content_json: Option<Value>,
    pub provider_metadata_json: Option<Value>,
    pub visibility: String,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortTermMemoryContextEpoch {
    pub id: i64,
    pub epoch_code: String,
    pub session_code: String,
    pub workspace_code: String,
    pub baseline_seq: i64,
    pub replacement_seq: i64,
    pub agent_code: Option<String>,
    pub prompt_prefix_message_count: u32,
    pub prompt_prefix_hash: Option<String>,
    pub prompt_prefix_preview: Option<String>,
    pub snapshot_json: Value,
    pub system_hash: Option<String>,
    pub developer_hash: Option<String>,
    pub tool_hash: Option<String>,
    pub skill_hash: Option<String>,
    pub variable_hash: Option<String>,
    pub execution_scope_hash: Option<String>,
    pub memory_hash: Option<String>,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
}

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
pub struct ShortTermMemoryCompaction {
    pub session_code: String,
    pub checkpoint_message_code: String,
    pub status: String,
    pub source_start_seq: i64,
    pub source_end_seq: i64,
    pub tail_start_seq: i64,
    pub selector_version: u32,
    pub coverage: Option<String>,
    pub summary_token_estimate: u32,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortTermMemoryFamily {
    pub id: i64,
    pub family_code: String,
    pub family_kind: String,
    pub family_key: String,
    pub workspace_code: String,
    pub agent_code: String,
    pub user_code: Option<String>,
    pub app_code: Option<String>,
    pub requirement_code: Option<String>,
    pub task_code: Option<String>,
    pub parent_family_code: Option<String>,
    pub root_session_code: Option<String>,
    pub active_session_code: Option<String>,
    pub active_run_code: Option<String>,
    pub latest_snapshot_code: Option<String>,
    pub title: Option<String>,
    pub status: String,
    pub load_policy: String,
    pub load_policy_json: Option<Value>,
    pub metadata_json: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
    pub last_activity_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortTermMemoryFamilyMember {
    pub id: i64,
    pub family_code: String,
    pub session_code: String,
    pub membership_role: String,
    pub parent_session_code: Option<String>,
    pub member_kind: String,
    pub origin_kind: String,
    pub model_context_enabled: bool,
    pub load_weight_override: Option<f64>,
    pub load_limits_json: Option<Value>,
    pub first_run_code: Option<String>,
    pub last_run_code: Option<String>,
    pub joined_at: String,
    pub last_activity_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortTermMemoryFamilySnapshot {
    pub id: i64,
    pub snapshot_code: String,
    pub family_code: String,
    pub session_code: String,
    pub run_code: Option<String>,
    pub snapshot_kind: String,
    pub status: String,
    pub source_start_seq: Option<i64>,
    pub source_end_seq: Option<i64>,
    pub source_checkpoint_message_code: Option<String>,
    pub summary_markdown: Option<String>,
    pub token_estimate: Option<u32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub metadata_json: Option<Value>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub expires_at: Option<String>,
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
pub struct ShortTermMemoryGoalEvent {
    pub id: i64,
    pub event_code: String,
    pub goal_code: String,
    pub session_code: String,
    pub workspace_code: String,
    pub run_code: Option<String>,
    pub input_code: Option<String>,
    pub event_kind: String,
    pub event_seq: i64,
    pub summary: Option<String>,
    pub payload_json: Option<Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortTermMemoryGoalView {
    pub goal: ShortTermMemoryGoal,
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
