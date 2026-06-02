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
    CompiledRetrievalProfile, RagConfigRepository, RagEmbeddingCall, RagEmbeddingResult,
    RagIngestAccessContext, RagSearchTraceWrite, RagStrategyRegistry, compile_retrieval_profile,
    merge_search_filters,
    search_observability::{persist_trace_if_enabled, publish_route_events, publish_search_event},
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

impl<R, S, E> RagSearchService<R, S, E>
where
    R: RagSearchRepository,
    S: RagStrategyRegistry,
    E: RagSearchRuntime,
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

    pub async fn search(&self, request: RagSearchRequest) -> Result<RagSearchResult, RagError> {
        validate_request(&request)?;
        publish_search_event(&self.event_bus, "rag.search.started", &request);
        let profile = self
            .repository
            .get_profile(&request.profile_id)?
            .ok_or(RagError::RetrievalProfileNotFound)?;
        let corpus = self
            .repository
            .get_corpus(
                profile
                    .primary_corpus_id
                    .as_deref()
                    .ok_or(RagError::RetrievalProfileInvalid)?,
            )?
            .ok_or(RagError::CorpusNotFound)?;
        if corpus.status != RagStatus::Enabled {
            return Err(RagError::RetrievalProfileInvalid);
        }
        let compiled = compile_retrieval_profile(&profile, &corpus, &self.strategy_registry)?;
        if !compiled
            .supported_context_strategies
            .contains(&request.context.strategy)
        {
            return Err(RagError::RetrievalProfileInvalid);
        }
        let route_corpora = self.load_route_corpora(&compiled)?;
        validate_return_fields(&request.context.return_fields, &route_corpora)?;
        let (mut ranked_hits, mut warnings) = self
            .execute_routes(&compiled, &route_corpora, &profile.filters, &request)
            .await?;
        ranked_hits.truncate(request.limit);
        self.expand_final_graph_hits(&mut ranked_hits, &compiled, &request, &route_corpora)?;
        let context_pack = assemble_context_pack(
            &request.context.strategy,
            &ranked_hits,
            request.token_budget,
        );
        publish_route_events(&self.event_bus, &ranked_hits, &request);
        let trace_code = persist_trace_if_enabled(
            &self.repository,
            &request,
            &compiled,
            &ranked_hits,
            context_pack.as_ref(),
            &mut warnings,
        )?;
        publish_search_event(&self.event_bus, "rag.search.completed", &request);
        let hits = ranked_hits
            .into_iter()
            .map(|candidate| candidate.result)
            .collect();

        Ok(RagSearchResult {
            profile_id: compiled.profile_id,
            query: request.query,
            context_strategy: request.context.strategy,
            hits,
            context_pack,
            warnings,
            trace_code,
        })
    }

    fn load_route_corpora(
        &self,
        compiled: &CompiledRetrievalProfile,
    ) -> Result<BTreeMap<String, crate::domain::rag::RagCorpusRecord>, RagError> {
        let mut corpora = BTreeMap::new();
        for corpus_id in &compiled.corpus_ids {
            let corpus = self
                .repository
                .get_corpus(corpus_id)?
                .ok_or(RagError::CorpusNotFound)?;
            if corpus.status != RagStatus::Enabled {
                return Err(RagError::RetrievalProfileInvalid);
            }
            corpora.insert(corpus_id.clone(), corpus);
        }
        Ok(corpora)
    }

    async fn execute_routes(
        &self,
        compiled: &CompiledRetrievalProfile,
        route_corpora: &BTreeMap<String, crate::domain::rag::RagCorpusRecord>,
        profile_filters: &Value,
        request: &RagSearchRequest,
    ) -> Result<(Vec<RankedCandidate>, Vec<Value>), RagError> {
        let blank_query = request.query.trim().is_empty();
        let mut executable_routes = Vec::new();
        for route in &compiled.routes {
            let strategy = retriever_strategy(&route.retriever)?;
            if blank_query && !strategy.can_run_without_query() {
                continue;
            }
            if strategy.is_structured_filter() {
                let corpus = route_corpora
                    .get(&route.corpus_id)
                    .ok_or(RagError::CorpusNotFound)?;
                if corpus.storage_binding.layout
                    == crate::domain::rag::RagStorageLayout::GraphSingleNode
                {
                    continue;
                }
            }
            executable_routes.push((route, strategy));
        }
        if blank_query && executable_routes.is_empty() {
            return Err(RagError::SearchRequestInvalid);
        }
        let request_filter_corpora = executable_routes
            .iter()
            .filter_map(|(route, _)| route_corpora.get(&route.corpus_id))
            .collect::<Vec<_>>();
        validate_request_filters_for_corpora(&request_filter_corpora, &request.filters)?;

        let embedding_routes = executable_routes
            .iter()
            .filter_map(|(route, strategy)| strategy.needs_query_embedding().then_some(*route))
            .collect::<Vec<_>>();
        let mut warnings = Vec::new();
        let query_embedding = if !embedding_routes.is_empty() {
            match self
                .runtime
                .embed(RagEmbeddingCall {
                    workspace_code: request.workspace_code.clone(),
                    model_code: compiled.embedding_model_code.clone(),
                    input: request.query.clone(),
                })
                .await
            {
                Ok(embedding) => Some(embedding),
                Err(_) if compiled.degrade_policy == "FAIL_FAST" => {
                    return Err(RagError::SearchRouteUnavailable);
                }
                Err(_) => {
                    warnings.extend(embedding_routes.iter().filter_map(|route| {
                        retriever_strategy(&route.retriever).ok().map(|strategy| {
                            json!({
                                "routeId": route.route_id,
                                "code": strategy.unavailable_code(),
                                "message": strategy.unavailable_message()
                            })
                        })
                    }));
                    None
                }
            }
        } else {
            None
        };
        let mut route_hits = Vec::new();
        for (route, strategy) in executable_routes {
            if strategy.should_skip_without_embedding() && query_embedding.is_none() {
                continue;
            }
            let corpus = route_corpora
                .get(&route.corpus_id)
                .ok_or(RagError::CorpusNotFound)?;
            let route_profile_filters =
                merge_search_filters(corpus, profile_filters, &route.filters)?;
            let (filters, source_version) =
                filters_and_version(corpus, &route_profile_filters, request, compiled)?;
            let query = RagRouteSearchQuery {
                corpus_id: route.corpus_id.clone(),
                route_id: route.route_id.clone(),
                retriever: strategy.storage_retriever.to_string(),
                query: request.query.clone(),
                workspace_code: request.workspace_code.clone(),
                query_embedding: (strategy.needs_query_embedding())
                    .then(|| {
                        query_embedding
                            .as_ref()
                            .map(|result| result.embedding.clone())
                    })
                    .flatten(),
                query_embedding_model_code: (strategy.needs_query_embedding())
                    .then(|| {
                        query_embedding
                            .as_ref()
                            .map(|result| result.model_code.clone())
                    })
                    .flatten(),
                source_version,
                scopes: search_scopes(&request.access_context),
                filters,
                return_fields: request.context.return_fields.clone(),
                limit: route.top_k.max(request.limit).max(1) * 4,
            };
            let rows = match self.repository.search_route(query) {
                Ok(rows) => rows,
                Err(RagError::SearchRouteUnavailable) if compiled.degrade_policy != "FAIL_FAST" => {
                    warnings.push(json!({
                        "routeId": route.route_id,
                        "code": strategy.unavailable_code(),
                        "message": strategy.unavailable_message()
                    }));
                    continue;
                }
                Err(error) => return Err(error),
            };
            route_hits.extend(rank_route_rows(
                rows,
                corpus,
                route,
                &strategy,
                request,
                query_embedding
                    .as_ref()
                    .map(|result| result.embedding.as_slice()),
            ));
        }
        let mut hits = fuse_and_dedup(&compiled, route_hits);
        hits.sort_by(stable_hit_order);
        apply_topic_policy(&compiled, &mut hits);
        if !blank_query && compiled.rerank_enabled && hits.len() > 1 {
            apply_rerank(&self.runtime, compiled, request, &mut hits).await?;
        }
        Ok((hits, warnings))
    }

    fn expand_final_graph_hits(
        &self,
        hits: &mut [RankedCandidate],
        compiled: &CompiledRetrievalProfile,
        request: &RagSearchRequest,
        route_corpora: &BTreeMap<String, crate::domain::rag::RagCorpusRecord>,
    ) -> Result<(), RagError> {
        let Some(policy) = compiled.graph_expansion_policy.as_ref() else {
            return Ok(());
        };
        if !policy
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return Ok(());
        }
        for hit in hits {
            let Some(corpus) = route_corpora.get(&hit.result.corpus_id) else {
                continue;
            };
            if !corpus.storage_binding.is_graph_backend() {
                continue;
            }
            let expanded = self
                .repository
                .expand_related_graph(RagGraphExpansionQuery {
                    corpus_id: hit.result.corpus_id.clone(),
                    source_code: hit.result.source_code.clone(),
                    source_version: hit.result.source_version.clone(),
                    target_code: hit.target_code.clone(),
                    scope_type: hit.scope_type.clone(),
                    scope_code: hit.scope_code.clone(),
                    workspace_code: request.workspace_code.clone(),
                    relationships: request.context.graph_expansion.relationships.clone(),
                    policy: policy.clone(),
                })?;
            if let Some(expanded) = expanded {
                let mut metadata = hit.result.metadata.as_object().cloned().unwrap_or_default();
                metadata.insert("relatedGraph".into(), expanded);
                hit.result.metadata = Value::Object(metadata);
            }
        }
        Ok(())
    }
}
