use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlatformMainAgentConfig {
    pub personality: String,
    pub instructions: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManagedAgentType {
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManagedAgentSource {
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SystemAgentStatus {
    Enabled,
    Disabled,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAgentRegistryEntry {
    pub agent_code: String,
    pub name: String,
    pub description: String,
    pub command_name: String,
    pub source: ManagedAgentSource,
    pub enabled: bool,
    pub available: bool,
    pub last_scanned_at: Option<String>,
    pub last_seen_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedSystemAgentRecord {
    pub agent_code: String,
    pub name: String,
    pub description: String,
    pub agent_type: ManagedAgentType,
    pub source: ManagedAgentSource,
    pub inherited_from: Option<String>,
    pub platform_enabled: Option<bool>,
    pub workspace_override_disabled: Option<bool>,
    pub status: SystemAgentStatus,
    pub enabled: bool,
    pub available: bool,
    #[serde(default)]
    pub models: Vec<SystemAgentModelRecord>,
    pub last_scanned_at: Option<String>,
    pub last_seen_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SystemAgentModelRecord {
    pub model_id: String,
    pub display_name: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSystemAgentPolicy {
    pub workspace_code: String,
    pub agent_code: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAgentSyncResult {
    pub scanned_count: usize,
    pub available_count: usize,
    pub unavailable_count: usize,
    pub records: Vec<ManagedSystemAgentRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeAdapterCapabilityRecord {
    pub adapter_key: String,
    pub name: String,
    pub stage: String,
    pub available: bool,
    pub enabled: bool,
    pub supports_resume: bool,
    pub supports_cancel: bool,
    pub supports_streaming: bool,
    pub supports_ephemeral: bool,
    pub supports_workdir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentManagementError {
    InvalidWorkspace,
    SystemAgentNotFound,
    SystemAgentUnavailable,
    StorageFailure,
}
