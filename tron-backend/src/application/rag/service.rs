use std::sync::{Arc, Mutex};

use crate::domain::rag::{RagCorpusRecord, RagError};

pub trait RagConfigRepository {
    fn recover_record_tasks(&self) -> Result<(), RagError> {
        Ok(())
    }
    fn save_record_task(&self, _task: &super::RagRecordTask) -> Result<(), RagError> {
        Err(RagError::StorageFailure)
    }
    fn record_tasks(&self, _user: &str) -> Result<Vec<super::RagRecordTask>, RagError> {
        Err(RagError::StorageFailure)
    }
    fn record_task(&self, user: &str, id: &str) -> Result<Option<super::RagRecordTask>, RagError> {
        Ok(self
            .record_tasks(user)?
            .into_iter()
            .find(|task| task.id == id))
    }
    fn record_task_is_latest(&self, task: &super::RagRecordTask) -> Result<bool, RagError> {
        Ok(!self.record_tasks(&task.user_code)?.iter().any(|other| {
            other.id != task.id
                && other.corpus_id == task.corpus_id
                && other.source_code == task.source_code
                && other.source_version == task.source_version
                && other.scope_type == task.scope_type
                && other.scope_code == task.scope_code
                && other.created_at >= task.created_at
        }))
    }
    fn claim_record_task(&self, _user: &str, _id: &str) -> Result<bool, RagError> {
        Err(RagError::StorageFailure)
    }
    fn index_counts(&self, _corpus: &str) -> Result<super::RagIndexCounts, RagError> {
        Err(RagError::StorageFailure)
    }
    fn list_corpora(&self) -> Result<Vec<RagCorpusRecord>, RagError>;
    fn get_corpus(&self, corpus_id: &str) -> Result<Option<RagCorpusRecord>, RagError>;
}

impl<T> RagConfigRepository for Arc<Mutex<T>>
where
    T: RagConfigRepository,
{
    fn recover_record_tasks(&self) -> Result<(), RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .recover_record_tasks()
    }
    fn save_record_task(&self, task: &super::RagRecordTask) -> Result<(), RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .save_record_task(task)
    }
    fn record_tasks(&self, user: &str) -> Result<Vec<super::RagRecordTask>, RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .record_tasks(user)
    }
    fn record_task(&self, user: &str, id: &str) -> Result<Option<super::RagRecordTask>, RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .record_task(user, id)
    }
    fn record_task_is_latest(&self, task: &super::RagRecordTask) -> Result<bool, RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .record_task_is_latest(task)
    }
    fn claim_record_task(&self, user: &str, id: &str) -> Result<bool, RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .claim_record_task(user, id)
    }
    fn index_counts(&self, corpus: &str) -> Result<super::RagIndexCounts, RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .index_counts(corpus)
    }
    fn list_corpora(&self) -> Result<Vec<RagCorpusRecord>, RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .list_corpora()
    }

    fn get_corpus(&self, corpus_id: &str) -> Result<Option<RagCorpusRecord>, RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .get_corpus(corpus_id)
    }
}
