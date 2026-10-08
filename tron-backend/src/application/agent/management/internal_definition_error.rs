use crate::domain::agent::AgentManagementError;

pub struct ApplicationErrorDescriptor {
    pub code: &'static str,
    pub message_key: &'static str,
}

impl AgentManagementError {
    pub fn internal_definition_descriptor(&self) -> ApplicationErrorDescriptor {
        let (code, message_key) = match self {
            Self::LoopInvalid => ("101028011", "tron.agent.loop_invalid"),
            Self::LoopNotFound => ("101028012", "tron.agent.loop_not_found"),
            Self::LoopConflict => ("101028013", "tron.agent.loop_conflict"),
            Self::LoopInUse => ("101028014", "tron.agent.loop_in_use"),
            Self::InternalAgentDefinitionInvalid => ("101028001", "tron.agent.definition_invalid"),
            Self::InternalAgentCodeInvalid => ("101028009", "tron.agent.code_invalid"),
            Self::InternalAgentCodeConflict => ("101028010", "tron.agent.code_conflict"),
            Self::InternalAgentNotFound => ("101028002", "tron.agent.internal_not_found"),
            Self::InvalidWorkspace => ("101028003", "tron.agent.workspace_invalid"),
            Self::InternalAgentPolicyInvalid => ("101028005", "tron.agent.policy_invalid"),
            Self::InternalToolUnknown(_) => ("101028006", "tron.agent.tool_unknown"),
            Self::InternalToolAssignmentStale(_) => ("101028007", "tron.agent.tool_unavailable"),
            Self::InternalMcpServerAssignmentStale(_) => {
                ("101028008", "tron.agent.mcp_unavailable")
            }
            _ => ("101028004", "tron.agent.storage_failed"),
        };
        ApplicationErrorDescriptor { code, message_key }
    }
}
