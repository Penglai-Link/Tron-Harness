use crate::domain::variable::VariableError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentInvocationError {
    PromptRequired,
    WorkspaceRequired,
    WorkspaceInvalid,
    ModelRequired,
    HistoryStorageFailed,
    RunStopped,
    SessionBusy(String),
    RuntimeFailed(String),
    RuntimeSecretDeliveryUntrusted,
    RuntimeVariableResolutionFailed(VariableError),
    ModelFailed(String),
    ToolFailed(String),
    EmptyFinalOutput(String),
    MaxStepsExceeded,
    MaxToolCallsExceeded,
}

impl AgentInvocationError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::PromptRequired => "AGENT_PROMPT_REQUIRED",
            Self::WorkspaceRequired => "AGENT_WORKSPACE_REQUIRED",
            Self::WorkspaceInvalid => "AGENT_WORKSPACE_INVALID",
            Self::ModelRequired => "AGENT_MODEL_REQUIRED",
            Self::HistoryStorageFailed => "AGENT_HISTORY_STORAGE_FAILED",
            Self::RunStopped => "AGENT_RUN_STOPPED",
            Self::SessionBusy(_) => "RUN_ALREADY_ACTIVE",
            Self::RuntimeFailed(_) => "AGENT_RUNTIME_FAILED",
            Self::RuntimeSecretDeliveryUntrusted => "VARIABLE_RUNTIME_SECRET_DELIVERY_UNTRUSTED",
            Self::RuntimeVariableResolutionFailed(error) => match error {
                VariableError::DecryptFailed => "VARIABLE_RUNTIME_DECRYPT_FAILED",
                VariableError::EncryptionNotConfigured => {
                    "VARIABLE_RUNTIME_ENCRYPTION_NOT_CONFIGURED"
                }
                VariableError::StorageFailure => "VARIABLE_RUNTIME_STORAGE_FAILED",
                _ => "VARIABLE_RUNTIME_RESOLUTION_FAILED",
            },
            Self::ModelFailed(_) => "AGENT_MODEL_FAILED",
            Self::ToolFailed(_) => "AGENT_TOOL_FAILED",
            Self::EmptyFinalOutput(_) => "AGENT_EMPTY_FINAL_OUTPUT",
            Self::MaxStepsExceeded => "AGENT_MAX_STEPS_EXCEEDED",
            Self::MaxToolCallsExceeded => "AGENT_MAX_TOOL_CALLS_EXCEEDED",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::PromptRequired => "prompt is required".into(),
            Self::WorkspaceRequired => "workspaceCode is required".into(),
            Self::WorkspaceInvalid => "workspace must reference an active workspace".into(),
            Self::ModelRequired => "model config is required".into(),
            Self::HistoryStorageFailed => "agent run history storage failed".into(),
            Self::RunStopped => "agent run was stopped".into(),
            Self::SessionBusy(_) => "session already has an active agent run".into(),
            Self::RuntimeFailed(message)
            | Self::ModelFailed(message)
            | Self::ToolFailed(message)
            | Self::EmptyFinalOutput(message) => message.clone(),
            Self::RuntimeSecretDeliveryUntrusted => {
                "Sensitive runtime variable delivery is not trusted".into()
            }
            Self::RuntimeVariableResolutionFailed(error) => {
                format!("runtime variables could not be resolved: {error}")
            }
            Self::MaxStepsExceeded => "agent run exceeded max steps".into(),
            Self::MaxToolCallsExceeded => "agent run exceeded max tool calls".into(),
        }
    }
}
