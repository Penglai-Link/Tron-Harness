use crate::domain::rag::{RagCorpusRecord, RagError};
use std::sync::{Arc, Mutex};
#[derive(Debug, Clone, PartialEq)]
pub struct RagCorpusManagementRecord {
    pub record: RagCorpusRecord,
}
pub trait RagManagementRepository {
    fn get_managed_corpus(&self, id: &str) -> Result<Option<RagCorpusManagementRecord>, RagError>;
    fn save_managed_corpus(&self, record: &RagCorpusRecord) -> Result<(), RagError>;
    fn delete_managed_corpus(&self, id: &str) -> Result<bool, RagError>;
}
impl<T: RagManagementRepository> RagManagementRepository for Arc<Mutex<T>> {
    fn get_managed_corpus(&self, id: &str) -> Result<Option<RagCorpusManagementRecord>, RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .get_managed_corpus(id)
    }
    fn save_managed_corpus(&self, record: &RagCorpusRecord) -> Result<(), RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .save_managed_corpus(record)
    }
    fn delete_managed_corpus(&self, id: &str) -> Result<bool, RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .delete_managed_corpus(id)
    }
}
