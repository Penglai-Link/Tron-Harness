use serde::{Deserialize, Serialize};

use crate::domain::agent::{AgentMessage, AgentModelConfig};

#[derive(Clone)]
pub struct SessionSummaryJobContext {
    pub session_code: String,
    pub workspace_code: String,
    pub run_code: String,
    pub locale: String,
    pub model: AgentModelConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummaryRunOutcome {
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
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
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGoalCommand {
    pub session_code: String,
    pub goal_code: String,
    pub objective: String,
    pub max_runs: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalSessionCommand {
    pub session_code: String,
    pub goal_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedModelContext {
    /// Authorized sources only; the SDK asynchronous assembler owns model admission.
    pub sources: Option<PreparedContextSources>,
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

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedContextSources {
    pub revision: eliteforge_agent_memory::MemoryRevision,
    pub prefix: Vec<AgentMessage>,
    pub checkpoint: Vec<AgentMessage>,
    pub family: Vec<AgentMessage>,
    pub working: Vec<AgentMessage>,
    pub history: Vec<AgentMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextBudgetBreakdown {
    /// Subset of prompt_prefix_tokens; kept separate for the context inspector.
    pub application_context_tokens: u32,
    pub application_context_message_count: usize,
    pub prompt_prefix_message_count: usize,
    pub prompt_prefix_tokens: u32,
    pub checkpoint_tokens: u32,
    pub session_family_context_tokens: u32,
    pub working_state_tokens: u32,
    pub recent_transcript_tokens: u32,
    pub raw_tail_transcript_tokens: u32,
    pub compressed_history_tokens: u32,
    pub tool_schema_tokens: u32,
    pub tool_schema_budget_tokens: Option<u32>,
    pub estimated_input_tokens: u32,
    pub prompt_prefix_budget_tokens: Option<u32>,
    pub checkpoint_budget_tokens: Option<u32>,
    pub session_family_context_budget_tokens: Option<u32>,
    pub working_state_budget_tokens: Option<u32>,
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
