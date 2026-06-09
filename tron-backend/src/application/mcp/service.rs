mod gateway;
mod runtime;
mod validation;

pub(crate) use runtime::RuntimeMcpWarning;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use reqwest::Client;

use crate::{
    application::variable::VariableService,
    application::workspace::WorkspaceService,
    domain::{
        mcp::{
            CreatePlatformMcpServerCommand, McpDeleteResult, McpError, McpHealthStatus,
            McpServerTransportKind, PlatformMcpGatewayConfig, PlatformMcpServerListQuery,
            PlatformMcpServerRecord, UpdatePlatformMcpServerCommand,
            VerifyPlatformMcpServerCommand, VerifyPlatformMcpServerDraftCommand,
            WorkspaceEffectiveMcpServerRecord, WorkspaceMcpServerListQuery,
            WorkspaceMcpServerPolicy, WorkspaceMcpServerSource, normalize_mcp_server_code,
            validate_mcp_server_name,
        },
        variable::VariableCatalogQuery,
        workspace::{WorkspaceDetail, WorkspaceStatus},
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
    variables: VariableService,
    http_client: Client,
    runtime_tool_bindings: runtime::RuntimeMcpToolBindings,
}

impl McpService {
    pub fn new(
        repository: impl McpRepository + 'static,
        workspaces: WorkspaceService,
        variables: VariableService,
        http_client: Client,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            workspaces,
            variables,
            http_client,
            runtime_tool_bindings: runtime::new_runtime_tool_bindings(),
        }
    }

    pub fn list_platform_servers(
        &self,
        query: PlatformMcpServerListQuery,
    ) -> Result<Vec<PlatformMcpServerRecord>, McpError> {
        let search = validation::normalized_search(query.search);
        let status = query.status;
        let mut records = self
            .repository
            .lock()
            .expect("MCP repository mutex poisoned")
            .list_platform_servers()?;
        records.retain(|record| {
            validation::matches_platform_status(record, status)
                && search
                    .as_deref()
                    .map(|term| validation::matches_platform_search(record, term))
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
        let record = self.build_server_record(command, true)?;
        let mut repository = self
            .repository
            .lock()
            .expect("MCP repository mutex poisoned");
        if repository
            .get_platform_server(&record.server_code)?
            .is_some()
        {
            return Err(McpError::DuplicateServerCode);
        }
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
        let updated = self.build_server_record_from_update(current, command)?;
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
        self.verify_platform_server(
            server_code,
            VerifyPlatformMcpServerCommand {
                workspace_code: None,
                definition: None,
            },
        )
        .await
    }

    pub async fn verify_platform_server_draft(
        &self,
        command: VerifyPlatformMcpServerDraftCommand,
    ) -> Result<PlatformMcpServerRecord, McpError> {
        let definition = self.build_server_record(command.definition, false)?;
        let verified = self
            .verify_server_definition(&definition, command.workspace_code.as_deref())
            .await?;
        if verified.last_health_status == McpHealthStatus::Healthy {
            Ok(verified)
        } else {
            Err(McpError::VerificationFailed(
                verified
                    .last_health_message
                    .clone()
                    .unwrap_or_else(|| "unknown verification failure".to_string()),
            ))
        }
    }

    pub async fn verify_platform_server(
        &self,
        server_code: &str,
        command: VerifyPlatformMcpServerCommand,
    ) -> Result<PlatformMcpServerRecord, McpError> {
        let normalized_server_code = normalize_mcp_server_code(server_code)?;
        let current = self
            .repository
            .lock()
            .expect("MCP repository mutex poisoned")
            .get_platform_server(&normalized_server_code)?
            .ok_or(McpError::ServerNotFound)?;
        let should_persist = command.definition.is_none();
        let target = if let Some(definition) = command.definition {
            self.build_server_record_from_update(current, definition)?
        } else {
            current
        };
        let verified = self
            .verify_server_definition(&target, command.workspace_code.as_deref())
            .await?;
        if should_persist {
            self.repository
                .lock()
                .expect("MCP repository mutex poisoned")
                .upsert_platform_server(&verified)?;
        }
        if verified.last_health_status == McpHealthStatus::Healthy {
            Ok(verified)
        } else {
            Err(McpError::VerificationFailed(
                verified
                    .last_health_message
                    .clone()
                    .unwrap_or_else(|| "unknown verification failure".to_string()),
            ))
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
        let search = validation::normalized_search(query.search);
        let records = self.resolve_workspace_effective_servers(workspace_code)?;
        Ok(records
            .into_iter()
            .filter(|record| {
                search
                    .as_deref()
                    .map(|term| validation::matches_workspace_search(record, term))
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

    fn build_server_record(
        &self,
        command: CreatePlatformMcpServerCommand,
        enabled: bool,
    ) -> Result<PlatformMcpServerRecord, McpError> {
        let validated = validation::validate_transport_definition(
            command.transport_kind,
            command.url,
            command.auth_kind,
            command.auth_payload,
            command.headers,
            command.headers_variable_refs,
            command.command,
            command.args,
            command.env_variable_refs,
            command.env_passthrough_refs,
            command.working_directory,
        )?;
        let now = current_timestamp();
        Ok(PlatformMcpServerRecord {
            server_code: normalize_mcp_server_code(&command.server_code)?,
            name: validate_mcp_server_name(&command.name)?,
            description: validation::optional_trimmed(command.description),
            transport_kind: validated.transport_kind,
            url: validated.url,
            auth_kind: validated.auth_kind,
            auth_payload: validated.auth_payload,
            headers: validated.headers,
            headers_variable_refs: validated.headers_variable_refs,
            command: validated.command,
            args: validated.args,
            env_variable_refs: validated.env_variable_refs,
            env_passthrough_refs: validated.env_passthrough_refs,
            working_directory: validated.working_directory,
            enabled,
            last_health_status: McpHealthStatus::NeverChecked,
            last_health_message: None,
            last_health_checked_at: None,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    fn build_server_record_from_update(
        &self,
        current: PlatformMcpServerRecord,
        command: UpdatePlatformMcpServerCommand,
    ) -> Result<PlatformMcpServerRecord, McpError> {
        let validated = validation::validate_transport_definition(
            command.transport_kind,
            command.url,
            command.auth_kind,
            command.auth_payload,
            command.headers,
            command.headers_variable_refs,
            command.command,
            command.args,
            command.env_variable_refs,
            command.env_passthrough_refs,
            command.working_directory,
        )?;
        Ok(PlatformMcpServerRecord {
            server_code: current.server_code,
            name: validate_mcp_server_name(&command.name)?,
            description: validation::optional_trimmed(command.description),
            transport_kind: validated.transport_kind,
            url: validated.url,
            auth_kind: validated.auth_kind,
            auth_payload: validated.auth_payload,
            headers: validated.headers,
            headers_variable_refs: validated.headers_variable_refs,
            command: validated.command,
            args: validated.args,
            env_variable_refs: validated.env_variable_refs,
            env_passthrough_refs: validated.env_passthrough_refs,
            working_directory: validated.working_directory,
            enabled: current.enabled,
            last_health_status: current.last_health_status,
            last_health_message: current.last_health_message,
            last_health_checked_at: current.last_health_checked_at,
            created_at: current.created_at,
            updated_at: current_timestamp(),
        })
    }

    async fn verify_server_definition(
        &self,
        server: &PlatformMcpServerRecord,
        workspace_code: Option<&str>,
    ) -> Result<PlatformMcpServerRecord, McpError> {
        let resolved_variables = if validation::needs_variable_resolution(server) {
            let workspace_code = workspace_code.ok_or(McpError::VerifyContextRequired)?;
            Some(self.resolve_workspace_variable_values(workspace_code)?)
        } else {
            None
        };

        let checked_at = current_timestamp();
        let outcome = match server.transport_kind {
            McpServerTransportKind::StreamableHttp => {
                let headers =
                    match validation::resolve_http_headers(server, resolved_variables.as_ref()) {
                        Ok(headers) => headers,
                        Err(McpError::VerificationFailed(message)) => {
                            let updated = PlatformMcpServerRecord {
                                last_health_status: McpHealthStatus::Unhealthy,
                                last_health_message: Some(message),
                                last_health_checked_at: Some(checked_at),
                                updated_at: current_timestamp(),
                                ..server.clone()
                            };
                            return Ok(updated);
                        }
                        Err(error) => return Err(error),
                    };
                runtime::verify_streamable_http_server(&self.http_client, server, &headers).await
            }
            McpServerTransportKind::Stdio => {
                let env = match validation::resolve_stdio_env(server, resolved_variables.as_ref()) {
                    Ok(env) => env,
                    Err(McpError::VerificationFailed(message)) => {
                        let updated = PlatformMcpServerRecord {
                            last_health_status: McpHealthStatus::Unhealthy,
                            last_health_message: Some(message),
                            last_health_checked_at: Some(checked_at),
                            updated_at: current_timestamp(),
                            ..server.clone()
                        };
                        return Ok(updated);
                    }
                    Err(error) => return Err(error),
                };
                runtime::verify_stdio_server(server, &env).await
            }
        };

        let (status, message) = match outcome {
            Ok(()) => (McpHealthStatus::Healthy, None),
            Err(message) => (McpHealthStatus::Unhealthy, Some(message)),
        };

        let updated = PlatformMcpServerRecord {
            last_health_status: status,
            last_health_message: message.clone(),
            last_health_checked_at: Some(checked_at),
            updated_at: current_timestamp(),
            ..server.clone()
        };

        Ok(updated)
    }

    fn resolve_workspace_variable_values(
        &self,
        workspace_code: &str,
    ) -> Result<HashMap<String, String>, McpError> {
        let variables = self
            .variables
            .list_variables(workspace_code, VariableCatalogQuery { search: None })
            .map_err(validation::map_variable_error)?;
        Ok(variables
            .into_iter()
            .map(|record| (record.key, record.value))
            .collect())
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
            .map_err(validation::map_workspace_error)?;
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
