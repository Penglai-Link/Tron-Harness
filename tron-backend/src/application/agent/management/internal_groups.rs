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

    pub fn delete_internal_agent_group(
        &self,
        scope: &str,
        name: &str,
    ) -> Result<(), AgentManagementError> {
        if !scope.is_empty() {
            self.active_workspace_detail(scope)?;
        }
        let name = normalize_group_name(name)?;
        if name.is_empty() || !self.list_internal_agent_groups(scope)?.contains(&name) {
            return Err(AgentManagementError::InternalAgentGroupInvalid);
        }

        if scope.is_empty() {
            // Group membership is stored on the platform definition. Persisting the
            // ungrouped value also handles built-ins whose initial group is code-defined.
            for mut config in self.platform_internal_agent_configs()? {
                if config.group_name.as_deref() == Some(name.as_str()) {
                    config.group_name = None;
                    self.repository()
                        .upsert_platform_internal_agent_config(&config)?;
                }
            }
        } else {
            // An inherited platform agent needs a workspace override so that deleting
            // this workspace's group does not make the inherited group reappear.
            let members = self
                .list_workspace_internal_agents(scope)?
                .into_iter()
                .filter(|agent| agent.group_name.as_deref() == Some(name.as_str()))
                .map(|agent| agent.agent_code)
                .collect::<Vec<_>>();
            let saved = self
                .repository()
                .list_workspace_internal_agent_definitions(scope)?;
            let now = current_timestamp();
            for agent_code in members {
                let mut definition = saved
                    .iter()
                    .find(|record| record.agent_code == agent_code)
                    .cloned()
                    .unwrap_or_else(|| WorkspaceInternalAgentDefinition {
                        workspace_code: scope.to_owned(),
                        agent_code,
                        name: None,
                        role_prompt: None,
                        loop_code: None,
                        avatar_id: None,
                        candidate_model_codes: None,
                        group_name: None,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                    });
                definition.group_name = Some(String::new());
                definition.updated_at = now.clone();
                self.repository()
                    .upsert_workspace_internal_agent_definition(&definition)?;
            }
        }

        self.repository()
            .delete_internal_agent_group(scope, &name)?;
        Ok(())
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
