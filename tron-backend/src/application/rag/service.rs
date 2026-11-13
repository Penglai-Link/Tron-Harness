use std::sync::{Arc, Mutex};

use crate::domain::rag::{RagCorpusRecord, RagError};

pub trait RagConfigRepository {
    fn deleted_task_history(
        &self,
        _user: &str,
    ) -> Result<std::collections::BTreeSet<String>, RagError> {
        Ok(Default::default())
    }
    fn delete_task_history(&self, _user: &str, _id: &str) -> Result<(), RagError> {
        Err(RagError::StorageFailure)
    }
    fn recover_record_tasks(&self) -> Result<(), RagError> {
        Ok(())
    }
    fn save_record_task(&self, _task: &super::RagRecordTask) -> Result<(), RagError> {
        Err(RagError::StorageFailure)
    }
    fn record_tasks(&self, _user: &str) -> Result<Vec<super::RagRecordTask>, RagError> {
        Err(RagError::StorageFailure)
    }
    /// List metadata without loading document bodies or vector checkpoints.
    fn record_task_summaries(&self, user: &str) -> Result<Vec<super::RagRecordTask>, RagError>;
    fn record_task(&self, user: &str, id: &str) -> Result<Option<super::RagRecordTask>, RagError>;
    fn record_task_summary(
        &self,
        user: &str,
        id: &str,
    ) -> Result<Option<super::RagRecordTask>, RagError>;
    fn record_task_is_latest(&self, task: &super::RagRecordTask) -> Result<bool, RagError>;
    fn claim_record_task(&self, _user: &str, _id: &str) -> Result<bool, RagError> {
        Err(RagError::StorageFailure)
    }
    fn index_counts(&self, _corpus: &str) -> Result<super::RagIndexCounts, RagError> {
        Err(RagError::StorageFailure)
    }
    fn list_corpora_with_counts(
        &self,
    ) -> Result<Vec<(RagCorpusRecord, super::RagIndexCounts)>, RagError>;
    fn list_corpora(&self) -> Result<Vec<RagCorpusRecord>, RagError>;
    fn get_corpora(
        &self,
        ids: &[String],
    ) -> Result<std::collections::BTreeMap<String, RagCorpusRecord>, RagError>;
    fn get_corpus(&self, corpus_id: &str) -> Result<Option<RagCorpusRecord>, RagError>;
}

impl<T> RagConfigRepository for Arc<Mutex<T>>
where
    T: RagConfigRepository,
{
    fn record_task_summary(
        &self,
        user: &str,
        id: &str,
    ) -> Result<Option<super::RagRecordTask>, RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .record_task_summary(user, id)
    }
    fn deleted_task_history(
        &self,
        user: &str,
    ) -> Result<std::collections::BTreeSet<String>, RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .deleted_task_history(user)
    }
    fn delete_task_history(&self, user: &str, id: &str) -> Result<(), RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .delete_task_history(user, id)
    }
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
    fn record_task_summaries(&self, user: &str) -> Result<Vec<super::RagRecordTask>, RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .record_task_summaries(user)
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
    fn list_corpora_with_counts(
        &self,
    ) -> Result<Vec<(RagCorpusRecord, super::RagIndexCounts)>, RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .list_corpora_with_counts()
    }
    fn list_corpora(&self) -> Result<Vec<RagCorpusRecord>, RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .list_corpora()
    }

    fn get_corpora(
        &self,
        ids: &[String],
    ) -> Result<std::collections::BTreeMap<String, RagCorpusRecord>, RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .get_corpora(ids)
    }
    fn get_corpus(&self, corpus_id: &str) -> Result<Option<RagCorpusRecord>, RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .get_corpus(corpus_id)
    }
}
