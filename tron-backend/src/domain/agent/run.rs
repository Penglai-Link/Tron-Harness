use serde::{Deserialize, Serialize};

use super::{ModelTokenUsage, TokenEstimateSnapshot, TokenUsageAuditSnapshot, ToolObservation};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunResult {
    pub run_code: String,
    pub conversation_code: Option<String>,
    pub workspace_code: String,
    pub final_output: String,
    pub step_count: u32,
    pub token_usage: Option<ModelTokenUsage>,
    pub latest_request_estimate: Option<TokenEstimateSnapshot>,
    pub latest_usage_audit: Option<TokenUsageAuditSnapshot>,
    pub system_agent_session_id: Option<String>,
    pub observations: Vec<ToolObservation>,
    #[serde(default)]
    pub native: Option<AgentNativeRunOutcome>,
    #[serde(skip)]
    pub sdk_execution: Option<AgentNativeExecutionSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentRunStatus {
    Completed,
    Failed,
}

/// SDK completion and interaction identity are separate from product history status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentNativeRunOutcome {
    pub run_id: String,
    pub status: AgentNativeRunStatus,
    pub interactions: Vec<AgentNativeInteraction>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentNativeRunStatus {
    Completed,
    AwaitingInput,
    StepLimit,
    ToolCallLimit,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentNativeInteraction {
    pub id: String,
    pub kind: AgentNativeInteractionKind,
    pub prompt: serde_json::Value,
    pub response_schema: serde_json::Value,
    pub presentation: Option<serde_json::Value>,
    pub expires_at_unix_ms: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentNativeInteractionKind {
    Input,
    Approval {
        #[serde(rename = "invocationId")]
        invocation_id: String,
    },
}

/// Internal conservative execution observation. Nullable provider usage is separate.
/// Limits are a complete inert snapshot interpreted only at the application boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentNativeExecutionSnapshot {
    pub limits: serde_json::Value,
    pub budget_usage: AgentChargedUsage,
    pub model_calls: usize,
    #[serde(default)]
    pub child_runs: usize,
    pub steps: usize,
    pub tool_calls: usize,
    pub deadline_unix_ms: u64,
    pub opaque: bool,
    #[serde(default)]
    pub incomplete: bool,
    #[serde(default)]
    pub historical_calls_bound: Option<usize>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentChargedUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}
