use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::{
    agent::{
        AgentExecutionScope, AgentModelConfig, AgentRunLimits, EffectiveTool, ModelTokenUsage,
        TokenEstimateSnapshot, TokenUsageAuditSnapshot, ToolObservation,
    },
    chat::ChatMessageAttachment,
    reasoning::ReasoningEffort,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentPageContext {
    pub title: String,
    pub path: String,
}

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
    #[serde(skip)]
    pub execution_scope_override: Option<AgentExecutionScope>,
    pub task_code: Option<String>,
    pub model: Option<AgentModelConfig>,
    pub limits: Option<AgentRunLimits>,
    #[serde(default)]
    pub tools: Vec<EffectiveTool>,
    #[serde(default)]
    pub attachments: Vec<ChatMessageAttachment>,
    #[serde(default)]
    pub(crate) page_context: Option<AgentPageContext>,
    #[serde(skip)]
    pub supports_user_input_requests: bool,
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
    pub model_code: String,
    pub runtime_adapter_key: String,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub status: &'static str,
    pub final_output: String,
    pub step_count: u32,
    pub token_usage: Option<ModelTokenUsage>,
    pub latest_request_estimate: Option<TokenEstimateSnapshot>,
    pub latest_usage_audit: Option<TokenUsageAuditSnapshot>,
    pub system_agent_session_id: Option<String>,
    #[serde(skip)]
    pub observations: Vec<ToolObservation>,
}
