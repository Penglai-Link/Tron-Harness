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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentRunStatus {
    Completed,
    Failed,
}
