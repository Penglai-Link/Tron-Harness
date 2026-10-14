use super::*;

impl AgentManagementService {
    pub fn agent_workspace_grants(&self, agent: &str) -> Result<Vec<String>, AgentManagementError> {
        self.require_platform_agent(agent)?;
        if let Some(grants) = self.repository().agent_workspace_grants(agent)? {
            return Ok(grants);
        }
        self.workspaces
            .list_workspace_options(crate::domain::workspace::WorkspaceListQuery {
                include_archived: Some(false),
                search: None,
                status: Some(WorkspaceStatus::Active),
                root_only: None,
            })
            .map(|options| {
                options
                    .into_iter()
                    .map(|option| option.workspace_code)
                    .collect()
            })
            .map_err(|_| AgentManagementError::StorageFailure)
    }

    pub fn assign_agent_workspaces(
        &self,
        agent: &str,
        workspaces: Vec<String>,
    ) -> Result<(), AgentManagementError> {
        self.require_platform_agent(agent)?;
        // The built-in main Agent remains available in every space.
        if agent == MAIN_AGENT_CODE {
            return Err(AgentManagementError::InternalAgentPolicyInvalid);
        }
        let workspaces = internal_agents::normalize_string_codes(workspaces);
        for workspace in &workspaces {
            self.active_workspace_detail(workspace)?;
        }
        self.repository()
            .replace_agent_workspace_grants(agent, &workspaces)
    }

    pub(super) fn assigned_agent_codes(
        &self,
        workspace: &str,
        agents: &[String],
    ) -> Result<std::collections::HashSet<String>, AgentManagementError> {
        let mut assigned = self
            .repository()
            .assigned_agent_codes(workspace, agents)?
            .into_iter()
            .collect::<std::collections::HashSet<_>>();
        if agents.iter().any(|agent| agent == MAIN_AGENT_CODE) {
            assigned.insert(MAIN_AGENT_CODE.to_owned());
        }
        Ok(assigned)
    }

    pub(super) fn agent_assigned(
        &self,
        agent: &str,
        workspace: &str,
    ) -> Result<bool, AgentManagementError> {
        if agent == MAIN_AGENT_CODE {
            return Ok(true);
        }
        Ok(self
            .repository()
            .agent_workspace_grants(agent)?
            .is_none_or(|grants| grants.iter().any(|code| code == workspace)))
    }

    fn require_platform_agent(&self, agent: &str) -> Result<(), AgentManagementError> {
        if self
            .platform_internal_agent_configs()?
            .iter()
            .any(|config| config.agent_code == agent)
            || self
                .repository()
                .list_system_agents()?
                .iter()
                .any(|record| record.agent_code == agent)
        {
            return Ok(());
        }
        Err(AgentManagementError::InternalAgentNotFound)
    }
}
