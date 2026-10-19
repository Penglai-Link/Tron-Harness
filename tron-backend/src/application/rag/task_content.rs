use super::{
    RagConfigRepository, RagDebugSourceRequest, RagService, RagSourceContentRepository,
    RagSourceUnitsPage,
};

#[derive(Clone, Copy)]
pub enum RagTaskChunksError {
    NotFound,
    InvalidPagination,
    Unavailable,
    StorageFailure,
}
impl RagTaskChunksError {
    pub fn descriptor(self) -> super::TaskHistoryErrorDescriptor {
        let (code, message_key) = match self {
            Self::NotFound => ("101034001", "tron.rag.task_chunks_not_found"),
            Self::InvalidPagination => ("101034002", "tron.rag.task_chunks_pagination_invalid"),
            Self::Unavailable => ("101034003", "tron.rag.task_chunks_unavailable"),
            Self::StorageFailure => ("101034004", "tron.rag.task_chunks_storage_failed"),
        };
        super::TaskHistoryErrorDescriptor { code, message_key }
    }
}
impl<R: RagConfigRepository + RagSourceContentRepository + Clone, E: Clone> RagService<R, E> {
    /// Built-in tasks resolve the source and scope on the server, never from caller input.
    pub fn record_task_chunks(
        &self,
        user: &str,
        id: &str,
        current: usize,
        size: usize,
    ) -> Result<RagSourceUnitsPage, RagTaskChunksError> {
        use RagTaskChunksError::*;
        if current == 0 || current > 1_000_000 || size == 0 || size > 100 {
            return Err(InvalidPagination);
        }
        let task = match self
            .repository
            .record_task_summary(user, id)
            .map_err(|_| StorageFailure)?
        {
            Some(task) => task,
            None => self
                .repository
                .record_task_summary("SYSTEM", id)
                .map_err(|_| StorageFailure)?
                .filter(|task| task.corpus_id == "skill")
                .ok_or(NotFound)?,
        };
        if !matches!(task.corpus_id.as_str(), "skill" | "memory-card") {
            return Err(NotFound);
        }
        // Built-in writes replace the source generation; older attempts cannot promise an immutable snapshot.
        if task.state != "COMPLETED"
            || !self
                .repository
                .record_task_is_latest(&task)
                .map_err(|_| StorageFailure)?
        {
            return Err(Unavailable);
        }
        self.repository
            .source_units_page(
                RagDebugSourceRequest {
                    workspace_code: task.workspace_code,
                    corpus_id: task.corpus_id,
                    source_code: task.source_code,
                    source_version: task.source_version,
                    scope_type: task.scope_type,
                    scope_code: task.scope_code,
                    include_text_units: true,
                    include_warnings: false,
                },
                current,
                size,
            )
            .map_err(|_| StorageFailure)
    }
}
