mod system_agents;

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
            ManagedAgentType, ManagedSystemAgentRecord, PlatformMainAgentConfig,
            RuntimeAdapterCapabilityRecord, SystemAgentModelRecord, SystemAgentRegistryEntry,
            SystemAgentStatus, SystemAgentSyncResult, ToolAssignmentMode, ToolCatalogRecord,
            ToolDomainRecord, ToolExposureMode, WorkspaceInternalAgentPolicy,
            WorkspaceInternalAgentToolAssignment, WorkspaceSystemAgentPolicy,
        },
        workspace::{WorkspaceDetail, WorkspaceError, WorkspaceStatus},
    },
    shared::time::current_timestamp,
};

#[derive(Debug, Clone, Default)]
pub struct UpdatePlatformMainAgentCommand {
    pub personality: Option<String>,
    pub instructions: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateWorkspaceInternalAgentPolicyCommand {
    pub tool_assignment_mode: Option<ToolAssignmentMode>,
    pub tool_exposure_mode: Option<ToolExposureMode>,
    pub expanded_domain_persistence_mode: Option<ExpandedDomainPersistenceMode>,
    pub max_steps: Option<u32>,
    pub max_tool_calls: Option<u32>,
    pub max_parallel_tool_calls: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAgentRuntime {
    pub agent_code: String,
    pub runtime_type: AgentRuntimeType,
    pub command_name: Option<String>,
    pub name: String,
}

const MAIN_AGENT_CODE: &str = "main-agent";

#[derive(Debug, Clone)]
struct InternalAgentDefinition {
    agent_code: String,
    name: String,
    description: String,
    tool_assignment_mode: ToolAssignmentMode,
    tool_exposure_mode: ToolExposureMode,
    expanded_domain_persistence_mode: ExpandedDomainPersistenceMode,
    max_steps: u32,
    max_tool_calls: u32,
    max_parallel_tool_calls: u32,
}

fn internal_agents() -> Vec<InternalAgentDefinition> {
    vec![InternalAgentDefinition {
        agent_code: MAIN_AGENT_CODE.to_string(),
        name: "Main Agent".to_string(),
        description: "Primary workspace internal agent with access to platform business tools."
            .to_string(),
        tool_assignment_mode: ToolAssignmentMode::AllPlatformTools,
        tool_exposure_mode: ToolExposureMode::DirectList,
        expanded_domain_persistence_mode: ExpandedDomainPersistenceMode::StickyExpanded,
        max_steps: 8,
        max_tool_calls: 100,
        max_parallel_tool_calls: 4,
    }]
}

pub trait AgentManagementRepository: Send {
    fn get_main_agent_config(
        &mut self,
    ) -> Result<Option<PlatformMainAgentConfig>, AgentManagementError>;
    fn upsert_main_agent_config(
        &mut self,
        config: &PlatformMainAgentConfig,
    ) -> Result<(), AgentManagementError>;
    fn list_system_agents(&mut self)
    -> Result<Vec<SystemAgentRegistryEntry>, AgentManagementError>;
    fn upsert_system_agent(
        &mut self,
        agent: &SystemAgentRegistryEntry,
    ) -> Result<(), AgentManagementError>;
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

    pub fn get_main_agent_config(&self) -> Result<PlatformMainAgentConfig, AgentManagementError> {
        Ok(self
            .repository()
            .get_main_agent_config()?
            .unwrap_or_default())
    }

    pub fn update_main_agent_config(
        &self,
        command: UpdatePlatformMainAgentCommand,
    ) -> Result<PlatformMainAgentConfig, AgentManagementError> {
        let current = self.get_main_agent_config()?;
        let now = current_timestamp();
        let config = PlatformMainAgentConfig {
            personality: command
                .personality
                .unwrap_or(current.personality)
                .trim()
                .to_string(),
            instructions: command
                .instructions
                .unwrap_or(current.instructions)
                .trim()
                .to_string(),
            created_at: current.created_at.or_else(|| Some(now.clone())),
            updated_at: Some(now),
        };
        self.repository().upsert_main_agent_config(&config)?;
        Ok(config)
    }

    pub fn current_main_agent_guidance(&self) -> Result<Option<String>, AgentManagementError> {
        let config = self.get_main_agent_config()?;
        let mut sections = Vec::new();
        if !config.personality.trim().is_empty() {
            sections.push(format!("Personality:\n{}", config.personality.trim()));
        }
        if !config.instructions.trim().is_empty() {
            sections.push(format!("Instructions:\n{}", config.instructions.trim()));
        }
        if sections.is_empty() {
            return Ok(None);
        }
        Ok(Some(format!(
            "You are Tron Main Agent.\n\n{}",
            sections.join("\n\n")
        )))
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

    pub fn list_workspace_internal_agents(
        &self,
        workspace_code: &str,
    ) -> Result<Vec<InternalAgentToolPolicy>, AgentManagementError> {
        self.active_workspace_detail(workspace_code)?;
        let overrides = self
            .repository()
            .list_workspace_internal_agent_policies(Some(workspace_code))?
            .into_iter()
            .map(|policy| (policy.agent_code.clone(), policy))
            .collect::<HashMap<_, _>>();
        let assignments = self
            .repository()
            .list_workspace_internal_agent_tool_assignments(Some(workspace_code), None)?;
        let mut assigned_map = HashMap::<String, usize>::new();
        let registered = BusinessToolCatalog::tool_codes();
        for assignment in assignments {
            if registered.iter().any(|code| code == &assignment.tool_code) {
                *assigned_map.entry(assignment.agent_code).or_insert(0) += 1;
            }
        }
        let mut policies = Vec::new();
        for definition in internal_agents() {
            let override_policy = overrides.get(&definition.agent_code);
            let default_policy = to_internal_policy_record(workspace_code, &definition);
            let policy = override_policy
                .cloned()
                .unwrap_or_else(|| default_policy.clone());
            let mode = policy.tool_assignment_mode;
            let selected_tool_codes =
                self.resolve_internal_agent_tool_codes(workspace_code, &definition.agent_code)?;
            let effective_tool_count = selected_tool_codes.len();
            let assigned_tool_count = if mode == ToolAssignmentMode::AllPlatformTools {
                registered.len()
            } else {
                *assigned_map.get(&definition.agent_code).unwrap_or(&0)
            };
            policies.push(InternalAgentToolPolicy {
                workspace_code: workspace_code.to_string(),
                agent_code: definition.agent_code,
                name: definition.name,
                description: definition.description,
                source: override_policy
                    .is_some()
                    .then_some(InternalAgentPolicySource::WorkspaceOverride)
                    .unwrap_or(InternalAgentPolicySource::PlatformDefault),
                tool_assignment_mode: mode,
                tool_exposure_mode: policy.tool_exposure_mode,
                expanded_domain_persistence_mode: policy.expanded_domain_persistence_mode,
                max_steps: policy.max_steps,
                max_tool_calls: policy.max_tool_calls,
                max_parallel_tool_calls: policy.max_parallel_tool_calls,
                selected_tool_codes,
                assigned_tool_count,
                effective_tool_count,
            })
        }
        Ok(policies)
    }

    pub fn workspace_internal_agent_policy(
        &self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<InternalAgentToolPolicy, AgentManagementError> {
        self.active_workspace_detail(workspace_code)?;
        self.list_workspace_internal_agents(workspace_code)?
            .into_iter()
            .find(|policy| policy.agent_code.eq_ignore_ascii_case(agent_code))
            .ok_or(AgentManagementError::InternalAgentNotFound)
    }

    pub fn update_internal_agent_policy(
        &self,
        workspace_code: &str,
        agent_code: &str,
        command: UpdateWorkspaceInternalAgentPolicyCommand,
    ) -> Result<InternalAgentToolPolicy, AgentManagementError> {
        self.active_workspace_detail(workspace_code)?;
        let definition = self
            .resolve_internal_agent_default(agent_code)
            .ok_or(AgentManagementError::InternalAgentNotFound)?;
        let current = self.workspace_internal_agent_policy(workspace_code, agent_code)?;
        let now = current_timestamp();
        let updated = WorkspaceInternalAgentPolicy {
            workspace_code: workspace_code.to_string(),
            agent_code: definition.agent_code.clone(),
            tool_assignment_mode: command
                .tool_assignment_mode
                .unwrap_or(current.tool_assignment_mode),
            tool_exposure_mode: command
                .tool_exposure_mode
                .unwrap_or(current.tool_exposure_mode),
            expanded_domain_persistence_mode: command
                .expanded_domain_persistence_mode
                .unwrap_or(current.expanded_domain_persistence_mode),
            max_steps: command.max_steps.unwrap_or(current.max_steps),
            max_tool_calls: command.max_tool_calls.unwrap_or(current.max_tool_calls),
            max_parallel_tool_calls: command
                .max_parallel_tool_calls
                .unwrap_or(current.max_parallel_tool_calls),
            created_at: current_timestamp(),
            updated_at: now,
        };
        let previous_created_at = self
            .repository()
            .list_workspace_internal_agent_policies(Some(workspace_code))?
            .into_iter()
            .find(|policy| policy.agent_code.eq_ignore_ascii_case(agent_code))
            .map(|policy| policy.created_at)
            .unwrap_or_else(|| updated.created_at.clone());
        let updated_policy = WorkspaceInternalAgentPolicy {
            created_at: previous_created_at,
            ..updated
        };
        self.repository()
            .upsert_workspace_internal_agent_policy(&updated_policy)?;
        self.workspace_internal_agent_policy(workspace_code, agent_code)
    }

    pub fn replace_internal_agent_tool_assignments(
        &self,
        workspace_code: &str,
        agent_code: &str,
        mut tool_codes: Vec<String>,
    ) -> Result<InternalAgentToolPolicy, AgentManagementError> {
        self.active_workspace_detail(workspace_code)?;
        let _ = self
            .resolve_internal_agent_default(agent_code)
            .ok_or(AgentManagementError::InternalAgentNotFound)?;
        let now = current_timestamp();
        let mut normalized = Vec::new();
        for tool_code in tool_codes.drain(..) {
            let trimmed = tool_code.trim().to_string();
            if !trimmed.is_empty() && !normalized.iter().any(|existing| existing == &trimmed) {
                let Some(tool_definition) = BusinessToolCatalog::tool(&trimmed, "en-US") else {
                    return Err(AgentManagementError::InternalToolUnknown(trimmed));
                };
                if tool_definition.availability_state
                    != crate::domain::agent::ToolAvailabilityState::Available
                {
                    return Err(AgentManagementError::InternalToolAssignmentStale(trimmed));
                }
                normalized.push(trimmed);
            }
        }
        self.repository()
            .delete_workspace_internal_agent_tool_assignments(workspace_code, agent_code)?;
        for tool_code in normalized {
            self.repository()
                .upsert_workspace_internal_agent_tool_assignment(
                    &WorkspaceInternalAgentToolAssignment {
                        workspace_code: workspace_code.to_string(),
                        agent_code: agent_code.to_string(),
                        tool_code,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                    },
                )?;
        }
        self.workspace_internal_agent_policy(workspace_code, agent_code)
    }

    pub fn reset_internal_agent_policy(
        &self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<InternalAgentToolPolicy, AgentManagementError> {
        self.active_workspace_detail(workspace_code)?;
        let _ = self
            .resolve_internal_agent_default(agent_code)
            .ok_or(AgentManagementError::InternalAgentNotFound)?;
        let _ = self
            .repository()
            .delete_workspace_internal_agent_policy(workspace_code, agent_code)?;
        self.repository()
            .delete_workspace_internal_agent_tool_assignments(workspace_code, agent_code)?;
        self.workspace_internal_agent_policy(workspace_code, agent_code)
    }

    pub fn resolve_internal_agent_effective_tool_definitions(
        &self,
        workspace_code: &str,
        agent_code: &str,
        locale: &str,
    ) -> Result<Vec<ToolCatalogRecord>, AgentManagementError> {
        self.workspace_internal_agent_policy(workspace_code, agent_code)?;
        let tool_codes = self.resolve_internal_agent_tool_codes(workspace_code, agent_code)?;
        Ok(tool_codes
            .into_iter()
            .filter_map(|tool_code| BusinessToolCatalog::tool(&tool_code, locale))
            .collect())
    }

    fn resolve_internal_agent_tool_codes(
        &self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<Vec<String>, AgentManagementError> {
        let definition = self
            .resolve_internal_agent_default(agent_code)
            .ok_or(AgentManagementError::InternalAgentNotFound)?;
        let mode = self
            .repository()
            .list_workspace_internal_agent_policies(Some(workspace_code))?
            .into_iter()
            .find(|policy| {
                policy
                    .agent_code
                    .eq_ignore_ascii_case(&definition.agent_code)
            })
            .map(|policy| policy.tool_assignment_mode)
            .unwrap_or(definition.tool_assignment_mode);

        let selected = match mode {
            ToolAssignmentMode::AllPlatformTools => BusinessToolCatalog::tool_codes(),
            ToolAssignmentMode::SelectedTools => self
                .repository()
                .list_workspace_internal_agent_tool_assignments(
                    Some(workspace_code),
                    Some(agent_code),
                )?
                .into_iter()
                .map(|assignment| assignment.tool_code)
                .collect(),
        };
        let registered = BusinessToolCatalog::tool_codes();
        let mut filtered = selected
            .into_iter()
            .map(|code| code.trim().to_string())
            .filter(|code| !code.is_empty())
            .filter(|code| registered.iter().any(|registered| registered == code))
            .collect::<Vec<_>>();
        filtered.sort_unstable();
        filtered.dedup();
        Ok(filtered)
    }

    fn resolve_internal_agent_default(&self, agent_code: &str) -> Option<InternalAgentDefinition> {
        internal_agents_by_code(agent_code)
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

fn internal_agents_by_code(agent_code: &str) -> Option<InternalAgentDefinition> {
    internal_agents()
        .into_iter()
        .find(|definition| definition.agent_code.eq_ignore_ascii_case(agent_code))
}

fn to_internal_policy_record(
    workspace_code: &str,
    definition: &InternalAgentDefinition,
) -> WorkspaceInternalAgentPolicy {
    WorkspaceInternalAgentPolicy {
        workspace_code: workspace_code.to_string(),
        agent_code: definition.agent_code.clone(),
        tool_assignment_mode: definition.tool_assignment_mode,
        tool_exposure_mode: definition.tool_exposure_mode,
        expanded_domain_persistence_mode: definition.expanded_domain_persistence_mode,
        max_steps: definition.max_steps,
        max_tool_calls: definition.max_tool_calls,
        max_parallel_tool_calls: definition.max_parallel_tool_calls,
        created_at: current_timestamp(),
        updated_at: current_timestamp(),
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
