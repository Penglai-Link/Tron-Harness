use crate::{
    application::agent::{AgentInvocationError, ResolvedAgentRuntime},
    domain::{
        agent::{AgentKind, AgentManagementError, AgentRuntimeType},
        mcp::McpError,
    },
};

const MAIN_AGENT_CODE: &str = "main-agent";

pub(super) fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn is_main_agent(agent_code: &str) -> bool {
    agent_code.eq_ignore_ascii_case(MAIN_AGENT_CODE)
}

pub(super) fn agent_kind_for_runtime(runtime_type: AgentRuntimeType) -> AgentKind {
    match runtime_type {
        AgentRuntimeType::React => AgentKind::React,
        AgentRuntimeType::CodexExec
        | AgentRuntimeType::CodexAppServer
        | AgentRuntimeType::OpenCodeRun
        | AgentRuntimeType::OpenCodeServe
        | AgentRuntimeType::OpenCodeAcp => AgentKind::SystemCli,
    }
}

pub(super) fn required_runtime_command_name(
    runtime: &ResolvedAgentRuntime,
) -> Result<String, AgentInvocationError> {
    runtime
        .command_name
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AgentInvocationError::RuntimeFailed(format!(
                "agent `{}` runtime command is empty",
                runtime.agent_code
            ))
        })
}

pub(super) fn runtime_type_label(runtime_type: AgentRuntimeType) -> &'static str {
    match runtime_type {
        AgentRuntimeType::React => "REACT",
        AgentRuntimeType::CodexExec => "CODEX_EXEC",
        AgentRuntimeType::CodexAppServer => "CODEX_APP_SERVER",
        AgentRuntimeType::OpenCodeRun => "OPENCODE_RUN",
        AgentRuntimeType::OpenCodeServe => "OPENCODE_SERVE",
        AgentRuntimeType::OpenCodeAcp => "OPENCODE_ACP",
    }
}

pub(super) fn safe_preview(label: &str, value: &str) -> String {
    format!("{label}_chars:{}", value.chars().count())
}

pub(super) fn map_agent_error(
    agent_code: &str,
    error: AgentManagementError,
) -> AgentInvocationError {
    match error {
        AgentManagementError::InvalidWorkspace => AgentInvocationError::WorkspaceInvalid,
        AgentManagementError::InternalAgentNotFound => AgentInvocationError::RuntimeFailed(
            format!("internal agent `{agent_code}` was not found"),
        ),
        AgentManagementError::InternalToolAssignmentStale(message)
        | AgentManagementError::InternalMcpServerAssignmentStale(message)
        | AgentManagementError::InternalToolUnknown(message) => {
            AgentInvocationError::RuntimeFailed(format!(
                "internal agent policy is invalid: {message}"
            ))
        }
        AgentManagementError::SystemAgentNotFound => {
            AgentInvocationError::RuntimeFailed(format!("agent `{agent_code}` was not found"))
        }
        AgentManagementError::SystemAgentUnavailable => AgentInvocationError::RuntimeFailed(
            format!("agent `{agent_code}` is unavailable or disabled for this workspace"),
        ),
        AgentManagementError::StorageFailure => {
            AgentInvocationError::RuntimeFailed("agent configuration load failed".into())
        }
    }
}

pub(super) fn map_mcp_error(error: McpError) -> AgentInvocationError {
    match error {
        McpError::InvalidWorkspace
        | McpError::WorkspaceRequired
        | McpError::VerifyContextRequired => AgentInvocationError::WorkspaceInvalid,
        McpError::RuntimeDiscoveryFailed(message) | McpError::VerificationFailed(message) => {
            AgentInvocationError::RuntimeFailed(message)
        }
        other => {
            AgentInvocationError::RuntimeFailed(format!("MCP runtime preparation failed: {other}"))
        }
    }
}
