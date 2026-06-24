use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::agent::{
    AgentModelConfig, AgentRunLimits, EffectiveTool, ModelTokenUsage, TokenEstimateSnapshot,
    TokenUsageAuditSnapshot,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInvocationCommand {
    #[serde(skip)]
    pub run_code: Option<String>,
    pub prompt: Option<String>,
    pub workspace_code: Option<String>,
    pub conversation_code: Option<String>,
    pub agent_code: Option<String>,
    pub system_model_id: Option<String>,
    #[serde(skip)]
    pub system_agent_session_id: Option<String>,
    pub app_code: Option<String>,
    pub task_code: Option<String>,
    pub model: Option<AgentModelConfig>,
    pub limits: Option<AgentRunLimits>,
    #[serde(default)]
    pub tools: Vec<EffectiveTool>,
    #[serde(skip)]
    pub transcript_input_kind: Option<String>,
    #[serde(skip)]
    pub transcript_input_metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInvocationResult {
    pub run_code: String,
    pub conversation_code: Option<String>,
    pub workspace_code: String,
    pub runtime_adapter_key: String,
    pub status: &'static str,
    pub final_output: String,
    pub step_count: u32,
    pub token_usage: Option<ModelTokenUsage>,
    pub latest_request_estimate: Option<TokenEstimateSnapshot>,
    pub latest_usage_audit: Option<TokenUsageAuditSnapshot>,
    pub system_agent_session_id: Option<String>,
}
