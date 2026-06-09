use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};

use reqwest::Client;
use serde_json::Value;

use crate::{
    application::workspace::WorkspaceService,
    domain::{
        mcp::{
            CreatePlatformMcpServerCommand, McpDeleteResult, McpError, McpHealthStatus,
            McpServerAuthKind, McpTransportAvailability, PlatformMcpGatewayConfig,
            PlatformMcpServerListQuery, PlatformMcpServerRecord, PlatformMcpServerStatusFilter,
            UpdatePlatformMcpGatewayCommand, UpdatePlatformMcpServerCommand,
            WorkspaceEffectiveMcpServerRecord, WorkspaceMcpServerListQuery,
            WorkspaceMcpServerPolicy, WorkspaceMcpServerSource, normalize_mcp_server_code,
            validate_mcp_server_name, validate_mcp_server_url,
        },
        workspace::{WorkspaceDetail, WorkspaceError, WorkspaceStatus},
    },
    shared::time::current_timestamp,
};

pub trait McpRepository: Send {
    fn get_gateway_config(&mut self) -> Result<Option<PlatformMcpGatewayConfig>, McpError>;
    fn upsert_gateway_config(&mut self, config: &PlatformMcpGatewayConfig) -> Result<(), McpError>;
    fn list_platform_servers(&mut self) -> Result<Vec<PlatformMcpServerRecord>, McpError>;
    fn get_platform_server(
        &mut self,
        server_code: &str,
    ) -> Result<Option<PlatformMcpServerRecord>, McpError>;
    fn upsert_platform_server(&mut self, server: &PlatformMcpServerRecord) -> Result<(), McpError>;
    fn delete_platform_server(&mut self, server_code: &str) -> Result<bool, McpError>;
    fn list_workspace_server_policies(
        &mut self,
        workspace_code: Option<&str>,
    ) -> Result<Vec<WorkspaceMcpServerPolicy>, McpError>;
    fn upsert_workspace_server_policy(
        &mut self,
        policy: &WorkspaceMcpServerPolicy,
    ) -> Result<(), McpError>;
    fn delete_workspace_server_policy(
        &mut self,
        workspace_code: &str,
        server_code: &str,
    ) -> Result<bool, McpError>;
}

#[derive(Clone)]
pub struct McpService {
    repository: Arc<Mutex<Box<dyn McpRepository>>>,
    workspaces: WorkspaceService,
    http_client: Client,
}

impl McpService {
    pub fn new(
        repository: impl McpRepository + 'static,
        workspaces: WorkspaceService,
        http_client: Client,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            workspaces,
            http_client,
        }
    }

    pub fn get_gateway_config(&self) -> Result<PlatformMcpGatewayConfig, McpError> {
        let mut repository = self
            .repository
            .lock()
            .expect("MCP repository mutex poisoned");
        if let Some(config) = repository.get_gateway_config()? {
            return Ok(config);
        }

        let now = current_timestamp();
        let config = PlatformMcpGatewayConfig {
            gateway_enabled: false,
            variables_endpoint_enabled: false,
            created_at: Some(now.clone()),
            updated_at: Some(now),
        };
        repository.upsert_gateway_config(&config)?;
        Ok(config)
    }

    pub fn update_gateway_config(
        &self,
        command: UpdatePlatformMcpGatewayCommand,
    ) -> Result<PlatformMcpGatewayConfig, McpError> {
        let current = self.get_gateway_config()?;
        let now = current_timestamp();
        let updated = PlatformMcpGatewayConfig {
            gateway_enabled: command.gateway_enabled,
            variables_endpoint_enabled: command.variables_endpoint_enabled,
            created_at: current.created_at,
            updated_at: Some(now),
        };
        self.repository
            .lock()
            .expect("MCP repository mutex poisoned")
            .upsert_gateway_config(&updated)?;
        Ok(updated)
    }

    pub fn list_platform_servers(
        &self,
        query: PlatformMcpServerListQuery,
    ) -> Result<Vec<PlatformMcpServerRecord>, McpError> {
        let search = normalized_search(query.search);
        let status = query.status;
        let mut records = self
            .repository
            .lock()
            .expect("MCP repository mutex poisoned")
            .list_platform_servers()?;
        records.retain(|record| {
            matches_platform_status(record, status)
                && search
                    .as_deref()
                    .map(|term| matches_platform_search(record, term))
                    .unwrap_or(true)
        });
        Ok(records)
    }

    pub fn get_platform_server(
        &self,
        server_code: &str,
    ) -> Result<PlatformMcpServerRecord, McpError> {
        let normalized_server_code = normalize_mcp_server_code(server_code)?;
        self.repository
            .lock()
            .expect("MCP repository mutex poisoned")
            .get_platform_server(&normalized_server_code)?
            .ok_or(McpError::ServerNotFound)
    }

    pub fn create_platform_server(
        &self,
        command: CreatePlatformMcpServerCommand,
    ) -> Result<PlatformMcpServerRecord, McpError> {
        let server_code = normalize_mcp_server_code(&command.server_code)?;
        let mut repository = self
            .repository
            .lock()
            .expect("MCP repository mutex poisoned");
        if repository.get_platform_server(&server_code)?.is_some() {
            return Err(McpError::DuplicateServerCode);
        }

        let now = current_timestamp();
        let record = PlatformMcpServerRecord {
            server_code,
            name: validate_mcp_server_name(&command.name)?,
            description: optional_trimmed(command.description),
            url: validate_mcp_server_url(&command.url)?,
            auth_kind: command.auth_kind,
            auth_payload: validate_auth_payload(command.auth_kind, command.auth_payload)?,
            headers: normalize_headers(command.headers)?,
            enabled: true,
            last_health_status: McpHealthStatus::NeverChecked,
            last_health_message: None,
            last_health_checked_at: None,
            created_at: now.clone(),
            updated_at: now,
        };
        repository.upsert_platform_server(&record)?;
        Ok(record)
    }

    pub fn update_platform_server(
        &self,
        server_code: &str,
        command: UpdatePlatformMcpServerCommand,
    ) -> Result<PlatformMcpServerRecord, McpError> {
        let normalized_server_code = normalize_mcp_server_code(server_code)?;
        let mut repository = self
            .repository
            .lock()
            .expect("MCP repository mutex poisoned");
        let current = repository
            .get_platform_server(&normalized_server_code)?
            .ok_or(McpError::ServerNotFound)?;
        let updated = PlatformMcpServerRecord {
            server_code: current.server_code,
            name: validate_mcp_server_name(&command.name)?,
            description: optional_trimmed(command.description),
            url: validate_mcp_server_url(&command.url)?,
            auth_kind: command.auth_kind,
            auth_payload: validate_auth_payload(command.auth_kind, command.auth_payload)?,
            headers: normalize_headers(command.headers)?,
            enabled: current.enabled,
            last_health_status: current.last_health_status,
            last_health_message: current.last_health_message,
            last_health_checked_at: current.last_health_checked_at,
            created_at: current.created_at,
            updated_at: current_timestamp(),
        };
        repository.upsert_platform_server(&updated)?;
        Ok(updated)
    }

    pub fn enable_platform_server(
        &self,
        server_code: &str,
    ) -> Result<PlatformMcpServerRecord, McpError> {
        self.set_platform_server_enabled(server_code, true)
    }

    pub fn disable_platform_server(
        &self,
        server_code: &str,
    ) -> Result<PlatformMcpServerRecord, McpError> {
        self.set_platform_server_enabled(server_code, false)
    }

    pub async fn refresh_platform_server_health(
        &self,
        server_code: &str,
    ) -> Result<PlatformMcpServerRecord, McpError> {
        let normalized_server_code = normalize_mcp_server_code(server_code)?;
        let current = self
            .repository
            .lock()
            .expect("MCP repository mutex poisoned")
            .get_platform_server(&normalized_server_code)?
            .ok_or(McpError::ServerNotFound)?;

        let checked_at = current_timestamp();
        let outcome = self.http_client.get(current.url.clone()).send().await;
        let (status, message) = match outcome {
            Ok(response) if response.status().is_success() => (McpHealthStatus::Healthy, None),
            Ok(response) => (
                McpHealthStatus::Unhealthy,
                Some(format!("HTTP {}", response.status().as_u16())),
            ),
            Err(error) => (McpHealthStatus::Unhealthy, Some(error.to_string())),
        };

        let updated = PlatformMcpServerRecord {
            last_health_status: status,
            last_health_message: message.clone(),
            last_health_checked_at: Some(checked_at),
            updated_at: current_timestamp(),
            ..current
        };
        self.repository
            .lock()
            .expect("MCP repository mutex poisoned")
            .upsert_platform_server(&updated)?;

        if updated.last_health_status == McpHealthStatus::Healthy {
            Ok(updated)
        } else {
            Err(McpError::HealthCheckFailed(message.unwrap_or_else(|| {
                "unknown health-check failure".to_string()
            })))
        }
    }

    pub fn delete_platform_server(&self, server_code: &str) -> Result<McpDeleteResult, McpError> {
        let normalized_server_code = normalize_mcp_server_code(server_code)?;
        let deleted = self
            .repository
            .lock()
            .expect("MCP repository mutex poisoned")
            .delete_platform_server(&normalized_server_code)?;
        if !deleted {
            return Err(McpError::ServerNotFound);
        }
        Ok(McpDeleteResult {
            server_code: normalized_server_code,
            deleted,
        })
    }

    pub fn list_workspace_effective_servers(
        &self,
        workspace_code: &str,
        query: WorkspaceMcpServerListQuery,
    ) -> Result<Vec<WorkspaceEffectiveMcpServerRecord>, McpError> {
        self.ensure_active_workspace(workspace_code)?;
        let search = normalized_search(query.search);
        let records = self.resolve_workspace_effective_servers(workspace_code)?;
        Ok(records
            .into_iter()
            .filter(|record| {
                search
                    .as_deref()
                    .map(|term| matches_workspace_search(record, term))
                    .unwrap_or(true)
            })
            .collect())
    }

    pub fn disable_workspace_server(
        &self,
        workspace_code: &str,
        server_code: &str,
    ) -> Result<WorkspaceEffectiveMcpServerRecord, McpError> {
        self.ensure_active_workspace(workspace_code)?;
        let normalized_server_code = normalize_mcp_server_code(server_code)?;
        let current = self.get_platform_server(&normalized_server_code)?;
        if current.enabled {
            let now = current_timestamp();
            let policy = WorkspaceMcpServerPolicy {
                workspace_code: workspace_code.trim().to_string(),
                server_code: normalized_server_code.clone(),
                enabled: false,
                created_at: now.clone(),
                updated_at: now,
            };
            self.repository
                .lock()
                .expect("MCP repository mutex poisoned")
                .upsert_workspace_server_policy(&policy)?;
        }
        self.workspace_effective_server(workspace_code, &normalized_server_code)
    }

    pub fn enable_workspace_server(
        &self,
        workspace_code: &str,
        server_code: &str,
    ) -> Result<WorkspaceEffectiveMcpServerRecord, McpError> {
        self.ensure_active_workspace(workspace_code)?;
        let normalized_server_code = normalize_mcp_server_code(server_code)?;
        let current = self.get_platform_server(&normalized_server_code)?;
        if !current.enabled {
            return Err(McpError::EffectiveEnableDenied);
        }
        self.repository
            .lock()
            .expect("MCP repository mutex poisoned")
            .delete_workspace_server_policy(workspace_code.trim(), &normalized_server_code)?;
        self.workspace_effective_server(workspace_code, &normalized_server_code)
    }

    pub fn variables_transport_availability(&self) -> Result<McpTransportAvailability, McpError> {
        let config = self.get_gateway_config()?;
        Ok(McpTransportAvailability {
            gateway_enabled: config.gateway_enabled,
            variables_endpoint_enabled: config.variables_endpoint_enabled,
        })
    }

    fn set_platform_server_enabled(
        &self,
        server_code: &str,
        enabled: bool,
    ) -> Result<PlatformMcpServerRecord, McpError> {
        let normalized_server_code = normalize_mcp_server_code(server_code)?;
        let mut repository = self
            .repository
            .lock()
            .expect("MCP repository mutex poisoned");
        let current = repository
            .get_platform_server(&normalized_server_code)?
            .ok_or(McpError::ServerNotFound)?;
        let updated = PlatformMcpServerRecord {
            enabled,
            updated_at: current_timestamp(),
            ..current
        };
        repository.upsert_platform_server(&updated)?;
        Ok(updated)
    }

    fn ensure_active_workspace(&self, workspace_code: &str) -> Result<WorkspaceDetail, McpError> {
        let code = workspace_code.trim();
        if code.is_empty() {
            return Err(McpError::WorkspaceRequired);
        }
        let workspace = self
            .workspaces
            .get_workspace(code)
            .map_err(map_workspace_error)?;
        if workspace.status != WorkspaceStatus::Active {
            return Err(McpError::InvalidWorkspace);
        }
        Ok(workspace)
    }

    fn resolve_workspace_effective_servers(
        &self,
        workspace_code: &str,
    ) -> Result<Vec<WorkspaceEffectiveMcpServerRecord>, McpError> {
        let servers = self
            .repository
            .lock()
            .expect("MCP repository mutex poisoned")
            .list_platform_servers()?;
        let policies = self
            .repository
            .lock()
            .expect("MCP repository mutex poisoned")
            .list_workspace_server_policies(Some(workspace_code.trim()))?;
        let policies_by_server: HashMap<_, _> = policies
            .into_iter()
            .map(|policy| (policy.server_code.clone(), policy))
            .collect();

        Ok(servers
            .into_iter()
            .map(|server| {
                let override_policy = policies_by_server.get(&server.server_code);
                let workspace_override_disabled = override_policy
                    .map(|policy| !policy.enabled)
                    .unwrap_or(false);
                let effective_enabled = server.enabled && !workspace_override_disabled;
                WorkspaceEffectiveMcpServerRecord {
                    server_code: server.server_code,
                    name: server.name,
                    description: server.description,
                    enabled: effective_enabled,
                    platform_enabled: server.enabled,
                    effective_enabled,
                    source: WorkspaceMcpServerSource::Platform,
                    workspace_override_disabled,
                    last_health_status: server.last_health_status,
                    last_health_message: server.last_health_message,
                    last_health_checked_at: server.last_health_checked_at,
                    updated_at: override_policy
                        .map(|policy| policy.updated_at.clone())
                        .unwrap_or(server.updated_at),
                }
            })
            .collect())
    }

    fn workspace_effective_server(
        &self,
        workspace_code: &str,
        server_code: &str,
    ) -> Result<WorkspaceEffectiveMcpServerRecord, McpError> {
        self.resolve_workspace_effective_servers(workspace_code)?
            .into_iter()
            .find(|record| record.server_code == server_code)
            .ok_or(McpError::ServerNotFound)
    }
}

fn normalized_search(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_lowercase())
        .filter(|item| !item.is_empty())
}

fn optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn normalize_headers(
    headers: BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, McpError> {
    let mut normalized = BTreeMap::new();
    for (key, value) in headers {
        let normalized_key = key.trim().to_string();
        if normalized_key.is_empty() {
            return Err(McpError::InvalidServerDefinition);
        }
        normalized.insert(normalized_key, value.trim().to_string());
    }
    Ok(normalized)
}

fn validate_auth_payload(
    auth_kind: McpServerAuthKind,
    auth_payload: Option<Value>,
) -> Result<Option<Value>, McpError> {
    match auth_kind {
        McpServerAuthKind::None => Ok(None),
        McpServerAuthKind::BearerStatic => {
            let payload = auth_payload.ok_or(McpError::InvalidServerDefinition)?;
            if payload
                .get("tokenRef")
                .and_then(Value::as_str)
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
            {
                Ok(Some(payload))
            } else {
                Err(McpError::InvalidServerDefinition)
            }
        }
    }
}

fn map_workspace_error(error: WorkspaceError) -> McpError {
    match error {
        WorkspaceError::StorageFailure => McpError::StorageFailure,
        WorkspaceError::NotFound => McpError::InvalidWorkspace,
        _ => McpError::InvalidWorkspace,
    }
}

fn matches_platform_status(
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

fn matches_platform_search(record: &PlatformMcpServerRecord, search: &str) -> bool {
    [
        record.server_code.as_str(),
        record.name.as_str(),
        record.description.as_deref().unwrap_or(""),
        record.url.as_str(),
    ]
    .join(" ")
    .to_lowercase()
    .contains(search)
}

fn matches_workspace_search(record: &WorkspaceEffectiveMcpServerRecord, search: &str) -> bool {
    [
        record.server_code.as_str(),
        record.name.as_str(),
        record.description.as_deref().unwrap_or(""),
    ]
    .join(" ")
    .to_lowercase()
    .contains(search)
}
