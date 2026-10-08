use super::*;
use crate::domain::rag::RagError;
#[derive(Clone)]
pub struct RagService<R, E> {
    pub(super) repository: R,
    pub(super) runtime: E,
    pub(super) embedding_batch_limits: RagEmbeddingBatchLimits,
    pub(super) invocation_runtime_config:
        Option<crate::application::system_config::BackendRuntimeConfig>,
}
impl<R: Clone, E: Clone> RagService<R, E> {
    pub fn new(repository: R, runtime: E) -> Self {
        Self {
            repository,
            runtime,
            embedding_batch_limits: RagEmbeddingBatchLimits::default(),
            invocation_runtime_config: None,
        }
    }
    pub fn with_runtime_config(
        mut self,
        runtime: crate::application::system_config::BackendRuntimeConfig,
    ) -> Self {
        self.invocation_runtime_config = Some(runtime);
        self
    }
    pub fn with_embedding_batch_limits(mut self, limits: RagEmbeddingBatchLimits) -> Self {
        self.embedding_batch_limits = limits;
        self
    }
    pub(crate) fn repository_handle(&self) -> R {
        self.repository.clone()
    }
}
impl<R: RagSourceLifecycleRepository + Clone, E: Clone> RagService<R, E> {
    pub fn deactivate_source(&self, command: RagDeactivateSourceCommand) -> Result<(), RagError> {
        self.repository.deactivate_source(command)
    }
}
impl<R: RagIngestRepository + Clone + Send + Sync, E: RagEmbeddingRuntime + RagGraphRuntime>
    RagService<R, E>
{
    pub async fn ingest(&self, request: RagIngestRequest) -> Result<RagIngestResult, RagError> {
        self.ingest_record(request).await
    }
}
