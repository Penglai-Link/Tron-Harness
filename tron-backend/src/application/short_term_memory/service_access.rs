use crate::domain::short_term_memory::ShortTermMemoryError;

use super::{ShortTermMemoryRepository, ShortTermMemoryService};

impl ShortTermMemoryService {
    pub(super) fn next_id(&self, table: &str) -> Result<i64, ShortTermMemoryError> {
        self.repository().next_id(table)
    }

    pub(super) fn next_code(&self, prefix: &str) -> Result<String, ShortTermMemoryError> {
        Ok(format!(
            "{prefix}-{}",
            self.id_provider
                .next_id()
                .map_err(|_| ShortTermMemoryError::StorageFailure)?
                .value()
        ))
    }

    pub(super) fn repository(
        &self,
    ) -> std::sync::MutexGuard<'_, Box<dyn ShortTermMemoryRepository>> {
        self.repository
            .lock()
            .expect("short-term memory repository mutex poisoned")
    }

    pub(super) fn ensure_session(
        &self,
        session_code: &str,
    ) -> Result<String, ShortTermMemoryError> {
        self.repository()
            .session_workspace(session_code.trim())?
            .ok_or(ShortTermMemoryError::SessionNotFound)
    }
}
