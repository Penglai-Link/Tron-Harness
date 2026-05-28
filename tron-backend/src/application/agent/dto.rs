use serde::{Deserialize, Serialize};

use crate::domain::agent::{AgentModelConfig, AgentRunLimits, EffectiveTool};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInvocationCommand {
    #[serde(skip)]
    pub run_code: Option<String>,
    pub prompt: Option<String>,
    pub workspace_code: Option<String>,
    pub conversation_code: Option<String>,
    pub app_code: Option<String>,
    pub task_code: Option<String>,
    pub model: Option<AgentModelConfig>,
    pub limits: Option<AgentRunLimits>,
    #[serde(default)]
    pub tools: Vec<EffectiveTool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInvocationResult {
    pub run_code: String,
    pub conversation_code: Option<String>,
    pub workspace_code: String,
    pub status: &'static str,
    pub final_output: String,
    pub step_count: u32,
}
