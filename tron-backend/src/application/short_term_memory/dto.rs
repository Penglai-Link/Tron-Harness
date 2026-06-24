use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::agent::{
    AgentMessage, ModelTokenUsage, TokenEstimateSnapshot, TokenUsageAuditSnapshot,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCompactionCommand {
    pub session_code: String,
    pub reason: Option<String>,
    pub note: Option<String>,
    #[serde(default)]
    pub todo_enabled: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolTranscriptStatus {
    Completed,
    Failed,
}

impl ToolTranscriptStatus {
    pub fn event_type(self) -> &'static str {
        match self {
            Self::Completed => "agent.tool.completed",
            Self::Failed => "agent.tool.failed",
        }
    }

    pub fn part_status(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactionContextPolicy {
    pub todo_enabled: bool,
}

impl CompactionContextPolicy {
    pub fn enabled() -> Self {
        Self { todo_enabled: true }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedCompactionInput {
    pub session_code: String,
    pub workspace_code: String,
    pub reason: Option<String>,
    pub note: Option<String>,
    pub todo_enabled: bool,
    pub enforce_idle: bool,
    pub strategy: String,
    pub triggered_by: String,
    pub source: String,
    pub previous_checkpoint_message_code: Option<String>,
    pub previous_checkpoint_included_recent: bool,
    pub source_start_seq: i64,
    pub source_end_seq: i64,
    pub tail_start_seq: i64,
    pub source_message_count: usize,
    pub source_token_estimate: u32,
    pub tail_token_estimate: u32,
    pub compaction_input_token_estimate: u32,
    pub max_input_tokens: u32,
    pub recent: String,
    pub prompt: String,
    pub prompt_hash: String,
    pub pinned_start_seq: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCompactionFailure {
    pub error_code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelCompactionOutput {
    pub summary: String,
    pub token_usage: Option<ModelTokenUsage>,
    pub provider_metadata: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRequestTranscriptFacts {
    pub message_count: usize,
    pub part_count: usize,
    pub estimated_tokens: u32,
    pub message_hashes: Vec<String>,
    pub tool_schema_token_estimate: u32,
    pub protocol: String,
    pub previous_response_id_present: bool,
    pub context_window_estimate: Option<u32>,
    pub checkpoint_code: Option<String>,
    pub usage_estimate: Option<TokenEstimateSnapshot>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceTodosCommand {
    pub todos: Vec<TodoUpdateItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoUpdateItem {
    pub todo_code: Option<String>,
    pub position: i64,
    pub content: String,
    pub status: String,
    pub priority: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetGoalCommand {
    pub session_code: String,
    pub objective: String,
    pub max_runs: Option<u32>,
    #[serde(default)]
    pub criteria: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalSessionCommand {
    pub session_code: String,
    pub goal_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddGoalCriterionCommand {
    pub session_code: String,
    pub goal_code: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveGoalCriterionCommand {
    pub session_code: String,
    pub goal_code: String,
    pub criterion_code: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GoalLoopDecision {
    pub goal_code: String,
    pub verdict: String,
    pub reason: String,
    pub continuation_prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedGoalJudgeInput {
    pub session_code: String,
    pub workspace_code: String,
    pub goal_code: String,
    pub run_code: String,
    pub expected_revision: i64,
    pub runs_used_after_judge: u32,
    pub max_runs: u32,
    pub prompt: String,
    pub prompt_hash: String,
    pub final_output_hash: String,
    pub input_token_estimate: u32,
    pub input_preview: Value,
    pub no_progress_candidate: bool,
    pub no_progress_detected: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalJudgeOutput {
    pub verdict: String,
    pub reason: String,
    #[serde(default)]
    pub completion_evidence: Vec<String>,
    #[serde(default)]
    pub missing_evidence: Vec<String>,
    #[serde(default)]
    pub required_user_input: Option<String>,
    #[serde(default)]
    pub next_step_hint: Option<String>,
    #[serde(default)]
    pub continuation_prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalJudgeFailure {
    pub error_code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedModelContext {
    pub messages: Vec<AgentMessage>,
    pub rebased: bool,
    pub context_hash: Option<String>,
    pub checkpoint_code: Option<String>,
    pub estimated_tokens: u32,
    pub message_token_estimate: u32,
    pub tool_schema_token_estimate: u32,
    pub context_window_limit: Option<u32>,
    pub effective_input_budget: Option<u32>,
    pub output_token_reserve: Option<u32>,
    pub protocol_overhead_reserve: Option<u32>,
    pub safety_margin_tokens: Option<u32>,
    pub over_budget: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelResponseTranscriptFacts {
    pub usage_audit: Option<TokenUsageAuditSnapshot>,
}
