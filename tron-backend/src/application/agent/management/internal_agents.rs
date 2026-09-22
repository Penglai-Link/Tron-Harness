use std::collections::HashMap;

use super::*;

#[derive(Debug, Clone)]
struct InternalAgentDefinition {
    agent_code: String,
    name: String,
    description: String,
    tool_assignment_mode: ToolAssignmentMode,
    mcp_access_mode: McpAccessMode,
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
        mcp_access_mode: McpAccessMode::AllEnabledServers,
        tool_exposure_mode: ToolExposureMode::DomainDiscovery,
        expanded_domain_persistence_mode: ExpandedDomainPersistenceMode::StickyExpanded,
        max_steps: 100,
        max_tool_calls: 100,
        max_parallel_tool_calls: 4,
    }]
}

impl AgentManagementService {
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
        let mcp_assignments = self
            .repository()
            .list_workspace_internal_agent_mcp_server_assignments(Some(workspace_code), None)?;
        let mut assigned_map = HashMap::<String, usize>::new();
        let mut selected_server_codes_by_agent = HashMap::<String, Vec<String>>::new();
        let registered = BusinessToolCatalog::tool_codes();
        for assignment in assignments {
            if registered.iter().any(|code| code == &assignment.tool_code) {
                *assigned_map.entry(assignment.agent_code).or_insert(0) += 1;
            }
        }
        for assignment in mcp_assignments {
            selected_server_codes_by_agent
                .entry(assignment.agent_code)
                .or_default()
                .push(assignment.server_code);
        }
        let mut policies = Vec::new();
        for definition in internal_agents() {
            let agent_code = definition.agent_code.clone();
            let override_policy = overrides.get(&definition.agent_code);
            let default_policy = to_internal_policy_record(workspace_code, &definition);
            let policy = override_policy
                .cloned()
                .unwrap_or_else(|| default_policy.clone());
            let mode = policy.tool_assignment_mode;
            let selected_tool_codes =
                self.resolve_internal_agent_tool_codes(workspace_code, &agent_code)?;
            let effective_tool_count = selected_tool_codes.len();
            let assigned_tool_count = if mode == ToolAssignmentMode::AllPlatformTools {
                registered.len()
            } else {
                *assigned_map.get(&agent_code).unwrap_or(&0)
            };
            policies.push(InternalAgentToolPolicy {
                workspace_code: workspace_code.to_string(),
                agent_code: agent_code.clone(),
                name: definition.name,
                description: definition.description,
                source: override_policy
                    .is_some()
                    .then_some(InternalAgentPolicySource::WorkspaceOverride)
                    .unwrap_or(InternalAgentPolicySource::PlatformDefault),
                tool_assignment_mode: mode,
                mcp_access_mode: policy.mcp_access_mode,
                tool_exposure_mode: policy.tool_exposure_mode,
                expanded_domain_persistence_mode: policy.expanded_domain_persistence_mode,
                max_steps: policy.max_steps,
                max_tool_calls: policy.max_tool_calls,
                max_parallel_tool_calls: policy.max_parallel_tool_calls,
                selected_tool_codes,
                selected_server_codes: normalize_string_codes(
                    selected_server_codes_by_agent
                        .remove(&agent_code)
                        .unwrap_or_default(),
                ),
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
            mcp_access_mode: current.mcp_access_mode,
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

    pub fn update_internal_agent_mcp_policy(
        &self,
        workspace_code: &str,
        agent_code: &str,
        command: UpdateWorkspaceInternalAgentMcpPolicyCommand,
    ) -> Result<InternalAgentToolPolicy, AgentManagementError> {
        self.active_workspace_detail(workspace_code)?;
        let definition = self
            .resolve_internal_agent_default(agent_code)
            .ok_or(AgentManagementError::InternalAgentNotFound)?;
        let current = self.workspace_internal_agent_policy(workspace_code, agent_code)?;
        let previous_created_at = self
            .repository()
            .list_workspace_internal_agent_policies(Some(workspace_code))?
            .into_iter()
            .find(|policy| policy.agent_code.eq_ignore_ascii_case(agent_code))
            .map(|policy| policy.created_at)
            .unwrap_or_else(current_timestamp);
        let updated = WorkspaceInternalAgentPolicy {
            workspace_code: workspace_code.to_string(),
            agent_code: definition.agent_code.clone(),
            tool_assignment_mode: current.tool_assignment_mode,
            mcp_access_mode: command.mcp_access_mode.unwrap_or(current.mcp_access_mode),
            tool_exposure_mode: current.tool_exposure_mode,
            expanded_domain_persistence_mode: current.expanded_domain_persistence_mode,
            max_steps: current.max_steps,
            max_tool_calls: current.max_tool_calls,
            max_parallel_tool_calls: current.max_parallel_tool_calls,
            created_at: previous_created_at,
            updated_at: current_timestamp(),
        };
        self.repository()
            .upsert_workspace_internal_agent_policy(&updated)?;
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
        self.repository()
            .delete_workspace_internal_agent_mcp_server_assignments(workspace_code, agent_code)?;
        self.workspace_internal_agent_policy(workspace_code, agent_code)
    }

    pub fn replace_internal_agent_mcp_server_assignments(
        &self,
        workspace_code: &str,
        agent_code: &str,
        mut server_codes: Vec<String>,
    ) -> Result<InternalAgentToolPolicy, AgentManagementError> {
        self.active_workspace_detail(workspace_code)?;
        let _ = self
            .resolve_internal_agent_default(agent_code)
            .ok_or(AgentManagementError::InternalAgentNotFound)?;
        let now = current_timestamp();
        let normalized = normalize_string_codes(server_codes.drain(..).collect());
        self.repository()
            .delete_workspace_internal_agent_mcp_server_assignments(workspace_code, agent_code)?;
        for server_code in normalized {
            self.repository()
                .upsert_workspace_internal_agent_mcp_server_assignment(
                    &WorkspaceInternalAgentMcpServerAssignment {
                        workspace_code: workspace_code.to_string(),
                        agent_code: agent_code.to_string(),
                        server_code,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                    },
                )?;
        }
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
        mcp_access_mode: definition.mcp_access_mode,
        tool_exposure_mode: definition.tool_exposure_mode,
        expanded_domain_persistence_mode: definition.expanded_domain_persistence_mode,
        max_steps: definition.max_steps,
        max_tool_calls: definition.max_tool_calls,
        max_parallel_tool_calls: definition.max_parallel_tool_calls,
        created_at: current_timestamp(),
        updated_at: current_timestamp(),
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
