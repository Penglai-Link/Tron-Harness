use super::{MemoryError, MemoryRecord, MemoryRecordPage, MemoryRecordQuery};
use crate::application::rag::{RagIdentity, RagRecordTask};

pub trait MemoryRecordRepository: Send + Sync {
    fn get(&self, user: &str, code: &str) -> Result<Option<MemoryRecord>, MemoryError>;
    fn find_semantic(
        &self,
        user: &str,
        scope: super::MemoryScope,
        scope_code: &str,
        key: &str,
    ) -> Result<Option<MemoryRecord>, MemoryError>;
    fn query(&self, user: &str, query: MemoryRecordQuery) -> Result<MemoryRecordPage, MemoryError>;
    fn recallable(&self, identity: &RagIdentity) -> Result<Vec<MemoryRecord>, MemoryError>;
    fn domain_exists(&self, code: &str) -> Result<bool, MemoryError>;
    /// Original text and its durable RAG task are committed atomically.
    fn save(
        &self,
        record: &MemoryRecord,
        expected: Option<u32>,
        task: Option<&RagRecordTask>,
    ) -> Result<(), MemoryError>;
    fn delete(&self, user: &str, code: &str, version: u32) -> Result<(), MemoryError>;
}
