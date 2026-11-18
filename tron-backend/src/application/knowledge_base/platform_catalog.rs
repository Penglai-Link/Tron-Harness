use super::*;

impl KnowledgeBaseService {
    pub fn platform_libraries(&self, user: &str) -> Result<Vec<KnowledgeBase>, KnowledgeBaseError> {
        if user.trim().is_empty() {
            return Err(KnowledgeBaseError::Forbidden);
        }
        self.repository.platform_library_summaries(user)
    }

    pub fn library_agent_grants(
        &self,
        user: &str,
        id: &str,
    ) -> Result<Vec<String>, KnowledgeBaseError> {
        let library = self.repository.get(id)?;
        if library.owner_user_code != user {
            return Err(KnowledgeBaseError::NotFound);
        }
        self.repository.grants(id)
    }

    pub fn assign_library_agents(
        &self,
        user: &str,
        id: &str,
        agents: &[String],
    ) -> Result<(), KnowledgeBaseError> {
        self.library_agent_grants(user, id)?;
        let valid = self
            .agents
            .all_platform_internal_agents()
            .map_err(|e| KnowledgeBaseError::storage_detail(format!("{e:?}")))?;
        if agents
            .iter()
            .any(|code| !valid.iter().any(|agent| agent.agent_code == *code))
        {
            return Err(KnowledgeBaseError::invalid(
                "knowledge.base.internal_agent_only",
            ));
        }
        self.repository
            .replace_library_agent_grants(user, id, agents)
    }
}
