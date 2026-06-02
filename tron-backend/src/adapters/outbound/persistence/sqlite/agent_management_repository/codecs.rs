use std::{fs, path::Path};

use rusqlite::Connection;

use crate::domain::agent::{
    AgentManagementError, AgentRuntimeType, ExpandedDomainPersistenceMode, ToolAssignmentMode,
    ToolExposureMode,
};

pub(super) fn ensure_database_parent(database_url: &str) -> Result<(), AgentManagementError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| AgentManagementError::StorageFailure)?;
    }
    Ok(())
}

pub(super) fn column_exists(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
) -> Result<bool, AgentManagementError> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut statement = connection.prepare(&pragma).map_err(storage_error)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(columns.iter().any(|value| value == column_name))
}

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
