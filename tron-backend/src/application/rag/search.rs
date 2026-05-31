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
};

pub trait RagSearchRepository: RagConfigRepository {
    fn search_text_units(
        &self,
        query: RagTextUnitSearchQuery,
    ) -> Result<Vec<RagSearchRow>, RagError>;
    fn persist_search_trace(&self, trace: RagSearchTraceWrite) -> Result<(), RagError>;
}

impl<T> RagSearchRepository for Arc<Mutex<T>>
where
    T: RagSearchRepository,
{
    fn search_text_units(
        &self,
        query: RagTextUnitSearchQuery,
    ) -> Result<Vec<RagSearchRow>, RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .search_text_units(query)
    }

    fn persist_search_trace(&self, trace: RagSearchTraceWrite) -> Result<(), RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .persist_search_trace(trace)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagTextUnitSearchQuery {
    pub corpus_id: String,
    pub query: String,
    pub source_version: String,
    pub scopes: Vec<RagSearchScope>,
    pub filters: Value,
    pub limit: usize,
    pub include_vectors: bool,
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
        <Self as super::RagEmbeddingRuntime>::embed(self, call)
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
        let (filters, source_version) =
            filters_and_version(&corpus, &profile.filters, &request, &compiled)?;
        let include_vectors = compiled
            .routes
            .iter()
            .any(|route| route.retriever == "VECTOR");
        let rows = self.repository.search_text_units(RagTextUnitSearchQuery {
            corpus_id: compiled.corpus_id.clone(),
            query: request.query.clone(),
            source_version,
            scopes: search_scopes(&request.access_context),
            filters,
            limit: request.limit.max(1) * 4,
            include_vectors,
        })?;
        let (mut ranked_hits, mut warnings) = self.rank_rows(rows, &compiled, &request).await?;
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

    async fn rank_rows(
        &self,
        rows: Vec<RagSearchRow>,
        compiled: &CompiledRetrievalProfile,
        request: &RagSearchRequest,
    ) -> Result<(Vec<CandidateHit>, Vec<Value>), RagError> {
        let vector_routes = compiled
            .routes
            .iter()
            .filter(|route| route.retriever == "VECTOR")
            .collect::<Vec<_>>();
        let mut warnings = Vec::new();
        let query_embedding = if !vector_routes.is_empty() {
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
                    warnings.extend(vector_routes.iter().map(|route| {
                        json!({
                            "routeId": route.route_id,
                            "code": "RAG_VECTOR_ROUTE_UNAVAILABLE",
                            "message": "Vector route skipped; fulltext route remains available"
                        })
                    }));
                    None
                }
            }
        } else {
            None
        };
        let mut by_target = BTreeMap::<String, CandidateHit>::new();
        for row in rows {
            let route_scores = route_scores(&row, compiled, request, query_embedding.as_deref());
            for (route_id, score) in route_scores {
                let entry = by_target
                    .entry(row.target_code.clone())
                    .or_insert_with(|| hit_from_row(&row));
                if !entry.route_ids.contains(&route_id) {
                    entry.route_ids.push(route_id);
                }
                entry.score += score;
            }
        }
        let mut hits = by_target.into_values().collect::<Vec<_>>();
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

fn route_scores(
    row: &RagSearchRow,
    compiled: &CompiledRetrievalProfile,
    request: &RagSearchRequest,
    query_embedding: Option<&[f32]>,
) -> Vec<(String, f64)> {
    compiled
        .routes
        .iter()
        .filter_map(|route| match route.retriever.as_str() {
            "FULLTEXT" if fulltext_matches(&row.search_text, &request.query) => Some((
                route.route_id.clone(),
                fulltext_score(&row.search_text, &request.query),
            )),
            "VECTOR" => row.embedding.as_deref().and_then(|embedding| {
                query_embedding
                    .map(|query| (route.route_id.clone(), cosine_similarity(query, embedding)))
            }),
            _ => None,
        })
        .collect()
}

fn fulltext_matches(text: &str, query: &str) -> bool {
    let text = text.to_ascii_lowercase();
    query
        .split_whitespace()
        .any(|term| text.contains(&term.to_ascii_lowercase()))
}

fn fulltext_score(text: &str, query: &str) -> f64 {
    query
        .split_whitespace()
        .filter(|term| {
            text.to_ascii_lowercase()
                .contains(&term.to_ascii_lowercase())
        })
        .count() as f64
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f64 {
    let (mut dot, mut left_norm, mut right_norm) = (0.0f64, 0.0f64, 0.0f64);
    for (left, right) in left.iter().zip(right.iter()) {
        let left = *left as f64;
        let right = *right as f64;
        dot += left * right;
        left_norm += left * left;
        right_norm += right * right;
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        dot / (left_norm.sqrt() * right_norm.sqrt())
    }
}

fn hit_from_row(row: &RagSearchRow) -> CandidateHit {
    let mut metadata = row.source_metadata.clone();
    if let (Some(target), Some(source)) = (metadata.as_object_mut(), row.text_metadata.as_object())
    {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
    CandidateHit {
        corpus_id: row.corpus_id.clone(),
        source_code: row.source_code.clone(),
        source_version: row.source_version.clone(),
        target_type: "TEXT_UNIT".into(),
        target_code: row.target_code.clone(),
        route_ids: Vec::new(),
        score: 0.0,
        scope_type: row.scope_type.clone(),
        scope_code: row.scope_code.clone(),
        preview: row.text.chars().take(160).collect(),
        text: row.text.clone(),
        token_count: row.token_count,
        metadata,
    }
}

async fn apply_rerank<E>(
    runtime: &E,
    compiled: &CompiledRetrievalProfile,
    request: &RagSearchRequest,
    hits: &mut Vec<CandidateHit>,
) -> Result<(), RagError>
where
    E: RagSearchRuntime,
{
    let documents = hits.iter().map(|hit| hit.text.clone()).collect::<Vec<_>>();
    let order = runtime
        .rerank(RagRerankCall {
            workspace_code: request.workspace_code.clone(),
            model_code: compiled.rerank_model_code.clone(),
            query: request.query.clone(),
            documents,
            top_n: Some(request.limit),
        })
        .await
        .map_err(|_| RagError::RerankFailed)?;
    if order.is_empty() {
        return Ok(());
    }
    let mut ranked = Vec::new();
    for (index, score) in order {
        if let Some(mut hit) = hits.get(index).cloned() {
            hit.score = score;
            ranked.push(hit);
        }
    }
    if !ranked.is_empty() {
        *hits = ranked;
    }
    Ok(())
}

fn stable_hit_order(left: &CandidateHit, right: &CandidateHit) -> std::cmp::Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| left.source_code.cmp(&right.source_code))
        .then_with(|| left.target_code.cmp(&right.target_code))
}

fn assemble_context_pack(
    strategy: &str,
    hits: &[CandidateHit],
    token_budget: usize,
) -> Option<ContextPack> {
    if strategy == "RESULT_LIST" {
        return None;
    }
    let mut token_count = 0usize;
    let mut items = Vec::new();
    for hit in hits {
        let tokens =
            (hit.token_count.max(1) as usize).min(token_budget.saturating_sub(token_count));
        if tokens == 0 {
            break;
        }
        token_count += tokens;
        items.push(ContextPackItem {
            source_code: hit.source_code.clone(),
            text: hit.text.clone(),
            tokens,
            metadata: json!({
                "strategy": strategy,
                "targetCode": hit.target_code,
                "metadata": hit.metadata
            }),
        });
    }
    Some(ContextPack { items, token_count })
}
