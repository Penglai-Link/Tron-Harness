use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use serde_json::{Value, json};

use crate::{
    application::llm::{LlmRerankCallCommand, LlmRuntimeService},
    domain::rag::{RagError, RagStatus},
    shared::event_bus::EventBusHandle,
};

use super::{
    CompiledRetrievalProfile, RagConfigRepository, RagEmbeddingCall, RagIngestAccessContext,
    RagSearchTraceWrite, RagStrategyRegistry, compile_retrieval_profile, merge_search_filters,
    search_observability::{persist_trace_if_enabled, publish_route_events, publish_search_event},
    search_ranking::{
        apply_rerank, assemble_context_pack, fuse_and_dedup, rank_route_rows, stable_hit_order,
    },
    search_strategy::retriever_strategy,
};

pub trait RagSearchRepository: RagConfigRepository {
    fn search_route(&self, query: RagRouteSearchQuery) -> Result<Vec<RagSearchRow>, RagError>;
    fn persist_search_trace(&self, trace: RagSearchTraceWrite) -> Result<(), RagError>;
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagRouteSearchQuery {
    pub corpus_id: String,
    pub route_id: String,
    pub retriever: String,
    pub query: String,
    pub query_embedding: Option<Vec<f32>>,
    pub source_version: String,
    pub scopes: Vec<RagSearchScope>,
    pub filters: Value,
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
    pub text: String,
    pub search_text: String,
    pub heading_path: Vec<String>,
    pub ordinal: i64,
    pub token_count: i64,
    pub embedding: Option<Vec<f32>>,
    pub storage_score: Option<f64>,
    pub source_metadata: Value,
    pub text_metadata: Value,
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
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>>;

    fn rerank<'a>(
        &'a self,
        call: RagRerankCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(usize, f64)>, String>> + Send + 'a>>;
}

impl RagSearchRuntime for LlmRuntimeService {
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>> {
        Box::pin(async move {
            <Self as super::RagEmbeddingRuntime>::embed(self, call)
                .await
                .map(|result| result.embedding)
        })
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
    pub target_type: String,
    pub target_code: String,
    pub route_ids: Vec<String>,
    pub score: f64,
    pub scope_type: String,
    pub scope_code: String,
    pub preview: String,
    pub text: String,
    pub token_count: i64,
    pub metadata: Value,
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
        let (mut ranked_hits, mut warnings) = self
            .execute_routes(&compiled, &route_corpora, &profile.filters, &request)
            .await?;
        ranked_hits.truncate(request.limit);
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

        Ok(RagSearchResult {
            profile_id: compiled.profile_id,
            query: request.query,
            context_strategy: request.context.strategy,
            hits: ranked_hits,
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
    ) -> Result<(Vec<CandidateHit>, Vec<Value>), RagError> {
        let embedding_routes = compiled
            .routes
            .iter()
            .filter_map(|route| {
                retriever_strategy(&route.retriever)
                    .ok()
                    .filter(|strategy| strategy.needs_query_embedding())
                    .map(|_| route)
            })
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
        for route in &compiled.routes {
            let strategy = retriever_strategy(&route.retriever)?;
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
                query_embedding: (strategy.needs_query_embedding())
                    .then(|| query_embedding.clone())
                    .flatten(),
                source_version,
                scopes: search_scopes(&request.access_context),
                filters,
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
                route,
                &strategy,
                request,
                query_embedding.as_deref(),
            ));
        }
        let mut hits = fuse_and_dedup(route_hits);
        hits.sort_by(stable_hit_order);
        if compiled.rerank_enabled && hits.len() > 1 {
            apply_rerank(&self.runtime, compiled, request, &mut hits).await?;
        }
        Ok((hits, warnings))
    }
}

fn validate_request(request: &RagSearchRequest) -> Result<(), RagError> {
    for value in [
        request.profile_id.as_str(),
        request.workspace_code.as_str(),
        request.query.as_str(),
        request.access_context.user_code.as_str(),
        request.access_context.agent_code.as_str(),
        request.access_context.workspace_code.as_str(),
        request.context.strategy.as_str(),
    ] {
        if value.trim().is_empty() {
            return Err(RagError::SearchRequestInvalid);
        }
    }
    if request.workspace_code != request.access_context.workspace_code || request.limit == 0 {
        return Err(RagError::SearchRequestInvalid);
    }
    Ok(())
}

fn filters_and_version(
    corpus: &crate::domain::rag::RagCorpusRecord,
    profile_filters: &Value,
    request: &RagSearchRequest,
    compiled: &CompiledRetrievalProfile,
) -> Result<(Value, String), RagError> {
    let merged = merge_search_filters(corpus, profile_filters, &request.filters)?;
    let source_version = merged
        .get("sourceVersion")
        .map(source_version_filter_value)
        .transpose()?
        .unwrap_or_else(|| compiled.required_source_version.clone());
    Ok((merged, source_version))
}

fn source_version_filter_value(value: &Value) -> Result<String, RagError> {
    let raw = value.get("value").ok_or(RagError::SearchFilterInvalid)?;
    if let Some(version) = raw.as_str() {
        return Ok(version.to_string());
    }
    let versions = raw.as_array().ok_or(RagError::SearchFilterInvalid)?;
    match versions.as_slice() {
        [version] => version
            .as_str()
            .map(ToOwned::to_owned)
            .ok_or(RagError::SearchFilterInvalid),
        _ => Err(RagError::SearchFilterInvalid),
    }
}

fn search_scopes(access_context: &RagIngestAccessContext) -> Vec<RagSearchScope> {
    vec![
        RagSearchScope {
            scope_type: "USER".into(),
            scope_code: access_context.user_code.clone(),
        },
        RagSearchScope {
            scope_type: "AGENT".into(),
            scope_code: access_context.agent_code.clone(),
        },
        RagSearchScope {
            scope_type: "SPACE".into(),
            scope_code: access_context.workspace_code.clone(),
        },
    ]
}
