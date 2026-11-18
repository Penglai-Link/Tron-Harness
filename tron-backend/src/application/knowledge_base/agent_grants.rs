use super::{KnowledgeBase, KnowledgeBaseError, KnowledgeBaseService};

impl KnowledgeBaseService {
    fn validate_grant_agent(&self, user: &str, agent: &str) -> Result<(), KnowledgeBaseError> {
        self.require_user(user)?;
        let agents = self
            .agents
            .all_platform_internal_agents()
            .map_err(|error| KnowledgeBaseError::storage_detail(format!("{error:?}")))?;
        if !agents.iter().any(|record| record.agent_code == agent) {
            return Err(KnowledgeBaseError::invalid("knowledge.base.agent_required"));
        }
        Ok(())
    }
    pub fn list_agent_knowledge_libraries(
        &self,
        user: &str,
        agent: &str,
    ) -> Result<Vec<KnowledgeBase>, KnowledgeBaseError> {
        self.validate_grant_agent(user, agent)?;
        self.platform_libraries(user)
    }
    pub fn set_agent_knowledge_libraries(
        &self,
        user: &str,
        agent: &str,
        library_ids: &[String],
    ) -> Result<(), KnowledgeBaseError> {
        self.validate_grant_agent(user, agent)?;
        self.repository
            .set_agent_libraries(user, agent, library_ids)
    }
}
