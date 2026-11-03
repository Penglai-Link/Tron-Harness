use super::{
    RagConfigRepository, RagEmbeddingCall, RagEmbeddingResult, RagIngestAccessContext,
    RagSearchTraceWrite,
};
use crate::{
    application::llm::{LlmRerankCallCommand, LlmRuntimeService},
    domain::rag::RagError,
};
use serde_json::{Map, Value};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

pub trait RagSearchRepository: RagConfigRepository {
    /// Invalidate reusable computations after an observed provider-model change;
    /// published evidence and its source pointer remain intact until a rebuild.
    fn invalidate_vector_cache(&self, query: RagRouteSearchQuery) -> Result<(), RagError>;
    fn published_graph_generations(
        &self,
        query: RagRouteSearchQuery,
    ) -> Result<Vec<String>, RagError>;
    fn embedding_partitions(
        &self,
        query: RagRouteSearchQuery,
    ) -> Result<Vec<RagEmbeddingPartition>, RagError>;
    fn search_route(&self, query: RagRouteSearchQuery) -> Result<Vec<RagSearchRow>, RagError>;
    fn persist_search_trace(&self, trace: RagSearchTraceWrite) -> Result<(), RagError>;
}

impl<T> RagSearchRepository for Arc<Mutex<T>>
where
    T: RagSearchRepository,
{
    fn invalidate_vector_cache(&self, query: RagRouteSearchQuery) -> Result<(), RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .invalidate_vector_cache(query)
    }
    fn published_graph_generations(
        &self,
        query: RagRouteSearchQuery,
    ) -> Result<Vec<String>, RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .published_graph_generations(query)
    }
    fn embedding_partitions(
        &self,
        query: RagRouteSearchQuery,
    ) -> Result<Vec<RagEmbeddingPartition>, RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .embedding_partitions(query)
    }
    fn search_route(&self, query: RagRouteSearchQuery) -> Result<Vec<RagSearchRow>, RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .search_route(query)
    }

    fn persist_search_trace(&self, trace: RagSearchTraceWrite) -> Result<(), RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .persist_search_trace(trace)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagRouteSearchQuery {
    pub corpus_id: String,
    pub route_id: String,
    pub retriever: String,
    pub workspace_code: String,
    pub query: String,
    pub query_embedding: Option<Vec<f32>>,
    pub query_embedding_model_code: Option<String>,
    pub source_version: String,
    pub scopes: Vec<RagSearchScope>,
    pub filters: Value,
    pub policy: Value,
    pub return_fields: Vec<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagSearchScope {
    pub scope_type: String,
    pub scope_code: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagSearchRow {
    pub corpus_id: String,
    pub source_code: String,
    pub source_version: String,
    pub target_code: String,
    pub scope_type: String,
    pub scope_code: String,
    pub title: Option<String>,
    pub text: String,
    pub search_text: String,
    pub heading_path: Vec<String>,
    pub ordinal: i64,
    pub token_count: i64,
    pub embedding: Option<Vec<f32>>,
    pub storage_score: Option<f64>,
    pub source_metadata: Value,
    pub text_metadata: Value,
    pub field_values: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagRerankCall {
    pub workspace_code: String,
    pub model_code: Option<String>,
    pub query: String,
    pub documents: Vec<String>,
    pub top_n: Option<usize>,
}

pub trait RagSearchRuntime: Clone + Send + Sync + 'static {
    fn embedding_fingerprint(
        &self,
        _workspace: &str,
        _model: Option<&str>,
        _dimension: usize,
    ) -> Result<Option<String>, String> {
        Ok(None)
    }
    fn embed_for_retrieval<'a>(
        &'a self,
        call: RagEmbeddingCall,
        _dimensions: Option<usize>,
        _run: Option<&'a eliteforge_agent_framework::RunContext>,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        self.embed(call)
    }
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>>;

    fn rerank<'a>(
        &'a self,
        call: RagRerankCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(usize, f64)>, String>> + Send + 'a>>;
}

impl RagSearchRuntime for LlmRuntimeService {
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move { <Self as super::RagEmbeddingRuntime>::embed(self, call).await })
    }

    fn rerank<'a>(
        &'a self,
        call: RagRerankCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(usize, f64)>, String>> + Send + 'a>> {
        Box::pin(async move {
            self.rerank_call(LlmRerankCallCommand {
                provider_id: None,
                required_modalities: vec![],
                workspace_code: call.workspace_code,
                model_code: call.model_code,
                query: call.query,
                documents: call.documents,
                top_n: call.top_n,
            })
            .await
            .map(|result| {
                result
                    .items
                    .into_iter()
                    .map(|item| (item.document_index, item.score))
                    .collect()
            })
            .map_err(|error| error.to_string())
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagSearchRequest {
    pub workspace_code: String,
    pub query: String,
    pub access_context: RagIngestAccessContext,
    pub filters: Value,
    pub limit: usize,
    pub token_budget: usize,
    pub context: RagSearchContextRequest,
    pub trace: RagSearchTraceRequest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagSearchContextRequest {
    pub strategy: String,
    pub return_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RagSearchTraceRequest {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagSearchResult {
    pub query: String,
    pub context_strategy: String,
    pub hits: Vec<CandidateHit>,
    pub context_pack: Option<ContextPack>,
    pub warnings: Vec<Value>,
    pub trace_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CandidateHit {
    pub corpus_id: String,
    pub source_code: String,
    pub source_version: String,
    pub scope_type: String,
    pub scope_code: String,
    pub title: Option<String>,
    pub score: f64,
    pub text: String,
    pub metadata: Value,
    pub fields: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextPack {
    pub items: Vec<ContextPackItem>,
    pub token_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextPackItem {
    pub source_code: String,
    pub text: String,
    pub tokens: usize,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct RagEmbeddingPartition {
    pub source_code: String,
    pub model_code: String,
    pub dimension: usize,
    pub provider_model: Option<String>,
    pub fingerprint: Option<String>,
}
