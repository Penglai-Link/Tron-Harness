use super::{KnowledgeBase, KnowledgeBaseError, KnowledgeBaseService};

impl KnowledgeBaseService {
    fn validate_grant_agent(&self, user: &str, agent: &str) -> Result<(), KnowledgeBaseError> {
        self.require_user(user)?;
        let agents = self.agents.list_platform_internal_agents()
            .map_err(|error| KnowledgeBaseError::Storage(format!("{error:?}")))?;
        if !agents.iter().any(|record| record.agent_code == agent) {
            return Err(KnowledgeBaseError::Invalid("请选择已定义的内部智能体".into()));
        }
        Ok(())
    }
    pub fn list_agent_knowledge_libraries(&self, user: &str, agent: &str) -> Result<Vec<KnowledgeBase>, KnowledgeBaseError> {
        self.validate_grant_agent(user, agent)?;
        self.platform_libraries(user)
    }
    pub fn set_agent_knowledge_libraries(&self, user: &str, agent: &str, library_ids: &[String]) -> Result<(), KnowledgeBaseError> {
        self.validate_grant_agent(user, agent)?;
        self.repository.set_agent_libraries(user, agent, library_ids)
    }
}
