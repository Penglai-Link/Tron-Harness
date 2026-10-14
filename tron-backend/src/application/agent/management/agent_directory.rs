//! Lightweight authorized identities: no tool or MCP policy assembly on list reads.
use super::*;

pub(crate) struct InternalAgentDirectoryEntry {
    pub agent_code: String,
    pub name: String,
    pub responsibility: String,
    pub avatar_id: String,
}

fn responsibility(description: &str) -> String {
    description.trim().chars().take(200).collect()
}

impl AgentManagementService {
    pub(crate) fn internal_agent_directory(
        &self,
        workspace: &str,
    ) -> Result<Vec<InternalAgentDirectoryEntry>, AgentManagementError> {
        let dudes = self.repository().list_dude_codes()?;
        if workspace.is_empty() {
            return Ok(self
                .platform_internal_agent_configs()?
                .into_iter()
                .filter(|agent| agent.enabled && !dudes.contains(&agent.agent_code))
                .map(|agent| InternalAgentDirectoryEntry {
                    responsibility: responsibility(&agent.description),
                    agent_code: agent.agent_code,
                    name: agent.name,
                    avatar_id: agent.avatar_id,
                })
                .collect());
        }
        self.active_workspace_detail(workspace)?;
        Ok(self
            .internal_agent_definitions(workspace)?
            .into_iter()
            .filter(|agent| agent.enabled && !dudes.contains(&agent.agent_code))
            .map(|agent| InternalAgentDirectoryEntry {
                responsibility: responsibility(&agent.description),
                agent_code: agent.agent_code,
                name: agent.name,
                avatar_id: agent.avatar_id,
            })
            .collect())
    }
}
