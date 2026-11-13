use std::collections::{HashMap, HashSet};

use super::*;

pub(super) fn supported_system_agents_from_config(
    config: &SystemAgentConfig,
) -> Vec<SupportedSystemAgentDefinition> {
    vec![
        SupportedSystemAgentDefinition {
            agent_code: "dsh".into(),
            name: "DeepSeek Harness".into(),
            description: "DeepSeek Harness CLI agent runtime.".into(),
            runtime_type: AgentRuntimeType::DshHeadless,
            command_name: resolve_dsh_command(&config.dsh_command),
        },
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
    .into_iter()
    .filter(|agent| agent.agent_code != "dsh" || !agent.command_name.trim().is_empty())
    .collect()
}

impl AgentManagementService {
    pub fn list_platform_system_agents(
        &self,
    ) -> Result<Vec<ManagedSystemAgentRecord>, AgentManagementError> {
        let mut entries = self.repository().list_system_agents()?;
        entries.sort_by(|left, right| left.agent_code.cmp(&right.agent_code));
        Ok(entries
            .into_iter()
            .map(|entry| {
                managed_system_record(
                    &entry,
                    None,
                    self.system_agent_models(
                        entry
                            .base_agent_code
                            .as_deref()
                            .unwrap_or(&entry.agent_code),
                    ),
                )
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
        let mut discovered_agent_codes = HashSet::new();

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
            if !available {
                unavailable_count += 1;
                self.set_system_agent_models(&supported.agent_code, Vec::new());
                self.repository()
                    .delete_system_agent(&supported.agent_code)?;
                continue;
            }

            let models = match self
                .probe
                .list_models(&supported.agent_code, &supported.command_name)
            {
                Ok(models) => models,
                Err(error) => {
                    last_error = Some(error);
                    Vec::new()
                }
            };
            self.set_system_agent_models(&supported.agent_code, models);
            available_count += 1;
            discovered_agent_codes.insert(supported.agent_code.clone());
            let entry = SystemAgentRegistryEntry {
                fast_mode: existing_entry.is_some_and(|entry| entry.fast_mode),
                reasoning_effort: existing_entry.and_then(|entry| entry.reasoning_effort),
                avatar_id: existing_entry.and_then(|entry| entry.avatar_id.clone()),
                agent_code: supported.agent_code.clone(),
                base_agent_code: None,
                model_id: None,
                name: supported.name.clone(),
                description: existing_entry
                    .map(|entry| entry.description.clone())
                    .unwrap_or_else(|| supported.description.clone()),
                runtime_type: supported.runtime_type,
                command_name: supported.command_name.clone(),
                source: ManagedAgentSource::System,
                enabled: existing_entry.map(|entry| entry.enabled).unwrap_or(true),
                available: true,
                last_scanned_at: Some(now.clone()),
                last_seen_at: Some(now.clone()),
                last_error,
            };
            self.repository().upsert_system_agent(&entry)?;
        }

        let mut repository = self.repository();
        let mut refreshed = repository
            .list_system_agents()?
            .into_iter()
            .map(|entry| (entry.agent_code.clone(), entry))
            .collect::<HashMap<_, _>>();
        for (agent_code, old_entry) in &existing {
            if let Some(base) = old_entry.base_agent_code.as_deref() {
                let mut entry = old_entry.clone();
                let supported = self
                    .supported_agents
                    .iter()
                    .find(|agent| agent.agent_code == base);
                let base_entry = refreshed.get(base);
                entry.available = base_entry.is_some_and(|agent| agent.available);
                entry.last_scanned_at = Some(now.clone());
                entry.last_error = base_entry.and_then(|agent| agent.last_error.clone());
                if let Some(supported) = supported {
                    entry.command_name = supported.command_name.clone();
                    entry.runtime_type = supported.runtime_type;
                }
                if entry.available {
                    entry.last_seen_at = Some(now.clone());
                }
                repository.upsert_system_agent(&entry)?;
                refreshed.insert(agent_code.clone(), entry);
                continue;
            }
            if !discovered_agent_codes.contains(agent_code) {
                self.set_system_agent_models(agent_code, Vec::new());
                repository.delete_system_agent(agent_code)?;
                refreshed.remove(agent_code);
            }
        }

        drop(repository);

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
            self.system_agent_models(
                entry
                    .base_agent_code
                    .as_deref()
                    .unwrap_or(&entry.agent_code),
            ),
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
            self.system_agent_models(
                entry
                    .base_agent_code
                    .as_deref()
                    .unwrap_or(&entry.agent_code),
            ),
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
        let entries = self.repository().list_system_agents()?;
        let codes = entries
            .iter()
            .map(|entry| entry.agent_code.clone())
            .collect::<Vec<_>>();
        let allowed = self.assigned_agent_codes(workspace_code, &codes)?;
        let mut assigned = Vec::new();
        for entry in entries {
            if allowed.contains(&entry.agent_code) {
                assigned.push(entry);
            }
        }
        let mut entries = assigned;
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
                    self.system_agent_models(
                        entry
                            .base_agent_code
                            .as_deref()
                            .unwrap_or(&entry.agent_code),
                    ),
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
        if let Some(definition) =
            self.resolve_internal_agent_default(workspace_code, normalized_agent_code)?
        {
            if !definition.enabled {
                return Err(AgentManagementError::SystemAgentUnavailable);
            }
            return Ok(ResolvedAgentRuntime {
                system_fast_mode: false,
                system_reasoning_effort: None,
                agent_code: definition.agent_code,
                system_model_id: None,
                runtime_type: AgentRuntimeType::React,
                command_name: None,
                name: definition.name,
            });
        }
        if !self.agent_assigned(normalized_agent_code, workspace_code)? {
            return Err(AgentManagementError::SystemAgentUnavailable);
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
            system_fast_mode: entry.fast_mode,
            system_reasoning_effort: entry.reasoning_effort,
            agent_code: entry.agent_code,
            system_model_id: entry.model_id,
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
            self.system_agent_models(
                entry
                    .base_agent_code
                    .as_deref()
                    .unwrap_or(&entry.agent_code),
            ),
        )
    }

    pub(super) fn required_system_agent(
        &self,
        agent_code: &str,
    ) -> Result<SystemAgentRegistryEntry, AgentManagementError> {
        self.repository()
            .list_system_agents()?
            .into_iter()
            .find(|entry| entry.agent_code == agent_code.trim())
            .ok_or(AgentManagementError::SystemAgentNotFound)
    }
}

fn managed_system_record(
    entry: &SystemAgentRegistryEntry,
    workspace_disabled_override: Option<bool>,
    mut models: Vec<SystemAgentModelRecord>,
) -> ManagedSystemAgentRecord {
    if let Some(model_id) = entry.model_id.as_deref() {
        models.retain(|model| model.model_id == model_id);
        if models.is_empty() {
            models.push(SystemAgentModelRecord {
                supported_reasoning_efforts: Vec::new(),
                model_id: model_id.into(),
                display_name: model_id.into(),
                context_window_tokens: None,
            });
        }
    }
    let effective_enabled =
        entry.available && entry.enabled && !workspace_disabled_override.unwrap_or(false);
    ManagedSystemAgentRecord {
        agent_code: entry.agent_code.clone(),
        base_agent_code: entry.base_agent_code.clone(),
        model_id: entry.model_id.clone(),
        fast_mode: entry.fast_mode,
        reasoning_effort: entry.reasoning_effort,
        avatar_id: entry.avatar_id.clone(),
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

fn resolve_dsh_command(command: &str) -> String {
    if command != "dsh" {
        return command.into();
    }
    if let Ok(output) = std::process::Command::new(if cfg!(windows) { "where" } else { "which" })
        .arg(command)
        .output()
    {
        if output.status.success() {
            if let Some(path) = String::from_utf8_lossy(&output.stdout)
                .lines()
                .find(|path| !path.trim().is_empty())
            {
                return path.trim().into();
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let suffix = "DeepSeek Harness.app/Contents/Resources/runtime/cli/bin/dsh";
        let mut roots = vec![std::path::PathBuf::from("/Applications")];
        if let Some(home) = std::env::var_os("HOME") {
            roots.push(std::path::PathBuf::from(home).join("Applications"));
        }
        for root in roots {
            let path = root.join(suffix);
            if path.is_file() {
                return path.to_string_lossy().into_owned();
            }
        }
    }
    command.into()
}
