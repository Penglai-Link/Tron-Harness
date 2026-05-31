use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{AgentMessage, EffectiveTool};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRequest {
    pub messages: Vec<AgentMessage>,
    pub tools: Vec<EffectiveTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_agent_session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub tool_name: String,
    pub input: Value,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelTokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelResponse {
    FinalOutput {
        output: String,
        token_usage: Option<ModelTokenUsage>,
        system_agent_session_id: Option<String>,
    },
    ToolCall {
        call: ToolCall,
        token_usage: Option<ModelTokenUsage>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStreamEvent {
    pub kind: ModelStreamEventKind,
    pub delta: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelStreamEventKind {
    Text,
    Reasoning,
    ReasoningSummary,
}

impl ModelStreamEvent {
    pub fn text_delta(delta: impl Into<String>) -> Self {
        Self {
            kind: ModelStreamEventKind::Text,
            delta: delta.into(),
        }
    }

    pub fn reasoning_delta(delta: impl Into<String>) -> Self {
        Self {
            kind: ModelStreamEventKind::Reasoning,
            delta: delta.into(),
        }
    }

    pub fn reasoning_summary_delta(delta: impl Into<String>) -> Self {
        Self {
            kind: ModelStreamEventKind::ReasoningSummary,
            delta: delta.into(),
        }
    }
}

impl ModelResponse {
    pub fn final_output(output: impl Into<String>) -> Self {
        Self::final_output_with_usage(output, None)
    }

    pub fn final_output_with_usage(
        output: impl Into<String>,
        token_usage: Option<ModelTokenUsage>,
    ) -> Self {
        Self::final_output_with_usage_and_session(output, token_usage, None)
    }

    pub fn final_output_with_usage_and_session(
        output: impl Into<String>,
        token_usage: Option<ModelTokenUsage>,
        system_agent_session_id: Option<String>,
    ) -> Self {
        Self::FinalOutput {
            output: output.into(),
            token_usage,
            system_agent_session_id,
        }
    }

    pub fn tool_call(tool_name: impl Into<String>, input: Value) -> Self {
        Self::tool_call_with_usage(tool_name, input, None)
    }

    pub fn tool_call_with_usage(
        tool_name: impl Into<String>,
        input: Value,
        token_usage: Option<ModelTokenUsage>,
    ) -> Self {
        Self::ToolCall {
            call: ToolCall {
                tool_name: tool_name.into(),
                input,
            },
            token_usage,
        }
    }

    pub fn token_usage(&self) -> Option<ModelTokenUsage> {
        match self {
            Self::FinalOutput { token_usage, .. } | Self::ToolCall { token_usage, .. } => {
                *token_usage
            }
        }
    }
}

impl ModelTokenUsage {
    pub fn add(self, other: Self) -> Self {
        Self {
            input_tokens: self.input_tokens.saturating_add(other.input_tokens),
            output_tokens: self.output_tokens.saturating_add(other.output_tokens),
            total_tokens: self.total_tokens.saturating_add(other.total_tokens),
        }
    }
}
