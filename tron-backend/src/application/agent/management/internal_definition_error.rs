use crate::domain::agent::AgentManagementError;

pub struct ApplicationErrorDescriptor {
    pub code: &'static str,
    pub message_key: &'static str,
}

impl AgentManagementError {
    pub fn internal_definition_descriptor(&self) -> ApplicationErrorDescriptor {
        let (code, message_key) = match self {
            Self::BuiltinAgentReadOnly => ("101028017", "tron.agent.builtin_read_only"),
            Self::InternalAgentGroupInvalid => ("101028016", "tron.agent.group_invalid"),
            Self::PlatformAgentReadOnly => ("101028018", "tron.agent.platform_read_only"),
            Self::ToolPermissionInvalid => ("101028015", "tron.agent.tool_permission_invalid"),
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
