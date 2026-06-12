use crate::domain::rag::{RagError, RagStorageBinding};
use crate::shared::event_bus::EventBusHandle;

use super::{
    RagConfigRepository, RagConfigService, RagCorpusCatalogView, RagCorpusManagementInput,
    RagCorpusManagementListItem, RagCorpusManagementView, RagCorpusVerifyView, RagCorpusView,
    RagDebugProfileResult, RagDebugSourceRequest, RagDebugSourceResult, RagDiagnosticsRepository,
    RagDiagnosticsService, RagDocumentSourceRuntime, RagEmbeddingRuntime, RagFilterValidateInput,
    RagFilterValidateView, RagFilterValidationRequest, RagFilterValidationView,
    RagIngestRepository, RagIngestRequest, RagIngestResult, RagIngestService,
    RagManagementRepository, RagManagementService, RagPipelineCatalogView, RagProfileCatalogView,
    RagProfileListQuery, RagProfileManagementInput, RagProfileManagementListItem,
    RagProfileManagementView, RagProfileValidationView, RagProfileVerifyView, RagProfileView,
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
    R: RagConfigRepository + RagManagementRepository + Clone,
    S: RagStrategyRegistry,
    E: Clone,
{
    pub fn list_corpora_management(&self) -> Result<Vec<RagCorpusManagementListItem>, RagError> {
        self.management_service().list_corpora()
    }

    pub fn get_corpus_management(
        &self,
        corpus_id: &str,
    ) -> Result<RagCorpusManagementView, RagError> {
        self.management_service().get_corpus(corpus_id)
    }

    pub fn corpus_catalog(&self) -> RagCorpusCatalogView {
        self.management_service().corpus_catalog()
    }

    pub fn verify_corpus_management(
        &self,
        request: RagCorpusManagementInput,
    ) -> Result<RagCorpusVerifyView, RagError> {
        self.management_service().verify_corpus(request)
    }

    pub fn create_corpus_management(
        &self,
        request: RagCorpusManagementInput,
    ) -> Result<RagCorpusManagementView, RagError> {
        self.management_service().create_corpus(request)
    }

    pub fn update_corpus_management(
        &self,
        corpus_id: &str,
        request: RagCorpusManagementInput,
    ) -> Result<RagCorpusManagementView, RagError> {
        self.management_service().update_corpus(corpus_id, request)
    }

    pub fn list_profiles_management(
        &self,
        profile_type: Option<&str>,
    ) -> Result<Vec<RagProfileManagementListItem>, RagError> {
        self.management_service().list_profiles(profile_type)
    }

    pub fn get_profile_management(
        &self,
        profile_id: &str,
    ) -> Result<RagProfileManagementView, RagError> {
        self.management_service().get_profile(profile_id)
    }

    pub fn profile_catalog(&self) -> Result<RagProfileCatalogView, RagError> {
        self.management_service().profile_catalog()
    }

    pub fn pipeline_catalog(&self, profile_type: &str) -> Result<RagPipelineCatalogView, RagError> {
        self.management_service().pipeline_catalog(profile_type)
    }

    pub fn verify_profile_management(
        &self,
        request: RagProfileManagementInput,
    ) -> Result<RagProfileVerifyView, RagError> {
        self.management_service().verify_profile(request)
    }

    pub fn create_profile_management(
        &self,
        request: RagProfileManagementInput,
    ) -> Result<RagProfileManagementView, RagError> {
        self.management_service().create_profile(request)
    }

    pub fn update_profile_management(
        &self,
        profile_id: &str,
        request: RagProfileManagementInput,
    ) -> Result<RagProfileManagementView, RagError> {
        self.management_service()
            .update_profile(profile_id, request)
    }

    pub fn validate_filters_management(
        &self,
        request: RagFilterValidateInput,
    ) -> Result<RagFilterValidateView, RagError> {
        self.management_service().validate_filters(request)
    }

    fn management_service(&self) -> RagManagementService<R, S> {
        RagManagementService::new(self.repository.clone(), self.strategy_registry.clone())
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
