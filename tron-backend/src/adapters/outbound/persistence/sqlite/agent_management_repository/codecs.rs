

use crate::domain::agent::{
    AgentManagementError, AgentRuntimeType, ExpandedDomainPersistenceMode, McpAccessMode,
    ToolAssignmentMode, ToolExposureMode,
};

pub(super) fn storage_error(_error: rusqlite::Error) -> AgentManagementError {
    AgentManagementError::StorageFailure
}

pub(super) fn runtime_type_to_storage(runtime_type: AgentRuntimeType) -> &'static str {
    match runtime_type {
        AgentRuntimeType::React => "REACT",
        AgentRuntimeType::CodexExec => "CODEX_EXEC",
        AgentRuntimeType::CodexAppServer => "CODEX_APP_SERVER",
        AgentRuntimeType::OpenCodeRun => "OPENCODE_RUN",
        AgentRuntimeType::OpenCodeServe => "OPENCODE_SERVE",
        AgentRuntimeType::OpenCodeAcp => "OPENCODE_ACP",
    }
}

pub(super) fn runtime_type_from_storage(value: &str) -> AgentRuntimeType {
    match value.trim().to_ascii_uppercase().as_str() {
        "REACT" => AgentRuntimeType::React,
        "CODEX_EXEC" => AgentRuntimeType::CodexExec,
        "CODEX_APP_SERVER" => AgentRuntimeType::CodexAppServer,
        "OPENCODE_RUN" => AgentRuntimeType::OpenCodeRun,
        "OPENCODE_SERVE" => AgentRuntimeType::OpenCodeServe,
        "OPENCODE_ACP" => AgentRuntimeType::OpenCodeAcp,
        _ => AgentRuntimeType::React,
    }
}

pub(super) fn tool_assignment_mode_to_storage(value: ToolAssignmentMode) -> &'static str {
    match value {
        ToolAssignmentMode::AllPlatformTools => "ALL_PLATFORM_TOOLS",
        ToolAssignmentMode::SelectedTools => "SELECTED_TOOLS",
    }
}

pub(super) fn tool_assignment_mode_from_storage(value: String) -> ToolAssignmentMode {
    match value.trim().to_ascii_uppercase().as_str() {
        "SELECTED_TOOLS" => ToolAssignmentMode::SelectedTools,
        _ => ToolAssignmentMode::AllPlatformTools,
    }
}

pub(super) fn tool_exposure_mode_to_storage(value: ToolExposureMode) -> &'static str {
    match value {
        ToolExposureMode::DirectList => "DIRECT_LIST",
        ToolExposureMode::DomainDiscovery => "DOMAIN_DISCOVERY",
    }
}

pub(super) fn tool_exposure_mode_from_storage(value: String) -> ToolExposureMode {
    match value.trim().to_ascii_uppercase().as_str() {
        "DOMAIN_DISCOVERY" => ToolExposureMode::DomainDiscovery,
        _ => ToolExposureMode::DirectList,
    }
}

pub(super) fn expanded_domain_persistence_mode_to_storage(
    value: ExpandedDomainPersistenceMode,
) -> &'static str {
    match value {
        ExpandedDomainPersistenceMode::StickyExpanded => "STICKY_EXPANDED",
        ExpandedDomainPersistenceMode::EphemeralExpanded => "EPHEMERAL_EXPANDED",
    }
}

pub(super) fn expanded_domain_persistence_mode_from_storage(
    value: String,
) -> ExpandedDomainPersistenceMode {
    match value.trim().to_ascii_uppercase().as_str() {
        "EPHEMERAL_EXPANDED" => ExpandedDomainPersistenceMode::EphemeralExpanded,
        _ => ExpandedDomainPersistenceMode::StickyExpanded,
    }
}

pub(super) fn mcp_access_mode_to_storage(value: McpAccessMode) -> &'static str {
    match value {
        McpAccessMode::Disabled => "DISABLED",
        McpAccessMode::AllEnabledServers => "ALL_ENABLED_SERVERS",
        McpAccessMode::SelectedServers => "SELECTED_SERVERS",
    }
}

pub(super) fn mcp_access_mode_from_storage(value: String) -> McpAccessMode {
    match value.trim().to_ascii_uppercase().as_str() {
        "ALL_ENABLED_SERVERS" => McpAccessMode::AllEnabledServers,
        "SELECTED_SERVERS" => McpAccessMode::SelectedServers,
        _ => McpAccessMode::Disabled,
    }
}
