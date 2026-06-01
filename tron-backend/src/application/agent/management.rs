use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use super::{
    runtime_capability::{build_runtime_capabilities, find_runtime_capability},
    system_agent_model_probe::{ProcessSystemCommandProbe, SystemCommandProbe},
};

use crate::{
    application::workspace::WorkspaceService,
    config::SystemAgentConfig,
    domain::{
        agent::{
            AgentManagementError, AgentRuntimeType, ManagedAgentSource, ManagedAgentType,
            ManagedSystemAgentRecord, PlatformMainAgentConfig, RuntimeAdapterCapabilityRecord,
            SystemAgentModelRecord, SystemAgentRegistryEntry, SystemAgentStatus,
            SystemAgentSyncResult, WorkspaceSystemAgentPolicy,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAgentRuntime {
    pub agent_code: String,
    pub runtime_type: AgentRuntimeType,
    pub command_name: Option<String>,
    pub name: String,
}

const MAIN_AGENT_CODE: &str = "main-agent";

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

    pub fn list_platform_system_agents(
        &self,
    ) -> Result<Vec<ManagedSystemAgentRecord>, AgentManagementError> {
        let mut entries = self.repository().list_system_agents()?;
        entries.sort_by(|left, right| left.agent_code.cmp(&right.agent_code));
        Ok(entries
            .into_iter()
            .map(|entry| {
                managed_system_record(&entry, None, self.system_agent_models(&entry.agent_code))
            })
            .collect())
    }

    pub fn rescan_system_agents(&self) -> Result<SystemAgentSyncResult, AgentManagementError> {
        let existing = self
            .repository()
            .list_system_agents()?
            .into_iter()
            .map(|entry| (entry.agent_code.clone(), entry))
            .collect::<HashMap<_, _>>();

        let now = current_timestamp();
        let mut available_count = 0usize;
        let mut unavailable_count = 0usize;

        for supported in &self.supported_agents {
            let existing_entry = existing.get(&supported.agent_code);
            let availability = self.probe.is_available(&supported.command_name);
            let mut last_error = None;
            let (available, availability_error) = match availability {
                Ok(true) => (true, None),
                Ok(false) => (false, None),
                Err(message) => (false, Some(message)),
            };
            if let Some(error) = availability_error {
                last_error = Some(error);
            }
            let models = if available {
                match self
                    .probe
                    .list_models(&supported.agent_code, &supported.command_name)
                {
                    Ok(models) => models,
                    Err(error) => {
                        last_error = Some(error);
                        Vec::new()
                    }
                }
            } else {
                Vec::new()
            };
            self.set_system_agent_models(&supported.agent_code, models);
            if available {
                available_count += 1;
            } else {
                unavailable_count += 1;
            }
            let entry = SystemAgentRegistryEntry {
                agent_code: supported.agent_code.clone(),
                name: supported.name.clone(),
                description: supported.description.clone(),
                runtime_type: supported.runtime_type,
                command_name: supported.command_name.clone(),
                source: ManagedAgentSource::System,
                enabled: existing_entry.map(|entry| entry.enabled).unwrap_or(true),
                available,
                last_scanned_at: Some(now.clone()),
                last_seen_at: if available {
                    Some(now.clone())
                } else {
                    existing_entry.and_then(|entry| entry.last_seen_at.clone())
                },
                last_error,
            };
            self.repository().upsert_system_agent(&entry)?;
        }

        let records = self.list_platform_system_agents()?;
        Ok(SystemAgentSyncResult {
            scanned_count: self.supported_agents.len(),
            available_count,
            unavailable_count,
            records,
        })
    }

    pub fn list_runtime_capabilities(
        &self,
    ) -> Result<Vec<RuntimeAdapterCapabilityRecord>, AgentManagementError> {
        let entries = self.repository().list_system_agents()?;
        Ok(build_runtime_capabilities(&entries))
    }

    pub fn runtime_capability(
        &self,
        adapter_key: &str,
    ) -> Result<Option<RuntimeAdapterCapabilityRecord>, AgentManagementError> {
        let entries = self.repository().list_system_agents()?;
        Ok(find_runtime_capability(&entries, adapter_key))
    }

    pub fn enable_platform_system_agent(
        &self,
        agent_code: &str,
    ) -> Result<ManagedSystemAgentRecord, AgentManagementError> {
        let mut entry = self.required_system_agent(agent_code)?;
        if !entry.available {
            return Err(AgentManagementError::SystemAgentUnavailable);
        }
        entry.enabled = true;
        self.repository().upsert_system_agent(&entry)?;
        Ok(managed_system_record(
            &entry,
            None,
            self.system_agent_models(&entry.agent_code),
        ))
    }

    pub fn disable_platform_system_agent(
        &self,
        agent_code: &str,
    ) -> Result<ManagedSystemAgentRecord, AgentManagementError> {
        let mut entry = self.required_system_agent(agent_code)?;
        if !entry.available {
            return Err(AgentManagementError::SystemAgentUnavailable);
        }
        entry.enabled = false;
        self.repository().upsert_system_agent(&entry)?;
        Ok(managed_system_record(
            &entry,
            None,
            self.system_agent_models(&entry.agent_code),
        ))
    }

    pub fn list_workspace_system_agents(
        &self,
        workspace_code: &str,
    ) -> Result<Vec<ManagedSystemAgentRecord>, AgentManagementError> {
        self.active_workspace_detail(workspace_code)?;
        let policies = self
            .repository()
            .list_workspace_system_policies(Some(workspace_code))?
            .into_iter()
            .map(|policy| (policy.agent_code.clone(), policy))
            .collect::<HashMap<_, _>>();
        let mut entries = self.repository().list_system_agents()?;
        entries.sort_by(|left, right| left.agent_code.cmp(&right.agent_code));
        Ok(entries
            .into_iter()
            .map(|entry| {
                let disabled = policies
                    .get(entry.agent_code.as_str())
                    .is_some_and(|policy| !policy.enabled);
                managed_system_record(
                    &entry,
                    Some(disabled),
                    self.system_agent_models(&entry.agent_code),
                )
            })
            .collect())
    }

    pub fn resolve_workspace_agent_runtime(
        &self,
        workspace_code: &str,
        requested_agent_code: Option<&str>,
    ) -> Result<ResolvedAgentRuntime, AgentManagementError> {
        self.active_workspace_detail(workspace_code)?;
        let normalized_agent_code = requested_agent_code
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(MAIN_AGENT_CODE);
        if normalized_agent_code.is_empty() {
            return Err(AgentManagementError::SystemAgentNotFound);
        }
        if normalized_agent_code.eq_ignore_ascii_case(MAIN_AGENT_CODE) {
            return Ok(ResolvedAgentRuntime {
                agent_code: MAIN_AGENT_CODE.to_string(),
                runtime_type: AgentRuntimeType::React,
                command_name: None,
                name: "Main Agent".to_string(),
            });
        }
        let entry = self.required_system_agent(normalized_agent_code)?;
        if !entry.available || !entry.enabled || entry.command_name.trim().is_empty() {
            return Err(AgentManagementError::SystemAgentUnavailable);
        }
        let workspace_override_disabled = self
            .repository()
            .list_workspace_system_policies(Some(workspace_code))?
            .into_iter()
            .find(|policy| policy.agent_code == entry.agent_code)
            .is_some_and(|policy| !policy.enabled);
        if workspace_override_disabled {
            return Err(AgentManagementError::SystemAgentUnavailable);
        }
        Ok(ResolvedAgentRuntime {
            agent_code: entry.agent_code,
            runtime_type: entry.runtime_type,
            command_name: Some(entry.command_name),
            name: entry.name,
        })
    }

    pub fn enable_workspace_system_agent(
        &self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<ManagedSystemAgentRecord, AgentManagementError> {
        self.active_workspace_detail(workspace_code)?;
        let entry = self.required_system_agent(agent_code)?;
        if !entry.available {
            return Err(AgentManagementError::SystemAgentUnavailable);
        }
        let _ = self
            .repository()
            .delete_workspace_system_policy(workspace_code, agent_code)?;
        Ok(self.workspace_system_agent_record(workspace_code, &entry))
    }

    pub fn disable_workspace_system_agent(
        &self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<ManagedSystemAgentRecord, AgentManagementError> {
        self.active_workspace_detail(workspace_code)?;
        let entry = self.required_system_agent(agent_code)?;
        if !entry.available {
            return Err(AgentManagementError::SystemAgentUnavailable);
        }
        let now = current_timestamp();
        self.repository()
            .upsert_workspace_system_policy(&WorkspaceSystemAgentPolicy {
                workspace_code: workspace_code.to_string(),
                agent_code: agent_code.to_string(),
                enabled: false,
                created_at: now.clone(),
                updated_at: now,
            })?;
        Ok(self.workspace_system_agent_record(workspace_code, &entry))
    }

    fn workspace_system_agent_record(
        &self,
        workspace_code: &str,
        entry: &SystemAgentRegistryEntry,
    ) -> ManagedSystemAgentRecord {
        let disabled = self
            .repository()
            .list_workspace_system_policies(Some(workspace_code))
            .ok()
            .and_then(|policies| {
                policies
                    .into_iter()
                    .find(|policy| policy.agent_code == entry.agent_code)
            })
            .is_some_and(|policy| !policy.enabled);
        managed_system_record(
            entry,
            Some(disabled),
            self.system_agent_models(&entry.agent_code),
        )
    }

    fn required_system_agent(
        &self,
        agent_code: &str,
    ) -> Result<SystemAgentRegistryEntry, AgentManagementError> {
        self.repository()
            .list_system_agents()?
            .into_iter()
            .find(|entry| entry.agent_code == agent_code.trim())
            .ok_or(AgentManagementError::SystemAgentNotFound)
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

fn managed_system_record(
    entry: &SystemAgentRegistryEntry,
    workspace_disabled_override: Option<bool>,
    models: Vec<SystemAgentModelRecord>,
) -> ManagedSystemAgentRecord {
    let effective_enabled =
        entry.available && entry.enabled && !workspace_disabled_override.unwrap_or(false);
    ManagedSystemAgentRecord {
        agent_code: entry.agent_code.clone(),
        name: entry.name.clone(),
        description: entry.description.clone(),
        agent_type: ManagedAgentType::System,
        runtime_type: entry.runtime_type,
        source: entry.source,
        inherited_from: workspace_disabled_override.map(|_| "Platform".to_string()),
        platform_enabled: workspace_disabled_override.map(|_| entry.enabled),
        workspace_override_disabled: workspace_disabled_override,
        status: if !entry.available {
            SystemAgentStatus::Unavailable
        } else if effective_enabled {
            SystemAgentStatus::Enabled
        } else {
            SystemAgentStatus::Disabled
        },
        enabled: effective_enabled,
        available: entry.available,
        models,
        last_scanned_at: entry.last_scanned_at.clone(),
        last_seen_at: entry.last_seen_at.clone(),
        last_error: entry.last_error.clone(),
    }
}

fn supported_system_agents_from_config(
    config: &SystemAgentConfig,
) -> Vec<SupportedSystemAgentDefinition> {
    vec![
        SupportedSystemAgentDefinition {
            agent_code: "codex".into(),
            name: "Codex".into(),
            description: "OpenAI Codex CLI agent runtime.".into(),
            runtime_type: AgentRuntimeType::CodexExec,
            command_name: config.codex_command.clone(),
        },
        SupportedSystemAgentDefinition {
            agent_code: "opencode".into(),
            name: "OpenCode".into(),
            description: "OpenCode CLI agent runtime.".into(),
            runtime_type: AgentRuntimeType::OpenCodeRun,
            command_name: config.opencode_command.clone(),
        },
    ]
}

fn map_workspace_error(error: WorkspaceError) -> AgentManagementError {
    match error {
        WorkspaceError::NotFound | WorkspaceError::InvalidWorkspaceCode => {
            AgentManagementError::InvalidWorkspace
        }
        _ => AgentManagementError::StorageFailure,
    }
}
