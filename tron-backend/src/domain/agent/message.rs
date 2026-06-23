use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<AgentMessageToolCall>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessageToolCall {
    pub call_id: String,
    pub tool_name: String,
    pub input: Value,
}

impl AgentMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
            tool_call_id: None,
            tool_output: None,
            tool_calls: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
            tool_call_id: None,
            tool_output: None,
            tool_calls: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
            tool_call_id: None,
            tool_output: None,
            tool_calls: None,
        }
    }

    pub fn assistant_tool_calls(tool_calls: Vec<AgentMessageToolCall>) -> Self {
        Self {
            role: "assistant".into(),
            content: String::new(),
            tool_call_id: None,
            tool_output: None,
            tool_calls: Some(tool_calls),
        }
    }

    pub fn tool(content: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: content.into(),
            tool_call_id: None,
            tool_output: None,
            tool_calls: None,
        }
    }

    pub fn tool_with_call_id(
        content: impl Into<String>,
        tool_call_id: impl Into<String>,
        tool_output: impl Into<String>,
    ) -> Self {
        Self {
            role: "tool".into(),
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
            tool_output: Some(tool_output.into()),
            tool_calls: None,
        }
    }
}
