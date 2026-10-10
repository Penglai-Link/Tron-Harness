//! Groups organize definitions; they never participate in runtime or grants.
use super::*;

impl AgentManagementService {
    /// Empty scope denotes platform definitions, otherwise a validated workspace.
    pub fn list_internal_agent_groups(
        &self,
        scope: &str,
    ) -> Result<Vec<String>, AgentManagementError> {
        let agents = if scope.is_empty() {
            self.list_platform_internal_agents()?
        } else {
            self.active_workspace_detail(scope)?;
            self.list_workspace_internal_agents(scope)?
        };
        let mut names = self.repository().list_internal_agent_groups(scope)?;
        names.extend(agents.into_iter().filter_map(|agent| agent.group_name));
        names.sort();
        names.dedup();
        Ok(names)
    }

    pub fn create_internal_agent_group(
        &self,
        scope: &str,
        name: &str,
    ) -> Result<String, AgentManagementError> {
        if !scope.is_empty() {
            self.active_workspace_detail(scope)?;
        }
        let name = normalize_group_name(name)?;
        if name.is_empty() {
            return Err(AgentManagementError::InternalAgentGroupInvalid);
        }
        // Same-scope duplicates are idempotent, including concurrent requests.
        self.repository().save_internal_agent_group(scope, &name)?;
        Ok(name)
    }

    pub(super) fn validate_internal_agent_group(
        &self,
        scope: &str,
        name: Option<String>,
    ) -> Result<Option<String>, AgentManagementError> {
        let Some(name) = name else {
            return Ok(None);
        };
        let name = normalize_group_name(&name)?;
        if !name.is_empty() && !self.list_internal_agent_groups(scope)?.contains(&name) {
            return Err(AgentManagementError::InternalAgentGroupInvalid);
        }
        // Some("") deliberately overrides a platform group with "ungrouped".
        Ok(Some(name))
    }
}

fn normalize_group_name(name: &str) -> Result<String, AgentManagementError> {
    let name = name.trim();
    if name.chars().count() > 60 || name.chars().any(char::is_control) {
        return Err(AgentManagementError::InternalAgentGroupInvalid);
    }
    Ok(name.to_owned())
}
