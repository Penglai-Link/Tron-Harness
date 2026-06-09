use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use crate::domain::mcp::{
    CreatePlatformMcpServerCommand, McpServerAuthKind, McpServerTransportKind,
    UpdatePlatformMcpGatewayCommand, UpdatePlatformMcpServerCommand,
    VerifyPlatformMcpServerCommand, VerifyPlatformMcpServerDraftCommand,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdatePlatformMcpGatewayRequest {
    pub gateway_enabled: bool,
    pub variables_endpoint_enabled: bool,
}

impl From<UpdatePlatformMcpGatewayRequest> for UpdatePlatformMcpGatewayCommand {
    fn from(request: UpdatePlatformMcpGatewayRequest) -> Self {
        Self {
            gateway_enabled: request.gateway_enabled,
            variables_endpoint_enabled: request.variables_endpoint_enabled,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreatePlatformMcpServerRequest {
    pub server_code: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_streamable_http_transport_kind")]
    pub transport_kind: McpServerTransportKind,
    pub url: String,
    pub auth_kind: McpServerAuthKind,
    pub auth_payload: Option<Value>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub headers_variable_refs: BTreeMap<String, String>,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env_variable_refs: BTreeMap<String, String>,
    #[serde(default)]
    pub env_passthrough_refs: Vec<String>,
    pub working_directory: Option<String>,
}

impl From<CreatePlatformMcpServerRequest> for CreatePlatformMcpServerCommand {
    fn from(request: CreatePlatformMcpServerRequest) -> Self {
        Self {
            server_code: request.server_code,
            name: request.name,
            description: request.description,
            transport_kind: request.transport_kind,
            url: request.url,
            auth_kind: request.auth_kind,
            auth_payload: request.auth_payload,
            headers: request.headers,
            headers_variable_refs: request.headers_variable_refs,
            command: request.command,
            args: request.args,
            env_variable_refs: request.env_variable_refs,
            env_passthrough_refs: request.env_passthrough_refs,
            working_directory: request.working_directory,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdatePlatformMcpServerRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_streamable_http_transport_kind")]
    pub transport_kind: McpServerTransportKind,
    pub url: String,
    pub auth_kind: McpServerAuthKind,
    pub auth_payload: Option<Value>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub headers_variable_refs: BTreeMap<String, String>,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env_variable_refs: BTreeMap<String, String>,
    #[serde(default)]
    pub env_passthrough_refs: Vec<String>,
    pub working_directory: Option<String>,
}

impl From<UpdatePlatformMcpServerRequest> for UpdatePlatformMcpServerCommand {
    fn from(request: UpdatePlatformMcpServerRequest) -> Self {
        Self {
            name: request.name,
            description: request.description,
            transport_kind: request.transport_kind,
            url: request.url,
            auth_kind: request.auth_kind,
            auth_payload: request.auth_payload,
            headers: request.headers,
            headers_variable_refs: request.headers_variable_refs,
            command: request.command,
            args: request.args,
            env_variable_refs: request.env_variable_refs,
            env_passthrough_refs: request.env_passthrough_refs,
            working_directory: request.working_directory,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerifyPlatformMcpServerDraftRequest {
    pub workspace_code: Option<String>,
    pub definition: CreatePlatformMcpServerRequest,
}

impl From<VerifyPlatformMcpServerDraftRequest> for VerifyPlatformMcpServerDraftCommand {
    fn from(request: VerifyPlatformMcpServerDraftRequest) -> Self {
        Self {
            workspace_code: request.workspace_code,
            definition: request.definition.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerifyPlatformMcpServerRequest {
    pub workspace_code: Option<String>,
    pub definition: Option<UpdatePlatformMcpServerRequest>,
}

impl From<VerifyPlatformMcpServerRequest> for VerifyPlatformMcpServerCommand {
    fn from(request: VerifyPlatformMcpServerRequest) -> Self {
        Self {
            workspace_code: request.workspace_code,
            definition: request.definition.map(Into::into),
        }
    }
}

const fn default_streamable_http_transport_kind() -> McpServerTransportKind {
    McpServerTransportKind::StreamableHttp
}
