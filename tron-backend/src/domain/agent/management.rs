use serde_json::Value;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManagedAgentType {
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToolAssignmentMode {
    AllPlatformTools,
    SelectedTools,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToolExposureMode {
    DirectList,
    DomainDiscovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExpandedDomainPersistenceMode {
    StickyExpanded,
    EphemeralExpanded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum McpAccessMode {
    Disabled,
    AllEnabledServers,
    SelectedServers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InternalAgentPolicySource {
    PlatformDefault,
    WorkspaceOverride,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInternalAgentPolicy {
    pub workspace_code: String,
    pub agent_code: String,
    pub tool_assignment_mode: ToolAssignmentMode,
    pub mcp_access_mode: McpAccessMode,
    pub tool_exposure_mode: ToolExposureMode,
    pub expanded_domain_persistence_mode: ExpandedDomainPersistenceMode,
    pub max_steps: u32,
    pub max_tool_calls: u32,
    pub max_parallel_tool_calls: u32,
    pub overridden_fields: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInternalAgentDefinition {
    pub workspace_code: String,
    pub agent_code: String,
    pub name: Option<String>,
    pub role_prompt: Option<String>,
    #[serde(default)]
    pub loop_code: Option<String>,
    #[serde(default)]
    pub avatar_id: Option<String>,
    #[serde(default)]
    pub group_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformInternalAgentConfig {
    pub agent_code: String,
    pub name: String,
    pub role_prompt: String,
    #[serde(default = "default_loop_code")]
    pub loop_code: String,
    #[serde(default = "default_avatar_id")]
    pub avatar_id: String,
    #[serde(default)]
    pub group_name: Option<String>,
    pub tool_assignment_mode: ToolAssignmentMode,
    pub mcp_access_mode: McpAccessMode,
    pub tool_exposure_mode: ToolExposureMode,
    pub expanded_domain_persistence_mode: ExpandedDomainPersistenceMode,
    pub max_steps: u32,
    pub max_tool_calls: u32,
    pub max_parallel_tool_calls: u32,
    pub selected_tool_codes: Vec<String>,
    pub selected_server_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInternalAgentToolAssignment {
    pub workspace_code: String,
    pub agent_code: String,
    pub tool_code: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInternalAgentMcpServerAssignment {
    pub workspace_code: String,
    pub agent_code: String,
    pub server_code: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalAgentToolPolicy {
    pub workspace_code: String,
    pub agent_code: String,
    pub name: String,
    pub description: String,
    pub role_prompt: String,
    #[serde(default = "default_loop_code")]
    pub loop_code: String,
    #[serde(default = "default_avatar_id")]
    pub avatar_id: String,
    #[serde(default)]
    pub group_name: Option<String>,
    pub platform_defined: bool,
    #[serde(default)]
    pub built_in: bool,
    pub source: InternalAgentPolicySource,
    pub tool_assignment_mode: ToolAssignmentMode,
    pub mcp_access_mode: McpAccessMode,
    pub tool_exposure_mode: ToolExposureMode,
    pub expanded_domain_persistence_mode: ExpandedDomainPersistenceMode,
    pub max_steps: u32,
    pub max_tool_calls: u32,
    pub max_parallel_tool_calls: u32,
    pub selected_tool_codes: Vec<String>,
    pub selected_server_codes: Vec<String>,
    pub assigned_tool_count: usize,
    pub effective_tool_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentRuntimeType {
    React,
    CodexExec,
    CodexAppServer,
    OpenCodeRun,
    OpenCodeServe,
    OpenCodeAcp,
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
    pub runtime_type: AgentRuntimeType,
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
    pub runtime_type: AgentRuntimeType,
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
    pub supports_steer: bool,
    pub supports_streaming: bool,
    pub supports_ephemeral: bool,
    pub supports_workdir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentManagementError {
    InvalidWorkspace,
    InternalAgentDefinitionInvalid,
    InternalAgentGroupInvalid,
    BuiltinAgentReadOnly,
    LoopInvalid,
    LoopNotFound,
    LoopConflict,
    LoopInUse,
    InternalAgentCodeInvalid,
    InternalAgentCodeConflict,
    InternalAgentPolicyInvalid,
    InternalAgentNotFound,
    SystemAgentNotFound,
    InternalToolAssignmentStale(String),
    InternalMcpServerAssignmentStale(String),
    InternalToolUnknown(String),
    SystemAgentUnavailable,
    ToolPermissionInvalid,
    StorageFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToolDomainRecord {
    pub domain_code: String,
    pub name: String,
    pub description: String,
    pub tool_count: usize,
    pub sample_tools: Vec<String>,
    pub has_mutation_tools: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToolOperationKind {
    Query,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToolAvailabilityState {
    Available,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCatalogRecord {
    pub tool_code: String,
    pub tool_name: String,
    pub domain_code: String,
    pub name: String,
    pub description: String,
    pub operation_kind: ToolOperationKind,
    pub input_schema: Value,
    pub module_owner: String,
    pub availability_state: ToolAvailabilityState,
    pub sample_result_summary: String,
}

fn default_loop_code() -> String {
    super::loop_design::REACT_LOOP_CODE.into()
}

fn default_avatar_id() -> String {
    "robot".into()
}
