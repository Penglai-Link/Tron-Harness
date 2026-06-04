use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use super::ToolError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentError {
    InvalidConfiguration(String),
    ModelFailure(String),
    MissingTool(String),
    UnauthorizedTool(String),
    ToolFailure(String),
    MaxStepsExceeded,
    MaxToolCallsExceeded,
    EventPublishFailed(String),
}

impl Display for AgentError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(message) => f.write_str(message),
            Self::ModelFailure(message) => write!(f, "model failure: {message}"),
            Self::MissingTool(tool) => write!(f, "missing tool: {tool}"),
            Self::UnauthorizedTool(tool) => write!(f, "unauthorized tool: {tool}"),
            Self::ToolFailure(message) => write!(f, "tool failure: {message}"),
            Self::MaxStepsExceeded => f.write_str("agent run exceeded max steps"),
            Self::MaxToolCallsExceeded => f.write_str("agent run exceeded max tool calls"),
            Self::EventPublishFailed(message) => write!(f, "event publish failed: {message}"),
        }
    }
}

impl Error for AgentError {}

impl From<ToolError> for AgentError {
    fn from(value: ToolError) -> Self {
        match value {
            ToolError::MissingTool(tool) => Self::MissingTool(tool),
            ToolError::UnauthorizedTool(tool) => Self::UnauthorizedTool(tool),
            ToolError::ExecutionFailed(message) => Self::ToolFailure(message),
        }
    }
}
