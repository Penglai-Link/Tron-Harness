use crate::domain::rag::{RagError, RagStorageBinding};
use crate::shared::event_bus::EventBusHandle;

use super::{
    RagConfigRepository, RagConfigService, RagCorpusView, RagDebugProfileResult,
    RagDebugSourceRequest, RagDebugSourceResult, RagDiagnosticsRepository, RagDiagnosticsService,
    RagDocumentSourceRuntime, RagEmbeddingRuntime, RagFilterValidationRequest,
    RagFilterValidationView, RagIngestRepository, RagIngestRequest, RagIngestResult,
    RagIngestService, RagProfileListQuery, RagProfileValidationView, RagProfileView,
    RagRetryIndexRepository, RagRetryIndexRequest, RagRetryIndexResult, RagRetryIndexService,
    RagSearchRepository, RagSearchRequest, RagSearchResult, RagSearchRuntime, RagSearchService,
    RagStrategyRegistry,
};

#[derive(Clone)]
pub struct RagService<R, S, E> {
    repository: R,
    strategy_registry: S,
    runtime: E,
    event_bus: EventBusHandle,
}

impl<R, S, E> RagService<R, S, E>
where
    R: Clone,
    S: Clone,
    E: Clone,
{
    pub fn new(repository: R, strategy_registry: S, runtime: E) -> Self {
        Self {
            repository,
            strategy_registry,
            runtime,
            event_bus: EventBusHandle::disabled(),
        }
    }

    pub fn with_event_bus(mut self, event_bus: EventBusHandle) -> Self {
        self.event_bus = event_bus;
        self
    }
}

impl<R, S, E> RagService<R, S, E>
where
    R: RagConfigRepository + Clone,
    S: RagStrategyRegistry,
    E: Clone,
{
    pub fn list_corpora(&self) -> Result<Vec<RagCorpusView>, RagError> {
        self.config_service().list_corpora()
    }

    pub fn get_corpus(&self, corpus_id: &str) -> Result<RagCorpusView, RagError> {
        self.config_service().get_corpus(corpus_id)
    }

    pub fn get_storage_binding(&self, corpus_id: &str) -> Result<RagStorageBinding, RagError> {
        self.config_service().get_storage_binding(corpus_id)
    }

    pub fn list_profiles(
        &self,
        query: RagProfileListQuery,
    ) -> Result<Vec<RagProfileView>, RagError> {
        self.config_service().list_profiles(query)
    }

    pub fn get_profile(&self, profile_id: &str) -> Result<RagProfileView, RagError> {
        self.config_service().get_profile(profile_id)
    }

    pub fn validate_profile(&self, profile_id: &str) -> Result<RagProfileValidationView, RagError> {
        self.config_service().validate_profile(profile_id)
    }

    pub fn validate_filters(
        &self,
        request: RagFilterValidationRequest,
    ) -> Result<RagFilterValidationView, RagError> {
        self.config_service().validate_filters(request)
    }

    fn config_service(&self) -> RagConfigService<R, S> {
        RagConfigService::new(self.repository.clone(), self.strategy_registry.clone())
    }
}

impl<R, S, E> RagService<R, S, E>
where
    R: RagIngestRepository
        + RagSearchRepository
        + RagDiagnosticsRepository
        + RagRetryIndexRepository
        + Clone,
    S: RagStrategyRegistry,
    E: RagEmbeddingRuntime + RagSearchRuntime + RagDocumentSourceRuntime,
{
    pub async fn ingest(&self, request: RagIngestRequest) -> Result<RagIngestResult, RagError> {
        RagIngestService::new(
            self.repository.clone(),
            self.strategy_registry.clone(),
            self.runtime.clone(),
        )
        .with_event_bus(self.event_bus.clone())
        .ingest(request)
        .await
    }

    pub async fn search(&self, request: RagSearchRequest) -> Result<RagSearchResult, RagError> {
        RagSearchService::new(
            self.repository.clone(),
            self.strategy_registry.clone(),
            self.runtime.clone(),
        )
        .with_event_bus(self.event_bus.clone())
        .search(request)
        .await
    }

    pub fn debug_profile(
        &self,
        workspace_code: &str,
        profile_id: &str,
    ) -> Result<RagDebugProfileResult, RagError> {
        RagDiagnosticsService::new(self.repository.clone(), self.strategy_registry.clone())
            .debug_profile(workspace_code, profile_id)
    }

    pub fn debug_source(
        &self,
        request: RagDebugSourceRequest,
    ) -> Result<RagDebugSourceResult, RagError> {
        RagDiagnosticsService::new(self.repository.clone(), self.strategy_registry.clone())
            .debug_source(request)
    }

    pub async fn retry_index(
        &self,
        request: RagRetryIndexRequest,
    ) -> Result<RagRetryIndexResult, RagError> {
        RagRetryIndexService::new(self.repository.clone(), self.runtime.clone())
            .with_event_bus(self.event_bus.clone())
            .retry_index(request)
            .await
    }
}
