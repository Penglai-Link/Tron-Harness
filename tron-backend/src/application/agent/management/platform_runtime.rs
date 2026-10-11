//! Platform execution resolves independent agent definitions and platform policy only.
use super::*;

impl AgentManagementService {
    pub fn platform_internal_agent_policy(
        &self,
        code: &str,
    ) -> Result<InternalAgentToolPolicy, AgentManagementError> {
        self.all_platform_internal_agents()?
            .into_iter()
            .find(|policy| policy.agent_code.eq_ignore_ascii_case(code))
            .ok_or(AgentManagementError::InternalAgentNotFound)
    }

    pub fn resolve_platform_agent_runtime(
        &self,
        requested: Option<&str>,
    ) -> Result<ResolvedAgentRuntime, AgentManagementError> {
        let code = requested
            .map(str::trim)
            .filter(|code| !code.is_empty())
            .ok_or(AgentManagementError::InternalAgentNotFound)?;
        let policy = self.platform_internal_agent_policy(code)?;
        Ok(ResolvedAgentRuntime {
            agent_code: policy.agent_code,
            runtime_type: AgentRuntimeType::React,
            command_name: None,
            name: policy.name,
        })
    }

    pub fn resolve_platform_agent_effective_tool_definitions(
        &self,
        code: &str,
        locale: &str,
    ) -> Result<Vec<ToolCatalogRecord>, AgentManagementError> {
        let policy = self.platform_internal_agent_policy(code)?;
        Ok(policy
            .selected_tool_codes
            .into_iter()
            .filter_map(|code| self.tool(&code, locale))
            .filter(|tool| {
                tool.availability_state == crate::domain::agent::ToolAvailabilityState::Available
            })
            .collect())
    }
}
