use crate::{
    application::prompt::PromptService,
    domain::rag::{RagError, RagStorageBinding},
    shared::event_bus::EventBusHandle,
};
use std::sync::Arc;

use super::{
    RagConfigRepository, RagConfigService, RagCorpusCatalogView, RagCorpusDeleteResult,
    RagCorpusManagementInput, RagCorpusManagementListItem, RagCorpusManagementView,
    RagCorpusVerifyView, RagCorpusView, RagDebugProfileResult, RagDebugSourceRequest,
    RagDebugSourceResult, RagDiagnosticsRepository, RagDiagnosticsService,
    RagDocumentSourceRuntime, RagEmbeddingRuntime, RagFilterValidateInput, RagFilterValidateView,
    RagFilterValidationRequest, RagFilterValidationView, RagIngestRepository, RagIngestRequest,
    RagIngestResult, RagIngestService, RagManagementRepository, RagManagementService,
    RagPipelineCatalogView, RagProfileCatalogView, RagProfileDeleteResult, RagProfileListQuery,
    RagProfileManagementInput, RagProfileManagementListItem, RagProfileManagementView,
    RagProfileValidationView, RagProfileVerifyView, RagProfileView, RagRetryIndexRepository,
    RagRetryIndexRequest, RagRetryIndexResult, RagRetryIndexService, RagSearchRepository,
    RagSearchRequest, RagSearchResult, RagSearchRuntime, RagSearchService,
    RagSourceLifecycleRepository, RagStrategyRegistry,
};

pub trait RagProductGuard: Send + Sync {
    fn is_product_corpus(&self, id: &str) -> bool;
}
#[derive(Clone)]
pub struct RagService<R, S, E> {
    repository: R,
    strategy_registry: S,
    runtime: E,
    event_bus: EventBusHandle,
    prompt_service: Option<PromptService>,
    product_guard: Option<Arc<dyn RagProductGuard>>,
}

impl<R, S, E> RagService<R, S, E>
where
    R: RagSourceLifecycleRepository + Clone,
    S: Clone,
    E: Clone,
{
    pub fn deactivate_source(
        &self,
        command: super::RagDeactivateSourceCommand,
    ) -> Result<(), RagError> {
        self.guard_corpus(&command.corpus_id)?;
        self.repository.deactivate_source(command)
    }
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
            prompt_service: None,
            product_guard: None,
        }
    }

    pub fn with_product_guard(mut self, guard: Arc<dyn RagProductGuard>) -> Self {
        self.product_guard = Some(guard);
        self
    }
    pub(crate) fn repository_handle(&self) -> R {
        self.repository.clone()
    }
    fn protected(&self, id: &str) -> bool {
        id.trim().starts_with("kb_")
            || self
                .product_guard
                .as_ref()
                .is_some_and(|g| g.is_product_corpus(id))
    }
    fn guard_corpus(&self, id: &str) -> Result<(), RagError>
    where
        R: RagConfigRepository,
    {
        if self.protected(id) {
            return Err(RagError::SearchScopeInvalid);
        }
        if let Some(corpus) = self.repository.get_corpus(id)?
            && corpus
                .storage_binding
                .segment_targets
                .values()
                .any(|target| {
                    target.table.as_deref().is_some_and(reserved_kb_storage)
                        || target.namespace.as_deref().is_some_and(reserved_kb_storage)
                })
        {
            return Err(RagError::SearchScopeInvalid);
        }
        Ok(())
    }
    pub fn with_event_bus(mut self, event_bus: EventBusHandle) -> Self {
        self.event_bus = event_bus;
        self
    }

    pub fn with_prompt_service(mut self, prompt_service: PromptService) -> Self {
        self.prompt_service = Some(prompt_service);
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
        Ok(self
            .config_service()
            .list_corpora()?
            .into_iter()
            .filter(|c| self.guard_corpus(&c.corpus_id).is_ok())
            .collect())
    }

    pub fn get_corpus(&self, corpus_id: &str) -> Result<RagCorpusView, RagError> {
        self.guard_corpus(corpus_id)?;
        self.config_service().get_corpus(corpus_id)
    }

    pub fn get_storage_binding(&self, corpus_id: &str) -> Result<RagStorageBinding, RagError> {
        self.guard_corpus(corpus_id)?;
        self.config_service().get_storage_binding(corpus_id)
    }

    pub fn list_profiles(
        &self,
        query: RagProfileListQuery,
    ) -> Result<Vec<RagProfileView>, RagError> {
        Ok(self
            .config_service()
            .list_profiles(query)?
            .into_iter()
            .filter(|p| {
                !p.corpus_ids
                    .iter()
                    .chain(p.primary_corpus_id.iter())
                    .any(|id| self.guard_corpus(id).is_err())
                    && self.guard_config(&p.config).is_ok()
            })
            .collect())
    }

    pub fn get_profile(&self, profile_id: &str) -> Result<RagProfileView, RagError> {
        self.guard_profile(profile_id)?;
        self.config_service().get_profile(profile_id)
    }

    pub fn validate_profile(&self, profile_id: &str) -> Result<RagProfileValidationView, RagError> {
        self.guard_profile(profile_id)?;
        self.config_service().validate_profile(profile_id)
    }

    pub fn validate_filters(
        &self,
        request: RagFilterValidationRequest,
    ) -> Result<RagFilterValidationView, RagError> {
        self.guard_corpus(&request.corpus_id)?;
        self.config_service().validate_filters(request)
    }

    fn guard_profile(&self, id: &str) -> Result<(), RagError> {
        let p = self
            .repository
            .get_profile(id)?
            .ok_or(RagError::ProfileNotFound)?;
        for corpus in p.corpus_ids.iter().chain(p.primary_corpus_id.iter()) {
            self.guard_corpus(corpus)?;
        }
        self.guard_config(&p.config)
    }
    fn guard_config(&self, value: &serde_json::Value) -> Result<(), RagError> {
        match value {
            serde_json::Value::Object(map) => {
                for (key, value) in map {
                    if matches!(key.as_str(), "table" | "namespace")
                        && value.as_str().is_some_and(reserved_kb_storage)
                    {
                        return Err(RagError::SearchScopeInvalid);
                    }
                    if key == "corpusId" || key == "primaryCorpusId" {
                        if let Some(id) = value.as_str() {
                            self.guard_corpus(id)?;
                        }
                    }
                    if key == "corpusIds" {
                        if let Some(ids) = value.as_array() {
                            for id in ids {
                                if let Some(id) = id.as_str() {
                                    self.guard_corpus(id)?;
                                }
                            }
                        }
                    }
                    self.guard_config(value)?;
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    self.guard_config(item)?;
                }
            }
            _ => {}
        }
        Ok(())
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
        Ok(self
            .management_service()
            .list_corpora()?
            .into_iter()
            .filter(|c| self.guard_corpus(&c.corpus_id).is_ok())
            .collect())
    }

    pub fn get_corpus_management(
        &self,
        corpus_id: &str,
    ) -> Result<RagCorpusManagementView, RagError> {
        self.guard_corpus(corpus_id)?;
        self.management_service().get_corpus(corpus_id)
    }

    pub fn corpus_catalog(&self) -> RagCorpusCatalogView {
        self.management_service().corpus_catalog()
    }

    pub fn verify_corpus_management(
        &self,
        request: RagCorpusManagementInput,
    ) -> Result<RagCorpusVerifyView, RagError> {
        self.guard_corpus(&request.corpus_id)?;
        self.guard_config(&serde_json::to_value(&request).map_err(|_| RagError::StorageFailure)?)?;
        self.management_service().verify_corpus(request)
    }

    pub fn create_corpus_management(
        &self,
        request: RagCorpusManagementInput,
    ) -> Result<RagCorpusManagementView, RagError> {
        self.guard_corpus(&request.corpus_id)?;
        self.guard_config(&serde_json::to_value(&request).map_err(|_| RagError::StorageFailure)?)?;
        self.management_service().create_corpus(request)
    }

    pub fn update_corpus_management(
        &self,
        corpus_id: &str,
        request: RagCorpusManagementInput,
    ) -> Result<RagCorpusManagementView, RagError> {
        self.guard_corpus(corpus_id)?;
        self.guard_corpus(&request.corpus_id)?;
        self.guard_config(&serde_json::to_value(&request).map_err(|_| RagError::StorageFailure)?)?;
        self.management_service().update_corpus(corpus_id, request)
    }

    pub fn delete_corpus_management(
        &self,
        corpus_id: &str,
    ) -> Result<RagCorpusDeleteResult, RagError> {
        self.guard_corpus(corpus_id)?;
        self.management_service().delete_corpus(corpus_id)
    }

    pub fn list_profiles_management(
        &self,
        profile_type: Option<&str>,
    ) -> Result<Vec<RagProfileManagementListItem>, RagError> {
        Ok(self
            .management_service()
            .list_profiles(profile_type)?
            .into_iter()
            .filter(|p| {
                !p.corpus_ids
                    .iter()
                    .chain(p.primary_corpus_id.iter())
                    .any(|id| self.guard_corpus(id).is_err())
                    && self
                        .guard_config(
                            &serde_json::to_value(&p.routes).unwrap_or(serde_json::Value::Null),
                        )
                        .is_ok()
            })
            .collect())
    }

    pub fn get_profile_management(
        &self,
        profile_id: &str,
    ) -> Result<RagProfileManagementView, RagError> {
        self.guard_profile(profile_id)?;
        self.management_service().get_profile(profile_id)
    }

    pub fn profile_catalog(&self) -> Result<RagProfileCatalogView, RagError> {
        let mut catalog = self.management_service().profile_catalog()?;
        catalog
            .corpora
            .retain(|corpus| self.guard_corpus(&corpus.corpus_id).is_ok());
        Ok(catalog)
    }

    pub fn pipeline_catalog(&self, profile_type: &str) -> Result<RagPipelineCatalogView, RagError> {
        self.management_service().pipeline_catalog(profile_type)
    }

    pub fn verify_profile_management(
        &self,
        request: RagProfileManagementInput,
    ) -> Result<RagProfileVerifyView, RagError> {
        self.guard_config(&serde_json::to_value(&request).map_err(|_| RagError::StorageFailure)?)?;
        self.management_service().verify_profile(request)
    }

    pub fn create_profile_management(
        &self,
        request: RagProfileManagementInput,
    ) -> Result<RagProfileManagementView, RagError> {
        self.guard_config(&serde_json::to_value(&request).map_err(|_| RagError::StorageFailure)?)?;
        self.management_service().create_profile(request)
    }

    pub fn update_profile_management(
        &self,
        profile_id: &str,
        request: RagProfileManagementInput,
    ) -> Result<RagProfileManagementView, RagError> {
        self.guard_profile(profile_id)?;
        self.guard_config(&serde_json::to_value(&request).map_err(|_| RagError::StorageFailure)?)?;
        self.management_service()
            .update_profile(profile_id, request)
    }

    pub fn delete_profile_management(
        &self,
        profile_id: &str,
    ) -> Result<RagProfileDeleteResult, RagError> {
        self.guard_profile(profile_id)?;
        self.management_service().delete_profile(profile_id)
    }

    pub fn validate_filters_management(
        &self,
        request: RagFilterValidateInput,
    ) -> Result<RagFilterValidateView, RagError> {
        self.guard_config(&serde_json::to_value(&request).map_err(|_| RagError::StorageFailure)?)?;
        self.management_service().validate_filters(request)
    }

    fn management_service(&self) -> RagManagementService<R, S> {
        RagManagementService::new(self.repository.clone(), self.strategy_registry.clone())
    }
}

impl<R, S, E> RagService<R, S, E>
where
    R: RagIngestRepository + Clone,
    S: RagStrategyRegistry,
    E: RagEmbeddingRuntime + RagDocumentSourceRuntime,
{
    pub async fn ingest(&self, request: RagIngestRequest) -> Result<RagIngestResult, RagError> {
        self.guard_corpus(&request.corpus_id)?;
        self.guard_profile(&request.ingest_profile_id)?;
        let service = RagIngestService::new(
            self.repository.clone(),
            self.strategy_registry.clone(),
            self.runtime.clone(),
        )
        .with_event_bus(self.event_bus.clone());
        let service = match &self.prompt_service {
            Some(prompt_service) => service.with_prompt_service(prompt_service.clone()),
            None => service,
        };
        service.ingest(request).await
    }
}

impl<R, S, E> RagService<R, S, E>
where
    R: RagSearchRepository + Clone,
    S: RagStrategyRegistry,
    E: RagSearchRuntime,
{
    pub async fn search(&self, request: RagSearchRequest) -> Result<RagSearchResult, RagError> {
        self.guard_profile(&request.profile_id)?;
        RagSearchService::new(
            self.repository.clone(),
            self.strategy_registry.clone(),
            self.runtime.clone(),
        )
        .with_event_bus(self.event_bus.clone())
        .search(request)
        .await
    }
}

impl<R, S, E> RagService<R, S, E>
where
    R: RagDiagnosticsRepository + RagConfigRepository + Clone,
    S: RagStrategyRegistry,
    E: Clone,
{
    pub fn debug_profile(
        &self,
        workspace_code: &str,
        profile_id: &str,
    ) -> Result<RagDebugProfileResult, RagError> {
        self.guard_profile(profile_id)?;
        RagDiagnosticsService::new(self.repository.clone(), self.strategy_registry.clone())
            .debug_profile(workspace_code, profile_id)
    }

    pub fn debug_source(
        &self,
        request: RagDebugSourceRequest,
    ) -> Result<RagDebugSourceResult, RagError> {
        self.guard_corpus(&request.corpus_id)?;
        RagDiagnosticsService::new(self.repository.clone(), self.strategy_registry.clone())
            .debug_source(request)
    }
}

impl<R, S, E> RagService<R, S, E>
where
    R: RagRetryIndexRepository + RagConfigRepository + Clone,
    S: Clone,
    E: RagEmbeddingRuntime,
{
    pub async fn retry_index(
        &self,
        request: RagRetryIndexRequest,
    ) -> Result<RagRetryIndexResult, RagError> {
        self.guard_corpus(&request.corpus_id)?;
        RagRetryIndexService::new(self.repository.clone(), self.runtime.clone())
            .with_event_bus(self.event_bus.clone())
            .retry_index(request)
            .await
    }
}

// KB carriers are reserved even when an advanced corpus tries to alias their
// physical table or namespace under a different logical corpus ID.
fn reserved_kb_storage(value: &str) -> bool {
    let name = value
        .trim()
        .rsplit('.')
        .next()
        .unwrap_or(value)
        .trim_matches(|c| matches!(c, '\"' | '\'' | '`' | '[' | ']'))
        .to_ascii_lowercase();
    name.starts_with("rag_kb_") || name.starts_with("kb_")
}
