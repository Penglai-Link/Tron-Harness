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
        managed_corpus_ids: &BTreeSet<String>,
    ) -> Result<Vec<RagCorpusSummary>, RagError> {
        self.repository
            .list_corpora_with_counts()?
            .into_iter()
            .filter(|(corpus, _)| {
                matches!(corpus.corpus_id.as_str(), "memory-card" | "skill")
                    || managed_corpus_ids.contains(&corpus.corpus_id)
            })
            .map(|(corpus, counts)| {
                let managed_source_id = managed_corpus_ids
                    .contains(&corpus.corpus_id)
                    .then(|| corpus.corpus_id.clone());
                Ok(RagCorpusSummary {
                    counts,
                    settings: RagSettings::from_capabilities(&corpus.capabilities)?,
                    kind: if managed_source_id.is_some() {
                        "managed"
                    } else {
                        "builtin"
                    }
                    .into(),
                    managed_source_id,
                    id: corpus.corpus_id,
                    name: corpus.display_name,
                })
            })
            .collect()
    }
    pub fn record_task_views(&self, user: &str) -> Result<Vec<RagTaskView>, RagError> {
        let mut tasks = self.repository.record_task_summaries(user)?;
        if user != "SYSTEM" {
            tasks.extend(
                self.repository
                    .record_task_summaries("SYSTEM")?
                    .into_iter()
                    .filter(|task| task.corpus_id == "skill"),
            );
        }
        // Repository order is creation order, independent of later checkpoint updates.
        let mut seen = BTreeSet::new();
        Ok(tasks
            .into_iter()
            .map(|task| {
                let is_latest = seen.insert((
                    task.user_code.clone(),
                    task.corpus_id.clone(),
                    task.workspace_code.clone(),
                    task.source_code.clone(),
                    task.scope_type.clone(),
                    task.scope_code.clone(),
                ));
                let mut view: RagTaskView = task.into();
                view.is_latest = is_latest;
                view
            })
            .collect())
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

#[derive(Clone, Copy, Debug)]
pub enum RagTaskHistoryError {
    NotFound,
    Active,
    StorageFailure,
}
pub struct TaskHistoryErrorDescriptor {
    pub code: &'static str,
    pub message_key: &'static str,
}
impl RagTaskHistoryError {
    pub fn descriptor(self) -> TaskHistoryErrorDescriptor {
        let (code, message_key) = match self {
            Self::NotFound => ("101033001", "tron.rag.task_history_not_found"),
            Self::Active => ("101033002", "tron.rag.task_history_active"),
            Self::StorageFailure => ("101033003", "tron.rag.task_history_storage_failed"),
        };
        TaskHistoryErrorDescriptor { code, message_key }
    }
}
