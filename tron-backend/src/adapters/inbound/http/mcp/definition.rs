use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlatformMcpGatewayConfig {
    pub gateway_enabled: bool,
    pub variables_endpoint_enabled: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpServerAuthKind {
    None,
    BearerStatic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum McpHealthStatus {
    NeverChecked,
    Healthy,
    Unhealthy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformMcpServerStatusFilter {
    Enabled,
    Disabled,
    Healthy,
    Unhealthy,
    NeverChecked,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlatformMcpServerListQuery {
    pub search: Option<String>,
    pub status: Option<PlatformMcpServerStatusFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformMcpServerRecord {
    pub server_code: String,
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub auth_kind: McpServerAuthKind,
    pub auth_payload: Option<Value>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    pub enabled: bool,
    pub last_health_status: McpHealthStatus,
    pub last_health_message: Option<String>,
    pub last_health_checked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct CreatePlatformMcpServerCommand {
    pub server_code: String,
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub auth_kind: McpServerAuthKind,
    pub auth_payload: Option<Value>,
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct UpdatePlatformMcpServerCommand {
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub auth_kind: McpServerAuthKind,
    pub auth_payload: Option<Value>,
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct UpdatePlatformMcpGatewayCommand {
    pub gateway_enabled: bool,
    pub variables_endpoint_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceMcpServerSource {
    Platform,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceMcpServerListQuery {
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceEffectiveMcpServerRecord {
    pub server_code: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub platform_enabled: bool,
    pub effective_enabled: bool,
    pub source: WorkspaceMcpServerSource,
    pub workspace_override_disabled: bool,
    pub last_health_status: McpHealthStatus,
    pub last_health_message: Option<String>,
    pub last_health_checked_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct WorkspaceMcpServerPolicy {
    pub workspace_code: String,
    pub server_code: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpDeleteResult {
    pub server_code: String,
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpTransportAvailability {
    pub gateway_enabled: bool,
    pub variables_endpoint_enabled: bool,
}
