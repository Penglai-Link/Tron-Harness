use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};

use reqwest::Client;
use serde_json::Value;
use tokio::runtime::Handle;

use crate::domain::{
    agent::{
        EffectiveTool, McpAccessMode, Tool, ToolError, ToolExecutionResult, ToolObservation,
        ToolRegistry, ToolRequest,
    },
    mcp::{McpError, McpServerAuthKind, McpServerTransportKind, PlatformMcpServerRecord},
};

use super::McpService;

mod http;
mod stdio;

pub(super) type RuntimeMcpToolBindings =
    Arc<Mutex<HashMap<String, HashMap<String, RuntimeMcpToolBinding>>>>;

pub(super) fn new_runtime_tool_bindings() -> RuntimeMcpToolBindings {
    Arc::new(Mutex::new(HashMap::new()))
}

#[derive(Debug, Clone)]
pub(super) enum RuntimeMcpToolBinding {
    StreamableHttp {
        server_code: String,
        remote_tool_name: String,
        url: String,
        headers: BTreeMap<String, String>,
    },
    Stdio {
        server_code: String,
        remote_tool_name: String,
        command: String,
        args: Vec<String>,
        env: BTreeMap<String, String>,
        working_directory: Option<String>,
        protocol_mode: stdio::StdioProtocolMode,
    },
}

#[derive(Debug, Clone)]
pub(super) struct DiscoveredMcpTool {
    remote_tool_name: String,
    description: Option<String>,
    input_schema: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeToolMaterialization {
    pub tools: Vec<EffectiveTool>,
    pub warnings: Vec<RuntimeMcpWarning>,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeMcpWarning {
    pub server_code: String,
    pub server_name: String,
    pub message: String,
}

struct McpRuntimeToolDispatch {
    name: String,
    http_client: Client,
    runtime_handle: Handle,
    bindings: RuntimeMcpToolBindings,
}

impl McpRuntimeToolDispatch {
    fn new(
        name: String,
        http_client: Client,
        runtime_handle: Handle,
        bindings: RuntimeMcpToolBindings,
    ) -> Self {
        Self {
            name,
            http_client,
            runtime_handle,
            bindings,
        }
    }
}

impl Tool for McpRuntimeToolDispatch {
    fn name(&self) -> &str {
        &self.name
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &crate::domain::agent::AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let binding = self
            .bindings
            .lock()
            .expect("MCP runtime binding mutex poisoned")
            .get(&self.name)
            .and_then(|runs| runs.get(&context.run_code))
            .cloned()
            .ok_or_else(|| {
                ToolError::ExecutionFailed(format!(
                    "MCP runtime binding for `{}` is unavailable in run `{}`",
                    self.name, context.run_code
                ))
            })?;
        let tool_name = self.name.clone();
        let http_client = self.http_client.clone();
        let runtime_handle = self.runtime_handle.clone();
        let input = request.input;
        let output = runtime_handle
            .block_on(async move { invoke_runtime_tool_call(&http_client, binding, input).await })
            .map_err(ToolError::ExecutionFailed)?;

        Ok(ToolExecutionResult::observation(ToolObservation {
            tool_name,
            output,
        }))
    }
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
        tool_registry: &ToolRegistry,
    ) -> Result<RuntimeToolMaterialization, McpError> {
        let runtime_handle = Handle::current();
        let servers =
            self.resolve_runtime_servers(workspace_code, access_mode, selected_server_codes)?;
        let mut effective_tools = Vec::new();
        let mut warnings = Vec::new();

        for server in servers {
            let required_variable_keys = required_variable_keys(&server);
            match server.transport_kind {
                McpServerTransportKind::StreamableHttp => {
                    let headers = match resolve_runtime_headers(&server, variable_values) {
                        Ok(headers) => headers,
                        Err(error) => {
                            warnings.push(runtime_warning(&server, error));
                            continue;
                        }
                    };
                    let discovered_tools = match http::discover_streamable_http_tools(
                        &self.http_client,
                        &server,
                        &headers,
                    )
                    .await
                    {
                        Ok(discovered_tools) => discovered_tools,
                        Err(error) => {
                            warnings.push(runtime_warning(&server, error));
                            continue;
                        }
                    };

                    for discovered_tool in discovered_tools {
                        let tool_name = format!(
                            "mcp.{}.{}",
                            server.server_code, discovered_tool.remote_tool_name
                        );
                        let binding = RuntimeMcpToolBinding::StreamableHttp {
                            server_code: server.server_code.clone(),
                            remote_tool_name: discovered_tool.remote_tool_name.clone(),
                            url: server.url.clone(),
                            headers: headers.clone(),
                        };
                        self.bind_runtime_tool(&tool_name, run_code, binding);
                        tool_registry
                            .register(Arc::new(McpRuntimeToolDispatch::new(
                                tool_name.clone(),
                                self.http_client.clone(),
                                runtime_handle.clone(),
                                self.runtime_tool_bindings.clone(),
                            )))
                            .map_err(|error| McpError::RuntimeDiscoveryFailed(error.to_string()))?;
                        effective_tools.push(EffectiveTool {
                            tool_name,
                            domain_code: Some(format!("mcp.{}", server.server_code)),
                            description: discovered_tool.description.unwrap_or_else(|| {
                                format!(
                                    "Call remote MCP tool `{}` from `{}`.",
                                    discovered_tool.remote_tool_name, server.name
                                )
                            }),
                            input_schema: http::normalize_runtime_input_schema(
                                discovered_tool.input_schema,
                            ),
                            skill_name: None,
                            required_variable_keys: required_variable_keys.clone(),
                        });
                    }
                }
                McpServerTransportKind::Stdio => {
                    let env = match resolve_runtime_stdio_env(&server, variable_values) {
                        Ok(env) => env,
                        Err(error) => {
                            warnings.push(runtime_warning(&server, error));
                            continue;
                        }
                    };
                    let discovery = match stdio::discover_stdio_tools(&server, &env).await {
                        Ok(discovery) => discovery,
                        Err(error) => {
                            warnings.push(runtime_warning(&server, error));
                            continue;
                        }
                    };

                    for discovered_tool in discovery.tools {
                        let tool_name = format!(
                            "mcp.{}.{}",
                            server.server_code, discovered_tool.remote_tool_name
                        );
                        let binding = RuntimeMcpToolBinding::Stdio {
                            server_code: server.server_code.clone(),
                            remote_tool_name: discovered_tool.remote_tool_name.clone(),
                            command: server.command.clone(),
                            args: server.args.clone(),
                            env: env.clone(),
                            working_directory: server.working_directory.clone(),
                            protocol_mode: discovery.protocol_mode,
                        };
                        self.bind_runtime_tool(&tool_name, run_code, binding);
                        tool_registry
                            .register(Arc::new(McpRuntimeToolDispatch::new(
                                tool_name.clone(),
                                self.http_client.clone(),
                                runtime_handle.clone(),
                                self.runtime_tool_bindings.clone(),
                            )))
                            .map_err(|error| McpError::RuntimeDiscoveryFailed(error.to_string()))?;
                        effective_tools.push(EffectiveTool {
                            tool_name,
                            domain_code: Some(format!("mcp.{}", server.server_code)),
                            description: discovered_tool.description.unwrap_or_else(|| {
                                format!(
                                    "Call remote MCP tool `{}` from `{}`.",
                                    discovered_tool.remote_tool_name, server.name
                                )
                            }),
                            input_schema: http::normalize_runtime_input_schema(
                                discovered_tool.input_schema,
                            ),
                            skill_name: None,
                            required_variable_keys: required_variable_keys.clone(),
                        });
                    }
                }
                }
            }
        }

        Ok(RuntimeToolMaterialization {
            tools: effective_tools,
            warnings,
        })
    }
}

impl McpService {
    fn resolve_runtime_servers(
        &self,
        workspace_code: &str,
        access_mode: McpAccessMode,
        selected_server_codes: &[String],
    ) -> Result<Vec<PlatformMcpServerRecord>, McpError> {
        self.ensure_active_workspace(workspace_code)?;
        if matches!(access_mode, McpAccessMode::Disabled) {
            return Ok(Vec::new());
        }

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

        let effective_enabled = servers
            .into_iter()
            .filter(|server| {
                let workspace_override_disabled = policies_by_server
                    .get(&server.server_code)
                    .map(|policy| !policy.enabled)
                    .unwrap_or(false);
                server.enabled && !workspace_override_disabled
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

    fn bind_runtime_tool(&self, tool_name: &str, run_code: &str, binding: RuntimeMcpToolBinding) {
        self.runtime_tool_bindings
            .lock()
            .expect("MCP runtime binding mutex poisoned")
            .entry(tool_name.to_string())
            .or_default()
            .insert(run_code.to_string(), binding);
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

async fn invoke_runtime_tool_call(
    http_client: &Client,
    binding: RuntimeMcpToolBinding,
    input: Value,
) -> Result<String, String> {
    match binding {
        RuntimeMcpToolBinding::StreamableHttp {
            server_code,
            remote_tool_name,
            url,
            headers,
        } => http::call_streamable_http_tool(http_client, &url, &headers, &remote_tool_name, input)
            .await
            .map_err(|error| {
                format!(
                    "MCP tool `{}` on server `{}` failed: {}",
                    remote_tool_name,
                    server_code,
                    runtime_error_message(&error)
                )
            }),
        RuntimeMcpToolBinding::Stdio {
            server_code,
            remote_tool_name,
            command,
            args,
            env,
            working_directory,
            protocol_mode,
        } => stdio::invoke_stdio_tool_call(
            server_code.clone(),
            remote_tool_name.clone(),
            command,
            args,
            env,
            working_directory,
            protocol_mode,
            input,
        )
        .await
        .map_err(|error| {
            format!("MCP tool `{remote_tool_name}` on server `{server_code}` failed: {error}")
        }),
    }
}

pub(super) async fn verify_stdio_server(
    server: &PlatformMcpServerRecord,
    env: &BTreeMap<String, String>,
) -> Result<(), String> {
    stdio::verify_stdio_server(server, env).await
}

pub(super) async fn verify_streamable_http_server(
    http_client: &Client,
    server: &PlatformMcpServerRecord,
    headers: &BTreeMap<String, String>,
) -> Result<(), String> {
    http::verify_streamable_http_server(http_client, server, headers).await
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
