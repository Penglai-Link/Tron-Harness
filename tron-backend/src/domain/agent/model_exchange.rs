use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{AgentMessage, EffectiveTool};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRequest {
    pub messages: Vec<AgentMessage>,
    pub tools: Vec<EffectiveTool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub tool_name: String,
    pub input: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelResponse {
    FinalOutput(String),
    ToolCall(ToolCall),
}

impl ModelResponse {
    pub fn final_output(output: impl Into<String>) -> Self {
        Self::FinalOutput(output.into())
    }

    pub fn tool_call(tool_name: impl Into<String>, input: Value) -> Self {
        Self::ToolCall(ToolCall {
            tool_name: tool_name.into(),
            input,
        })
    }
}
