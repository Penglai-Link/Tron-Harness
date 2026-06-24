use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use serde_json::{Map, Value, json};

use crate::{
    application::llm::{LlmRerankCallCommand, LlmRuntimeService},
    domain::rag::{RagError, RagStatus},
    shared::event_bus::EventBusHandle,
};

use super::{
    CompiledRagPipelineNode, CompiledRetrievalProfile, RagArtifactStore, RagConfigRepository,
    RagEmbeddingCall, RagEmbeddingResult, RagIngestAccessContext, RagPipelineExecutor,
    RagSearchTraceWrite, RagStageExecutionResult, RagStageOutcome, RagStrategyRegistry,
    compile_retrieval_profile, merge_search_filters,
    search_observability::{
        persist_trace_if_enabled, publish_route_events, publish_route_skipped_event,
        publish_search_completed_event, publish_search_failed_event, publish_search_started_event,
    },
    search_ranking::{
        apply_rerank, apply_topic_policy, assemble_context_pack, fuse_and_dedup, rank_route_rows,
        stable_hit_order,
    },
    search_strategy::retriever_strategy,
    search_validation::{
        filters_and_version, search_scopes, validate_request, validate_return_fields,
    },
    validate_request_filters_for_corpora,
};

pub trait RagSearchRepository: RagConfigRepository {
    fn search_route(&self, query: RagRouteSearchQuery) -> Result<Vec<RagSearchRow>, RagError>;
    fn persist_search_trace(&self, trace: RagSearchTraceWrite) -> Result<(), RagError>;
    fn expand_related_graph(
        &self,
        _query: RagGraphExpansionQuery,
    ) -> Result<Option<Value>, RagError> {
        Ok(None)
    }
}

fn effective_query(context: &RagRetrievalPipelineContext<'_>) -> Result<String, RagError> {
    for key in [
        "query.rewritten",
        "query.generalized",
        "query.effective",
        "query.original",
    ] {
        if let Some(value) = context.artifacts.get_optional_cloned::<Value>(key)? {
            if let Some(query) = value.as_str() {
                return Ok(query.to_string());
            }
        }
    }
    Err(RagError::RetrievalProfileInvalid)
}

fn query_contains_phrase(query: &str, phrase: &str) -> bool {
    let query = query.to_ascii_lowercase();
    let phrase = phrase.to_ascii_lowercase();
    query
        .split_whitespace()
        .collect::<Vec<_>>()
        .windows(phrase.split_whitespace().count())
        .any(|window| window.join(" ") == phrase)
}

impl<T> RagSearchRepository for Arc<Mutex<T>>
where
    T: RagSearchRepository,
{
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

    fn expand_related_graph(
        &self,
        query: RagGraphExpansionQuery,
    ) -> Result<Option<Value>, RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .expand_related_graph(query)
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

#[derive(Debug, Clone, PartialEq)]
pub struct RagGraphExpansionQuery {
    pub corpus_id: String,
    pub source_code: String,
    pub source_version: String,
    pub target_code: String,
    pub scope_type: String,
    pub scope_code: String,
    pub workspace_code: String,
    pub relationships: Vec<String>,
    pub policy: Value,
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
    pub profile_id: String,
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
    pub graph_expansion: RagSearchGraphExpansionRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RagSearchGraphExpansionRequest {
    pub relationships: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RagSearchTraceRequest {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagSearchResult {
    pub profile_id: String,
    pub query: String,
    pub context_strategy: String,
    pub hits: Vec<CandidateHit>,
    pub context_pack: Option<ContextPack>,
    pub warnings: Vec<Value>,
    pub trace_code: Option<String>,
    pub stage_results: Vec<RagStageExecutionResult>,
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
pub(super) struct RankedCandidate {
    pub result: CandidateHit,
    pub target_code: String,
    pub scope_type: String,
    pub scope_code: String,
    pub route_ids: Vec<String>,
    pub route_ranks: Vec<CandidateRouteRank>,
    pub token_count: i64,
    pub embedding: Option<Vec<f32>>,
    pub identity_key: String,
    pub canonical_dedup_key: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CandidateRouteRank {
    pub route_id: String,
    pub rank: usize,
    pub weight: f64,
}

struct RagRetrievalPipelineContext<'a> {
    route_corpora: &'a BTreeMap<String, crate::domain::rag::RagCorpusRecord>,
    profile_filters: &'a Value,
    request: &'a RagSearchRequest,
    artifacts: RagArtifactStore,
    query_embedding: Option<RagEmbeddingResult>,
    warnings: Vec<Value>,
}

impl<'a> RagRetrievalPipelineContext<'a> {
    fn new(
        route_corpora: &'a BTreeMap<String, crate::domain::rag::RagCorpusRecord>,
        profile_filters: &'a Value,
        request: &'a RagSearchRequest,
    ) -> Self {
        Self {
            route_corpora,
            profile_filters,
            request,
            artifacts: RagArtifactStore::default(),
            query_embedding: None,
            warnings: Vec::new(),
        }
    }
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

pub struct RagSearchService<R, S, E> {
    repository: R,
    strategy_registry: S,
    runtime: E,
    event_bus: EventBusHandle,
}
