use std::collections::{BTreeMap, HashMap};

use serde_json::Value;

use crate::domain::mcp::{
    McpError, McpHealthStatus, McpServerAuthKind, McpServerTransportKind, PlatformMcpServerRecord,
    PlatformMcpServerStatusFilter, WorkspaceEffectiveMcpServerRecord, validate_mcp_server_url,
};
use crate::domain::variable::{VariableError, validate_variable_key};
use crate::domain::workspace::WorkspaceError;

#[derive(Debug, Clone)]
pub(super) struct ValidatedTransportDefinition {
    pub transport_kind: McpServerTransportKind,
    pub url: String,
    pub auth_kind: McpServerAuthKind,
    pub auth_payload: Option<Value>,
    pub headers: BTreeMap<String, String>,
    pub headers_variable_refs: BTreeMap<String, String>,
    pub command: String,
    pub args: Vec<String>,
    pub env_variable_refs: BTreeMap<String, String>,
    pub env_passthrough_refs: Vec<String>,
    pub working_directory: Option<String>,
}

pub(super) fn validate_transport_definition(
    transport_kind: McpServerTransportKind,
    url: String,
    auth_kind: McpServerAuthKind,
    auth_payload: Option<Value>,
    headers: BTreeMap<String, String>,
    headers_variable_refs: BTreeMap<String, String>,
    command: String,
    args: Vec<String>,
    env_variable_refs: BTreeMap<String, String>,
    env_passthrough_refs: Vec<String>,
    working_directory: Option<String>,
) -> Result<ValidatedTransportDefinition, McpError> {
    match transport_kind {
        McpServerTransportKind::StreamableHttp => {
            if !command.trim().is_empty()
                || !args.is_empty()
                || !env_variable_refs.is_empty()
                || !env_passthrough_refs.is_empty()
                || working_directory
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .is_some()
            {
                return Err(McpError::InvalidServerDefinition);
            }

            Ok(ValidatedTransportDefinition {
                transport_kind,
                url: validate_mcp_server_url(&url)?,
                auth_kind,
                auth_payload: validate_auth_payload(auth_kind, auth_payload)?,
                headers: normalize_string_map(headers)?,
                headers_variable_refs: normalize_reference_map(headers_variable_refs)?,
                command: String::new(),
                args: Vec::new(),
                env_variable_refs: BTreeMap::new(),
                env_passthrough_refs: Vec::new(),
                working_directory: None,
            })
        }
        McpServerTransportKind::Stdio => {
            let normalized_command = command.trim().to_string();
            if normalized_command.is_empty()
                || !url.trim().is_empty()
                || auth_kind != McpServerAuthKind::None
                || auth_payload.is_some()
                || !headers.is_empty()
                || !headers_variable_refs.is_empty()
            {
                return Err(McpError::InvalidServerDefinition);
            }
            Ok(ValidatedTransportDefinition {
                transport_kind,
                url: String::new(),
                auth_kind: McpServerAuthKind::None,
                auth_payload: None,
                headers: BTreeMap::new(),
                headers_variable_refs: BTreeMap::new(),
                command: normalized_command,
                args: normalize_trimmed_list(args)?,
                env_variable_refs: normalize_reference_map(env_variable_refs)?,
                env_passthrough_refs: normalize_trimmed_list(env_passthrough_refs)?,
                working_directory: optional_trimmed(working_directory),
            })
        }
    }
}

pub(super) fn normalized_search(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_lowercase())
        .filter(|item| !item.is_empty())
}

pub(super) fn optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

pub(super) fn matches_platform_status(
    record: &PlatformMcpServerRecord,
    status: Option<PlatformMcpServerStatusFilter>,
) -> bool {
    match status {
        None => true,
        Some(PlatformMcpServerStatusFilter::Enabled) => record.enabled,
        Some(PlatformMcpServerStatusFilter::Disabled) => !record.enabled,
        Some(PlatformMcpServerStatusFilter::Healthy) => {
            record.last_health_status == McpHealthStatus::Healthy
        }
        Some(PlatformMcpServerStatusFilter::Unhealthy) => {
            record.last_health_status == McpHealthStatus::Unhealthy
        }
        Some(PlatformMcpServerStatusFilter::NeverChecked) => {
            record.last_health_status == McpHealthStatus::NeverChecked
        }
    }
}

pub(super) fn matches_platform_search(record: &PlatformMcpServerRecord, search: &str) -> bool {
    let transport_label = match record.transport_kind {
        McpServerTransportKind::StreamableHttp => "streamable_http",
        McpServerTransportKind::Stdio => "stdio",
    };
    [
        record.server_code.as_str(),
        record.name.as_str(),
        record.description.as_deref().unwrap_or(""),
        record.url.as_str(),
        record.command.as_str(),
        transport_label,
    ]
    .join(" ")
    .to_lowercase()
    .contains(search)
}

pub(super) fn matches_workspace_search(
    record: &WorkspaceEffectiveMcpServerRecord,
    search: &str,
) -> bool {
    [
        record.server_code.as_str(),
        record.name.as_str(),
        record.description.as_deref().unwrap_or(""),
    ]
    .join(" ")
    .to_lowercase()
    .contains(search)
}

pub(super) fn needs_variable_resolution(server: &PlatformMcpServerRecord) -> bool {
    !server.headers_variable_refs.is_empty()
        || !server.env_variable_refs.is_empty()
        || server
            .auth_payload
            .as_ref()
            .and_then(|payload| payload.get("tokenRef"))
            .and_then(Value::as_str)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
}

pub(super) fn resolve_http_headers(
    server: &PlatformMcpServerRecord,
    variable_values: Option<&HashMap<String, String>>,
) -> Result<BTreeMap<String, String>, McpError> {
    let mut headers = server.headers.clone();
    for (header_name, variable_ref) in &server.headers_variable_refs {
        let value = resolve_variable_value(variable_values, variable_ref, &server.server_code)?;
        headers.insert(header_name.clone(), value);
    }
    match server.auth_kind {
        McpServerAuthKind::None => {}
        McpServerAuthKind::BearerStatic => {
            let token_ref = server
                .auth_payload
                .as_ref()
                .and_then(|payload| payload.get("tokenRef"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or(McpError::InvalidServerDefinition)?;
            let token = resolve_variable_value(variable_values, token_ref, &server.server_code)?;
            headers.insert("Authorization".to_string(), format!("Bearer {token}"));
        }
    }
    Ok(headers)
}

pub(super) fn resolve_stdio_env(
    server: &PlatformMcpServerRecord,
    variable_values: Option<&HashMap<String, String>>,
) -> Result<BTreeMap<String, String>, McpError> {
    if server.transport_kind != McpServerTransportKind::Stdio {
        return Err(McpError::InvalidServerDefinition);
    }

    let mut env = BTreeMap::new();
    for (env_name, variable_ref) in &server.env_variable_refs {
        let value = resolve_variable_value(variable_values, variable_ref, &server.server_code)?;
        env.insert(env_name.clone(), value);
    }
    for passthrough_name in &server.env_passthrough_refs {
        if let Ok(value) = std::env::var(passthrough_name) {
            env.insert(passthrough_name.clone(), value);
        }
    }
    Ok(env)
}

pub(super) fn resolve_variable_value(
    variable_values: Option<&HashMap<String, String>>,
    variable_ref: &str,
    server_code: &str,
) -> Result<String, McpError> {
    let Some(variable_values) = variable_values else {
        return Err(McpError::VerifyContextRequired);
    };
    variable_values
        .get(variable_ref)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            McpError::VerificationFailed(format!(
                "MCP server `{server_code}` requires variable `{variable_ref}`"
            ))
        })
}

pub(super) fn map_workspace_error(error: WorkspaceError) -> McpError {
    match error {
        WorkspaceError::StorageFailure => McpError::StorageFailure,
        WorkspaceError::NotFound => McpError::InvalidWorkspace,
        _ => McpError::InvalidWorkspace,
    }
}

pub(super) fn map_variable_error(error: VariableError) -> McpError {
    match error {
        VariableError::WorkspaceRequired => McpError::VerifyContextRequired,
        VariableError::InvalidWorkspace => McpError::VerifyContextRequired,
        VariableError::StorageFailure => McpError::StorageFailure,
        _ => McpError::VerifyContextRequired,
    }
}

fn normalize_string_map(
    values: BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, McpError> {
    let mut normalized = BTreeMap::new();
    for (key, value) in values {
        let normalized_key = key.trim().to_string();
        if normalized_key.is_empty() {
            return Err(McpError::InvalidServerDefinition);
        }
        normalized.insert(normalized_key, value.trim().to_string());
    }
    Ok(normalized)
}

fn normalize_reference_map(
    values: BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, McpError> {
    let mut normalized = BTreeMap::new();
    for (key, value) in values {
        let normalized_key = key.trim().to_string();
        let normalized_value = value.trim().to_string();
        if normalized_key.is_empty() || normalized_value.is_empty() {
            return Err(McpError::InvalidServerDefinition);
        }
        validate_variable_key(&normalized_value).map_err(|_| McpError::InvalidServerDefinition)?;
        normalized.insert(normalized_key, normalized_value);
    }
    Ok(normalized)
}

fn normalize_trimmed_list(values: Vec<String>) -> Result<Vec<String>, McpError> {
    let mut normalized = Vec::new();
    for value in values {
        let normalized_value = value.trim().to_string();
        if normalized_value.is_empty() {
            return Err(McpError::InvalidServerDefinition);
        }
        if !normalized
            .iter()
            .any(|item: &String| item == &normalized_value)
        {
            normalized.push(normalized_value);
        }
    }
    Ok(normalized)
}

fn validate_auth_payload(
    auth_kind: McpServerAuthKind,
    auth_payload: Option<Value>,
) -> Result<Option<Value>, McpError> {
    match auth_kind {
        McpServerAuthKind::None => {
            if auth_payload.is_some() {
                Err(McpError::InvalidServerDefinition)
            } else {
                Ok(None)
            }
        }
        McpServerAuthKind::BearerStatic => {
            let payload = auth_payload.ok_or(McpError::InvalidServerDefinition)?;
            let token_ref = payload
                .get("tokenRef")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or(McpError::InvalidServerDefinition)?;
            validate_variable_key(token_ref).map_err(|_| McpError::InvalidServerDefinition)?;
            if payload.get("tokenRef").is_some() {
                Ok(Some(payload))
            } else {
                Err(McpError::InvalidServerDefinition)
            }
        }
    }
}
