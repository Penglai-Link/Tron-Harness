use super::McpService;
use crate::application::agent::tool_framework::FrameworkToolRegistryExt;
use crate::domain::{
    agent::{EffectiveTool, McpAccessMode, ToolRegistry},
    mcp::{
        McpError, McpHealthStatus, McpServerAuthKind, McpServerTransportKind,
        PlatformMcpServerRecord,
    },
};
use eliteforge_agent_core as fw;
use eliteforge_agent_official_plugin_tools_mcp::{
    McpClient, McpConfig, McpStdioFraming, McpTransport,
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub(crate) struct RuntimeToolMaterialization {
    pub tools: Vec<EffectiveTool>,
    pub descriptors: Vec<fw::ToolDescriptor>,
    pub warnings: Vec<RuntimeMcpWarning>,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeMcpWarning {
    pub server_code: String,
    pub server_name: String,
    pub message: String,
}

fn runtime_tool_name(server_code: &str, remote_tool_name: &str) -> String {
    let mut name = String::from("mcp_");
    for byte in server_code
        .as_bytes()
        .iter()
        .chain(std::iter::once(&b'_'))
        .chain(remote_tool_name.as_bytes())
    {
        if byte.is_ascii_alphanumeric() || *byte == b'_' || *byte == b'-' {
            name.push(*byte as char);
        } else {
            name.push('_');
        }
    }
    name
}

impl McpService {
    pub fn resolve_runtime_server_codes(
        &self,
        workspace_code: &str,
        access_mode: McpAccessMode,
        selected_server_codes: &[String],
    ) -> Result<Vec<String>, McpError> {
        Ok(self
            .resolve_runtime_servers(workspace_code, access_mode, selected_server_codes)?
            .into_iter()
            .map(|server| server.server_code)
            .collect())
    }
    pub(crate) async fn materialize_runtime_tools(
        &self,
        run_code: &str,
        workspace_code: &str,
        access_mode: McpAccessMode,
        selected_server_codes: &[String],
        variable_values: &HashMap<String, String>,
        registry: &ToolRegistry,
    ) -> Result<RuntimeToolMaterialization, McpError> {
        let mut tools = Vec::new();
        let mut descriptors = Vec::new();
        let mut warnings = Vec::new();
        for server in
            self.resolve_runtime_servers(workspace_code, access_mode, selected_server_codes)?
        {
            let discovered = async {
                let config = runtime_config(&server, variable_values)?;
                let client = McpClient::connect(config).await.map_err(mcp_error)?;
                match client.discover(&CancellationToken::new()).await {
                    Ok(bindings) => Ok((client, bindings)),
                    Err(error) => {
                        let _ = client.close().await;
                        Err(mcp_error(error))
                    }
                }
            }
            .await;
            let (client, bindings) = match discovered {
                Ok(value) => value,
                Err(error) => {
                    self.record_runtime_health(
                        &server.server_code,
                        Some(runtime_error_message(&error)),
                    );
                    warnings.push(runtime_warning(&server, error));
                    continue;
                }
            };
            self.record_runtime_health(&server.server_code, None);
            let mut public_bindings = Vec::new();
            for mut binding in bindings {
                // Resource operations use their public call name and executor too.
                let remote_name = binding
                    .descriptor
                    .id
                    .splitn(3, '/')
                    .nth(2)
                    .unwrap_or(&binding.descriptor.call_name)
                    .to_owned();
                let tool_name = runtime_tool_name(&server.server_code, &remote_name);
                if tools
                    .iter()
                    .any(|tool: &EffectiveTool| tool.tool_name == tool_name)
                {
                    client.cancel();
                    return Err(McpError::RuntimeDiscoveryFailed(
                        "MCP names collide after host mapping".into(),
                    ));
                }
                let domain_code = format!("mcp.{}", server.server_code);
                tools.push(EffectiveTool {
                    tool_name: tool_name.clone(),
                    domain_code: Some(domain_code.clone()),
                    description: binding.descriptor.description.clone(),
                    input_schema: binding.descriptor.input_schema.clone(),
                    skill_name: None,
                    required_variable_keys: required_variable_keys(&server),
                });
                binding.domain_code = Some(domain_code);
                binding.category =
                    Some(crate::application::agent::builtin_category("tron.platform"));
                binding.descriptor.id = format!("tron.{tool_name}");
                binding.descriptor.call_name = tool_name;
                if let (Some(resources), Some(owner)) = (
                    self.capabilities.get(),
                    self.capability_resource(&server.server_code),
                ) {
                    resources
                        .register(&[crate::application::capability::CapabilityResource {
                            pack_id: owner.pack_id,
                            kind: "tool".into(),
                            code: binding.descriptor.id.clone(),
                            available: owner.available,
                        }])
                        .map_err(McpError::RuntimeDiscoveryFailed)?;
                }

                binding.executor = std::sync::Arc::new(CurrentMcpTool {
                    inner: binding.executor,
                    service: self.clone(),
                    server: server.server_code.clone(),
                    tool: remote_name,
                });
                descriptors.push(binding.descriptor.clone());
                public_bindings.push(binding);
            }
            let close = client.clone();
            registry.register_framework_lease(
                run_code,
                fw::ToolLease::new(public_bindings)
                    .with_cleanup(move || {
                        client.cancel();
                    })
                    .with_shutdown(move || async move {
                        close.close().await.map_err(|error| error.to_string())
                    }),
            );
        }
        Ok(RuntimeToolMaterialization {
            tools,
            descriptors,
            warnings,
        })
    }
}
fn mcp_error(error: impl ToString) -> McpError {
    McpError::RuntimeDiscoveryFailed(error.to_string())
}
fn runtime_config(
    server: &PlatformMcpServerRecord,
    variables: &HashMap<String, String>,
) -> Result<McpConfig, McpError> {
    use sha2::{Digest, Sha256};
    let namespace = format!(
        "tron_{}",
        hex::encode(&Sha256::digest(server.server_code.as_bytes())[..8])
    );
    let transport = match server.transport_kind {
        McpServerTransportKind::Stdio => McpTransport::Stdio {
            program: server.command.clone().into(),
            args: server.args.clone(),
            env: resolve_runtime_stdio_env(server, variables)?
                .into_iter()
                .collect(),
            current_dir: server.working_directory.clone().map(Into::into),
            inherit_env: true,
        },
        McpServerTransportKind::StreamableHttp => McpTransport::StreamableHttp {
            url: server.url.clone(),
            headers: resolve_runtime_headers(server, variables)?
                .into_iter()
                .map(|(key, value)| {
                    Ok((
                        key.parse().map_err(mcp_error)?,
                        value.parse().map_err(mcp_error)?,
                    ))
                })
                .collect::<Result<_, McpError>>()?,
        },
    };
    let mut config = McpConfig::new(namespace, transport);
    config.stdio_framing = McpStdioFraming::Auto;
    config.connect_timeout = Duration::from_secs(3);
    config.request_timeout = Duration::from_secs(30);
    Ok(config)
}
pub(super) async fn verify_stdio_server(
    server: &PlatformMcpServerRecord,
    env: &BTreeMap<String, String>,
) -> Result<(), String> {
    let mut clone = server.clone();
    clone.env_variable_refs.clear();
    clone.env_passthrough_refs.clear();
    let mut config = runtime_config(&clone, &HashMap::new()).map_err(|error| error.to_string())?;
    if let McpTransport::Stdio { env: target, .. } = &mut config.transport {
        *target = env.clone().into_iter().collect();
    }
    verify(config).await
}
pub(super) async fn verify_streamable_http_server(
    _http: &reqwest::Client,
    server: &PlatformMcpServerRecord,
    headers: &BTreeMap<String, String>,
) -> Result<(), String> {
    let mut clone = server.clone();
    clone.auth_kind = McpServerAuthKind::None;
    clone.headers_variable_refs.clear();
    clone.headers = headers.clone();
    verify(runtime_config(&clone, &HashMap::new()).map_err(|error| error.to_string())?).await
}
async fn verify(config: McpConfig) -> Result<(), String> {
    let client = McpClient::connect(config)
        .await
        .map_err(|error| error.to_string())?;
    let result = client.discover(&CancellationToken::new()).await;
    let closed = client.close().await;
    result
        .map(|_| ())
        .and(closed)
        .map_err(|error| error.to_string())
}
impl McpService {
    // Update the current row rather than writing back a run's stale configuration.
    fn record_runtime_health(&self, server_code: &str, failure: Option<String>) {
        let result = (|| -> Result<(), McpError> {
            let mut repository = self
                .repository
                .lock()
                .expect("MCP repository mutex poisoned");
            let Some(mut server) = repository.get_platform_server(server_code)? else {
                return Ok(());
            };
            server.last_health_status = if failure.is_some() {
                McpHealthStatus::Unhealthy
            } else {
                McpHealthStatus::Healthy
            };
            server.last_health_message = failure.map(|message| {
                crate::application::short_term_memory::redact_sensitive_text(&message)
            });
            server.last_health_checked_at = Some(crate::shared::time::current_timestamp());
            repository.upsert_platform_server(&server)
        })();
        if let Err(error) = result {
            tracing::warn!(server_code, error = ?error, "MCP runtime health update failed");
        }
    }

    fn resolve_runtime_servers(
        &self,
        workspace_code: &str,
        access_mode: McpAccessMode,
        selected_server_codes: &[String],
    ) -> Result<Vec<PlatformMcpServerRecord>, McpError> {
        if !workspace_code.is_empty() {
            self.ensure_active_workspace(workspace_code)?;
        }
        if matches!(access_mode, McpAccessMode::Disabled) {
            return Ok(Vec::new());
        }

        let servers = self
            .repository
            .lock()
            .expect("MCP repository mutex poisoned")
            .list_platform_servers()?;
        let policies = if workspace_code.is_empty() {
            Vec::new()
        } else {
            self.repository
                .lock()
                .expect("MCP repository mutex poisoned")
                .list_workspace_server_policies(Some(workspace_code.trim()))?
        };
        let policies_by_server: HashMap<_, _> = policies
            .into_iter()
            .map(|policy| (policy.server_code.clone(), policy))
            .collect();

        let effective_enabled = servers
            .into_iter()
            .filter(|server| {
                let workspace_override_disabled = policies_by_server
                    .get(&server.server_code)
                    .map(|policy| !policy.enabled)
                    .unwrap_or(false);
                server.enabled
                    && !workspace_override_disabled
                    && self.capability_available(&server.server_code)
            })
            .collect::<Vec<_>>();

        match access_mode {
            McpAccessMode::Disabled => Ok(Vec::new()),
            McpAccessMode::AllEnabledServers => Ok(effective_enabled),
            McpAccessMode::SelectedServers => {
                let selected = normalize_string_codes(selected_server_codes.to_vec());
                let effective_by_code = effective_enabled
                    .into_iter()
                    .map(|server| (server.server_code.clone(), server))
                    .collect::<HashMap<_, _>>();
                Ok(selected
                    .into_iter()
                    .filter_map(|server_code| effective_by_code.get(&server_code).cloned())
                    .collect())
            }
        }
    }
}
fn required_variable_keys(server: &PlatformMcpServerRecord) -> Vec<String> {
    let mut keys = Vec::new();
    if let McpServerAuthKind::BearerStatic = server.auth_kind
        && let Some(token_ref) = server
            .auth_payload
            .as_ref()
            .and_then(|payload| payload.get("tokenRef"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    {
        keys.push(token_ref.to_string());
    }
    keys.extend(server.headers_variable_refs.values().cloned());
    keys.extend(server.env_variable_refs.values().cloned());
    keys.sort_unstable();
    keys.dedup();
    keys
}

fn resolve_runtime_headers(
    server: &PlatformMcpServerRecord,
    variable_values: &HashMap<String, String>,
) -> Result<BTreeMap<String, String>, McpError> {
    let mut headers = server.headers.clone();
    for (header_name, variable_ref) in &server.headers_variable_refs {
        let value = resolve_runtime_variable_value(server, variable_values, variable_ref)?;
        headers.insert(header_name.clone(), value);
    }
    if let McpServerAuthKind::BearerStatic = server.auth_kind {
        let token_ref = server
            .auth_payload
            .as_ref()
            .and_then(|payload| payload.get("tokenRef"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                McpError::RuntimeDiscoveryFailed(format!(
                    "MCP server `{}` is missing auth tokenRef",
                    server.server_code
                ))
            })?;
        let token = resolve_runtime_variable_value(server, variable_values, token_ref)?;
        headers.insert("Authorization".to_string(), format!("Bearer {token}"));
    }
    Ok(headers)
}

fn resolve_runtime_stdio_env(
    server: &PlatformMcpServerRecord,
    variable_values: &HashMap<String, String>,
) -> Result<BTreeMap<String, String>, McpError> {
    let mut env = BTreeMap::new();
    for (env_name, variable_ref) in &server.env_variable_refs {
        let value = resolve_runtime_variable_value(server, variable_values, variable_ref)?;
        env.insert(env_name.clone(), value);
    }
    for passthrough_name in &server.env_passthrough_refs {
        if let Ok(value) = std::env::var(passthrough_name) {
            env.insert(passthrough_name.clone(), value);
        }
    }
    Ok(env)
}

fn resolve_runtime_variable_value(
    server: &PlatformMcpServerRecord,
    variable_values: &HashMap<String, String>,
    variable_ref: &str,
) -> Result<String, McpError> {
    variable_values
        .get(variable_ref)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            McpError::RuntimeDiscoveryFailed(format!(
                "MCP server `{}` requires variable `{}`",
                server.server_code, variable_ref
            ))
        })
}

fn runtime_warning(server: &PlatformMcpServerRecord, error: McpError) -> RuntimeMcpWarning {
    let detail = runtime_error_message(&error);
    RuntimeMcpWarning {
        server_code: server.server_code.clone(),
        server_name: server.name.clone(),
        message: format!(
            "MCP server `{}` is unavailable; continuing without its tools. {detail}",
            server.name
        ),
    }
}

fn runtime_error_message(error: &McpError) -> String {
    match error {
        McpError::RuntimeDiscoveryFailed(message) | McpError::VerificationFailed(message) => {
            message.clone()
        }
        other => other.to_string(),
    }
}

fn normalize_string_codes(values: Vec<String>) -> Vec<String> {
    let mut normalized = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    normalized.sort_unstable();
    normalized.dedup();
    normalized
}

struct CurrentMcpTool {
    inner: std::sync::Arc<dyn fw::Tool>,
    service: McpService,
    server: String,
    tool: String,
}
#[async_trait::async_trait]
impl fw::Tool for CurrentMcpTool {
    async fn call(
        &self,
        args: Value,
        run: &fw::RunContext,
    ) -> Result<fw::ToolOutput, fw::ToolError> {
        let server = self
            .service
            .get_platform_server(&self.server)
            .map_err(|error| {
                fw::ToolError::Failed(format!(
                    "MCP server `{}` lookup failed: {error}",
                    self.server
                ))
            })?;
        if !self.service.capability_available(&self.server) || !server.enabled {
            return Err(fw::ToolError::Failed(format!(
                "MCP server `{}` or its capability package is disabled",
                self.server
            )));
        }
        match self.inner.call(args, run).await {
            Ok(output) => Ok(output),
            Err(error) => {
                let (indeterminate, detail) = match error {
                    fw::ToolError::Indeterminate(detail) => (true, detail),
                    fw::ToolError::Failed(detail) => (false, detail),
                };
                let message = format!(
                    "MCP server `{}`, tool `{}`: {detail}",
                    self.server, self.tool
                );
                if indeterminate {
                    self.service
                        .record_runtime_health(&self.server, Some(message.clone()));
                    // A failed response does not prove that the remote operation
                    // had no effect. Preserve uncertainty and never replay it.
                    Err(fw::ToolError::Indeterminate(message))
                } else {
                    Err(fw::ToolError::Failed(message))
                }
            }
        }
    }
}
