use super::{
    RagConfigRepository, RagCorpusSummary, RagManagementRepository, RagService, RagSettings,
    RagTaskView,
};
use crate::domain::rag::RagError;
use std::collections::BTreeSet;

impl<R: RagConfigRepository + Clone, E: Clone> RagService<R, E> {
    pub fn settings_for_corpus(&self, id: &str) -> Result<RagSettings, RagError> {
        let corpus = self
            .repository
            .get_corpus(id)?
            .ok_or(RagError::CorpusNotFound)?;
        RagSettings::from_capabilities(&corpus.capabilities)
    }
    pub fn corpus_summaries(
        &self,
        knowledge_base_ids: &BTreeSet<String>,
    ) -> Result<Vec<RagCorpusSummary>, RagError> {
        self.repository
            .list_corpora()?
            .into_iter()
            .filter(|corpus| {
                matches!(corpus.corpus_id.as_str(), "memory-card" | "skill")
                    || knowledge_base_ids.contains(&corpus.corpus_id)
            })
            .map(|corpus| {
                let knowledge_base_id = knowledge_base_ids
                    .contains(&corpus.corpus_id)
                    .then(|| corpus.corpus_id.clone());
                Ok(RagCorpusSummary {
                    counts: self.repository.index_counts(&corpus.corpus_id)?,
                    settings: RagSettings::from_capabilities(&corpus.capabilities)?,
                    kind: if knowledge_base_id.is_some() {
                        "knowledgeBase"
                    } else {
                        "builtin"
                    }
                    .into(),
                    knowledge_base_id,
                    id: corpus.corpus_id,
                    name: corpus.display_name,
                })
            })
            .collect()
    }
    pub fn record_task_views(&self, user: &str) -> Result<Vec<RagTaskView>, RagError> {
        let mut tasks = self.repository.record_tasks(user)?;
        if user != "SYSTEM" {
            tasks.extend(
                self.repository
                    .record_tasks("SYSTEM")?
                    .into_iter()
                    .filter(|task| task.corpus_id == "skill"),
            );
        }
        tasks.sort_by_key(|task| std::cmp::Reverse(task.created_at));
        Ok(tasks.into_iter().map(Into::into).collect())
    }
    pub fn cancel_record_task(&self, user: &str, id: &str) -> Result<RagTaskView, RagError> {
        let mut task = self
            .repository
            .record_task(user, id)?
            .ok_or(RagError::SourceNotFound)?;
        if !matches!(task.state.as_str(), "RUNNING" | "QUEUED") {
            return Err(RagError::IngestRequestInvalid);
        }
        task.state = "CANCELLED".into();
        task.updated_at = chrono::Utc::now().timestamp_millis();
        self.repository.save_record_task(&task)?;
        Ok(task.into())
    }
}
impl<R: RagManagementRepository + RagConfigRepository + Clone, E: Clone> RagService<R, E> {
    pub fn configure_corpus(
        &self,
        id: &str,
        settings: RagSettings,
    ) -> Result<RagSettings, RagError> {
        settings.validate()?;
        let mut corpus = self
            .repository
            .get_corpus(id)?
            .ok_or(RagError::CorpusNotFound)?;
        corpus.capabilities["settings"] =
            serde_json::to_value(&settings).map_err(|_| RagError::StorageFailure)?;
        self.repository.save_managed_corpus(&corpus)?;
        Ok(settings)
    }
}
