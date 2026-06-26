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
    pub coverage: String,
    // Legacy replay field name: this is the checkpoint format version, not the old selector policy.
    pub selector_version: u32,
    pub selection_source_end_seq: i64,
    pub trigger_ratio_bps: u32,
    pub retained_history_ratio_bps: u32,
    pub effective_retained_history_tokens: u32,
    pub source_start_seq: i64,
    pub source_end_seq: i64,
    pub tail_start_seq: i64,
    pub source_message_count: usize,
    pub selection_source_message_count: usize,
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
    pub budget_breakdown: Option<ContextBudgetBreakdown>,
    pub over_budget_reason: Option<String>,
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
    pub budget_breakdown: Option<ContextBudgetBreakdown>,
    pub over_budget_reason: Option<String>,
    pub context_window_limit: Option<u32>,
    pub effective_input_budget: Option<u32>,
    pub output_token_reserve: Option<u32>,
    pub over_budget: bool,
    pub compaction_trigger_reached: bool,
    pub compaction_required: bool,
    pub compaction_tail_start_seq: Option<i64>,
    pub compaction_trigger_tokens: Option<u32>,
    pub effective_retained_history_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextBudgetBreakdown {
    pub prompt_prefix_tokens: u32,
    pub checkpoint_tokens: u32,
    pub session_family_context_tokens: u32,
    pub working_state_tokens: u32,
    pub recent_transcript_tokens: u32,
    pub raw_tail_transcript_tokens: u32,
    pub compressed_history_tokens: u32,
    pub tool_schema_tokens: u32,
    pub estimated_input_tokens: u32,
    pub prompt_prefix_budget_tokens: Option<u32>,
    pub effective_input_budget: Option<u32>,
    pub context_window_limit: Option<u32>,
    pub output_token_reserve: Option<u32>,
    pub compaction_trigger_ratio_bps: Option<u32>,
    pub compaction_trigger_tokens: Option<u32>,
    pub compaction_trigger_reached: bool,
    pub input_budget_exceeded: bool,
    pub retained_history_ratio_bps: Option<u32>,
    pub effective_retained_history_tokens: Option<u32>,
    pub current_session_history_budget_tokens: Option<u32>,
    pub history_source_message_count: Option<usize>,
    pub raw_tail_message_count: Option<usize>,
    pub compressed_history_message_count: Option<usize>,
    pub raw_tail_start_seq: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelResponseTranscriptFacts {
    pub usage_audit: Option<TokenUsageAuditSnapshot>,
}
