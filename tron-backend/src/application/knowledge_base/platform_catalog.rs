use super::*;

impl KnowledgeBaseService {
    pub fn platform_libraries(&self, user: &str) -> Result<Vec<KnowledgeBase>, KnowledgeBaseError> {
        if user.trim().is_empty() {
            return Err(KnowledgeBaseError::Forbidden);
        }
        let mut libraries = self.repository.platform_libraries(user)?;
        for library in &mut libraries {
            library.document_count = self.repository.documents(&library.id)?.len();
            library.agent_codes = self.repository.grants(&library.id)?;
        }
        Ok(libraries)
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
            .map_err(|e| KnowledgeBaseError::Storage(format!("{e:?}")))?;
        if agents
            .iter()
            .any(|code| !valid.iter().any(|agent| agent.agent_code == *code))
        {
            return Err(KnowledgeBaseError::Invalid("只支持平台内部智能体".into()));
        }
        self.repository
            .replace_library_agent_grants(user, id, agents)
    }
}

impl KnowledgeBaseService {
    pub fn rag_task_views(&self, user: &str) -> Result<Vec<RagTaskView>, KnowledgeBaseError> {
        let mut views = vec![];
        for kb in self.platform_libraries(user)? {
            for doc in self.repository.documents(&kb.id)? {
                for task in self.ingestion_tasks(user, &kb.id, &doc.id)? {
                    views.push(RagTaskView {
                        file_size_bytes: task.file_size_bytes,
                        id: task.id,
                        document_id: Some(doc.id.clone()),
                        corpus_id: kb.id.clone(),
                        source_code: doc.id.clone(),
                        title: doc.name.clone(),
                        state: match task.state.as_str() {
                            "READY" => "COMPLETED",
                            "WAITING_EMBEDDING" | "WAITING_RETRY" => "QUEUED",
                            other => other,
                        }
                        .into(),
                        stage: task
                            .index_stage
                            .unwrap_or_else(|| format!("{:?}", task.stage).to_ascii_uppercase()),
                        completed_units: task.completed_units,
                        total_units: task.total_units,
                        error: task.error,
                        created_at: task.created_at,
                        updated_at: task.updated_at,
                    });
                }
            }
        }
        Ok(views)
    }
}
