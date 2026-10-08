mod internal_agents;
mod internal_definition_error;
mod internal_definitions;
mod platform_internal_agents;
mod system_agents;
mod workspace_grants;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use super::{
    BusinessToolCatalog,
    runtime_capability::{build_runtime_capabilities, find_runtime_capability},
    system_agent_model_probe::{ProcessSystemCommandProbe, SystemCommandProbe},
};

use self::system_agents::supported_system_agents_from_config;

use crate::{
    application::workspace::WorkspaceService,
    config::SystemAgentConfig,
    domain::{
        agent::{
            AgentManagementError, AgentRuntimeType, ExpandedDomainPersistenceMode,
            InternalAgentPolicySource, InternalAgentToolPolicy, ManagedAgentSource,
            ManagedAgentType, ManagedSystemAgentRecord, McpAccessMode, PlatformInternalAgentConfig,
            RuntimeAdapterCapabilityRecord, SystemAgentModelRecord, SystemAgentRegistryEntry,
            SystemAgentStatus, SystemAgentSyncResult, ToolAssignmentMode, ToolCatalogRecord,
            ToolDomainRecord, ToolExposureMode, WorkspaceInternalAgentDefinition,
            WorkspaceInternalAgentMcpServerAssignment, WorkspaceInternalAgentPolicy,
            WorkspaceInternalAgentToolAssignment, WorkspaceSystemAgentPolicy,
        },
        workspace::{WorkspaceDetail, WorkspaceError, WorkspaceStatus},
    },
    shared::time::current_timestamp,
};

#[derive(Debug, Clone, Default)]
pub struct UpdateWorkspaceInternalAgentPolicyCommand {
    pub tool_assignment_mode: Option<ToolAssignmentMode>,
    pub tool_exposure_mode: Option<ToolExposureMode>,
    pub expanded_domain_persistence_mode: Option<ExpandedDomainPersistenceMode>,
    pub max_steps: Option<u32>,
    pub max_tool_calls: Option<u32>,
    pub max_parallel_tool_calls: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct CreateWorkspaceInternalAgentCommand {
    pub agent_code: String,
    pub name: String,
    pub role_prompt: String,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateWorkspaceInternalAgentDefinitionCommand {
    pub name: Option<String>,
    pub role_prompt: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateWorkspaceInternalAgentMcpPolicyCommand {
    pub mcp_access_mode: Option<McpAccessMode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAgentRuntime {
    pub agent_code: String,
    pub runtime_type: AgentRuntimeType,
    pub command_name: Option<String>,
    pub name: String,
}

const MAIN_AGENT_CODE: &str = "main-agent";

pub trait AgentManagementRepository: Send {
    fn agent_workspace_grants(&mut self, _agent: &str) -> Result<Option<Vec<String>>, AgentManagementError> { Ok(None) }
    fn replace_agent_workspace_grants(&mut self, _agent: &str, _workspaces: &[String]) -> Result<(), AgentManagementError> { Err(AgentManagementError::StorageFailure) }

    fn insert_platform_internal_agent_config(
        &mut self,
        config: &PlatformInternalAgentConfig,
    ) -> Result<(), AgentManagementError>;
    fn insert_workspace_internal_agent_definition(
        &mut self,
        definition: &WorkspaceInternalAgentDefinition,
    ) -> Result<(), AgentManagementError>;
    fn list_platform_internal_agent_configs(
        &mut self,
    ) -> Result<Vec<PlatformInternalAgentConfig>, AgentManagementError>;
    fn upsert_platform_internal_agent_config(
        &mut self,
        config: &PlatformInternalAgentConfig,
    ) -> Result<(), AgentManagementError>;
    fn delete_workspace_internal_agent_definition(
        &mut self,
        workspace: &str,
        agent: &str,
    ) -> Result<(), AgentManagementError>;
    fn list_workspace_internal_agent_definitions(
        &mut self,
        workspace_code: &str,
    ) -> Result<Vec<WorkspaceInternalAgentDefinition>, AgentManagementError>;
    fn upsert_workspace_internal_agent_definition(
        &mut self,
        definition: &WorkspaceInternalAgentDefinition,
    ) -> Result<(), AgentManagementError>;
    fn list_system_agents(&mut self)
    -> Result<Vec<SystemAgentRegistryEntry>, AgentManagementError>;
    fn upsert_system_agent(
        &mut self,
        agent: &SystemAgentRegistryEntry,
    ) -> Result<(), AgentManagementError>;
    fn delete_system_agent(&mut self, agent_code: &str) -> Result<bool, AgentManagementError>;
    fn list_workspace_system_policies(
        &mut self,
        workspace_code: Option<&str>,
    ) -> Result<Vec<WorkspaceSystemAgentPolicy>, AgentManagementError>;
    fn upsert_workspace_system_policy(
        &mut self,
        policy: &WorkspaceSystemAgentPolicy,
    ) -> Result<(), AgentManagementError>;
    fn delete_workspace_system_policy(
        &mut self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<bool, AgentManagementError>;
    fn list_workspace_internal_agent_policies(
        &mut self,
        workspace_code: Option<&str>,
    ) -> Result<Vec<WorkspaceInternalAgentPolicy>, AgentManagementError>;
    fn upsert_workspace_internal_agent_policy(
        &mut self,
        policy: &WorkspaceInternalAgentPolicy,
    ) -> Result<(), AgentManagementError>;
    fn delete_workspace_internal_agent_policy(
        &mut self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<bool, AgentManagementError>;
    fn list_workspace_internal_agent_tool_assignments(
        &mut self,
        workspace_code: Option<&str>,
        agent_code: Option<&str>,
    ) -> Result<Vec<WorkspaceInternalAgentToolAssignment>, AgentManagementError>;
    fn upsert_workspace_internal_agent_tool_assignment(
        &mut self,
        assignment: &WorkspaceInternalAgentToolAssignment,
    ) -> Result<(), AgentManagementError>;
    fn delete_workspace_internal_agent_tool_assignments(
        &mut self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<(), AgentManagementError>;
    fn list_workspace_internal_agent_mcp_server_assignments(
        &mut self,
        workspace_code: Option<&str>,
        agent_code: Option<&str>,
    ) -> Result<Vec<WorkspaceInternalAgentMcpServerAssignment>, AgentManagementError>;
    fn upsert_workspace_internal_agent_mcp_server_assignment(
        &mut self,
        assignment: &WorkspaceInternalAgentMcpServerAssignment,
    ) -> Result<(), AgentManagementError>;
    fn delete_workspace_internal_agent_mcp_server_assignments(
        &mut self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<(), AgentManagementError>;
}

#[derive(Debug, Clone)]
struct SupportedSystemAgentDefinition {
    agent_code: String,
    name: String,
    description: String,
    runtime_type: AgentRuntimeType,
    command_name: String,
}

#[derive(Clone)]
pub struct AgentManagementService {
    repository: Arc<Mutex<Box<dyn AgentManagementRepository>>>,
    workspaces: WorkspaceService,
    supported_agents: Vec<SupportedSystemAgentDefinition>,
    probe: Arc<dyn SystemCommandProbe>,
    model_catalog: Arc<Mutex<HashMap<String, Vec<SystemAgentModelRecord>>>>,
}

impl AgentManagementService {
    pub fn new(
        repository: impl AgentManagementRepository + 'static,
        workspaces: WorkspaceService,
        config: &SystemAgentConfig,
    ) -> Self {
        Self::with_probe(
            repository,
            workspaces,
            supported_system_agents_from_config(config),
            Arc::new(ProcessSystemCommandProbe),
        )
    }

    fn with_probe(
        repository: impl AgentManagementRepository + 'static,
        workspaces: WorkspaceService,
        supported_agents: Vec<SupportedSystemAgentDefinition>,
        probe: Arc<dyn SystemCommandProbe>,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            workspaces,
            supported_agents,
            probe,
            model_catalog: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn bootstrap_registry(&self) -> Result<SystemAgentSyncResult, AgentManagementError> {
        self.rescan_system_agents()
    }

    pub fn list_tool_domains(&self, locale: &str) -> Vec<ToolDomainRecord> {
        BusinessToolCatalog::domains(locale)
    }

    pub fn list_tools(
        &self,
        domain_code: Option<&str>,
        search: Option<&str>,
        locale: &str,
    ) -> Vec<ToolCatalogRecord> {
        BusinessToolCatalog::list_tools(domain_code, search, locale)
    }

    pub fn tool(&self, tool_code: &str, locale: &str) -> Option<ToolCatalogRecord> {
        BusinessToolCatalog::tool(tool_code, locale)
    }

    fn active_workspace_detail(
        &self,
        workspace_code: &str,
    ) -> Result<WorkspaceDetail, AgentManagementError> {
        let workspace = self
            .workspaces
            .get_workspace(workspace_code)
            .map_err(map_workspace_error)?;
        if workspace.status != WorkspaceStatus::Active {
            return Err(AgentManagementError::InvalidWorkspace);
        }
        Ok(workspace)
    }

    fn repository(&self) -> std::sync::MutexGuard<'_, Box<dyn AgentManagementRepository>> {
        self.repository
            .lock()
            .expect("agent management repository mutex poisoned")
    }

    fn system_agent_models(&self, agent_code: &str) -> Vec<SystemAgentModelRecord> {
        self.model_catalog
            .lock()
            .expect("agent management model catalog mutex poisoned")
            .get(agent_code)
            .cloned()
            .unwrap_or_default()
    }

    fn set_system_agent_models(&self, agent_code: &str, models: Vec<SystemAgentModelRecord>) {
        self.model_catalog
            .lock()
            .expect("agent management model catalog mutex poisoned")
            .insert(agent_code.to_string(), models);
    }
}

fn map_workspace_error(error: WorkspaceError) -> AgentManagementError {
    match error {
        WorkspaceError::NotFound | WorkspaceError::InvalidWorkspaceCode => {
            AgentManagementError::InvalidWorkspace
        }
        _ => AgentManagementError::StorageFailure,
    }
}
