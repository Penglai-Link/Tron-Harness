use serde_json::{Value, json};

use crate::domain::rag::{RagCorpusRecord, RagError, RagProfileRecord, RagProfileType, RagStatus};

use super::search_strategy::{RetrieverKind, retriever_strategy};
use super::{RagStrategyRegistry, StrategyRegistrationStatus};

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledRetrievalProfile {
    pub profile_id: String,
    pub corpus_id: String,
    pub corpus_ids: Vec<String>,
    pub required_source_version: String,
    pub routes: Vec<RetrievalRoute>,
    pub supported_context_strategies: Vec<String>,
    pub embedding_model_code: Option<String>,
    pub rerank_model_code: Option<String>,
    pub rerank_enabled: bool,
    pub fusion_policy: FusionPolicy,
    pub dedup_policy: DedupPolicy,
    pub topic_policy: TopicPolicy,
    pub graph_expansion_policy: Option<Value>,
    pub degrade_policy: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetrievalRoute {
    pub route_id: String,
    pub corpus_id: String,
    pub retriever: String,
    pub top_k: usize,
    pub weight: f64,
    pub filters: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FusionPolicy {
    SumScore,
    Rrf { rank_constant: f64 },
}

impl FusionPolicy {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::SumScore => "SUM_SCORE",
            Self::Rrf { .. } => "RRF",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DedupPolicy {
    TextUnit,
    CanonicalTextUnit,
}

impl DedupPolicy {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::TextUnit => "TEXT_UNIT",
            Self::CanonicalTextUnit => "CANONICAL_TEXT_UNIT",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TopicPolicy {
    Disabled,
    EmbeddingMmr(EmbeddingMmrPolicy),
}

impl TopicPolicy {
    pub fn trace_type_name(&self) -> &'static str {
        match self {
            Self::Disabled => "DISABLED",
            Self::EmbeddingMmr(_) => "VECTOR_MMR_V1",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddingMmrPolicy {
    pub candidate_window: usize,
    pub lambda: f64,
    pub similarity_threshold: f64,
    pub max_per_topic: usize,
    pub fallback: TopicFallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopicFallback {
    KeepOrder,
}

pub fn compile_retrieval_profile<S>(
    profile: &RagProfileRecord,
    corpus: &RagCorpusRecord,
    registry: &S,
) -> Result<CompiledRetrievalProfile, RagError>
where
    S: RagStrategyRegistry,
{
    if profile.profile_type != RagProfileType::Retrieval
        || profile.status != RagStatus::Enabled
        || profile.primary_corpus_id.as_deref() != Some(corpus.corpus_id.as_str())
        || !profile.corpus_ids.contains(&corpus.corpus_id)
    {
        return Err(RagError::RetrievalProfileInvalid);
    }
    let required_source_version = profile
        .required_source_version
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("latest"))
        .ok_or(RagError::RetrievalProfileInvalid)?
        .to_string();
    let routes = parse_routes(profile, &corpus.corpus_id)?;
    if routes.is_empty() {
        return Err(RagError::RetrievalProfileInvalid);
    }
    for route in &routes {
        if registry.resolve_retriever(&route.retriever).status
            != StrategyRegistrationStatus::Enabled
        {
            return Err(RagError::RetrievalProfileInvalid);
        }
    }

    let supported_context_strategies = parse_context_strategies(&profile.config)?;
    let rerank_enabled = profile
        .config
        .get("rerankPolicy")
        .and_then(Value::as_object)
        .and_then(|policy| policy.get("enabled"))
        .and_then(Value::as_bool)
        .or_else(|| {
            profile
                .config
                .get("rerank")
                .and_then(Value::as_str)
                .map(|value| value != "DISABLED")
        })
        .unwrap_or(false);

    Ok(CompiledRetrievalProfile {
        profile_id: profile.profile_id.clone(),
        corpus_id: corpus.corpus_id.clone(),
        corpus_ids: profile.corpus_ids.clone(),
        required_source_version,
        routes,
        supported_context_strategies,
        embedding_model_code: string_config(&profile.config, "embeddingModelCode"),
        rerank_model_code: string_config(&profile.config, "rerankModelCode"),
        rerank_enabled,
        fusion_policy: parse_fusion_policy(&profile.config)?,
        dedup_policy: parse_dedup_policy(&profile.config)?,
        topic_policy: parse_topic_policy(&profile.config)?,
        graph_expansion_policy: profile.config.get("graphExpansionPolicy").cloned(),
        degrade_policy: profile.degrade_policy.clone(),
    })
}

fn parse_routes(
    profile: &RagProfileRecord,
    corpus_id: &str,
) -> Result<Vec<RetrievalRoute>, RagError> {
    if let Some(items) = profile.config.get("routes").and_then(Value::as_array) {
        return items
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let object = value.as_object().ok_or(RagError::RetrievalProfileInvalid)?;
                let retriever = object
                    .get("retriever")
                    .or_else(|| object.get("retrieverId"))
                    .and_then(Value::as_str)
                    .ok_or(RagError::RetrievalProfileInvalid)?;
                Ok(RetrievalRoute {
                    route_id: object
                        .get("routeId")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                        .unwrap_or_else(|| default_route_id(corpus_id, retriever, index)),
                    corpus_id: object
                        .get("corpusId")
                        .and_then(Value::as_str)
                        .unwrap_or(corpus_id)
                        .to_string(),
                    retriever: retriever.to_string(),
                    top_k: object
                        .get("topK")
                        .and_then(Value::as_u64)
                        .map(|value| value as usize)
                        .filter(|value| *value > 0)
                        .unwrap_or(20),
                    weight: object
                        .get("weight")
                        .and_then(Value::as_f64)
                        .filter(|value| *value > 0.0)
                        .unwrap_or(1.0),
                    filters: object.get("filters").cloned().unwrap_or_else(|| json!({})),
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .and_then(|routes| validate_route_corpora(profile, routes));
    }

    profile
        .config
        .get("retrievers")
        .and_then(Value::as_array)
        .ok_or(RagError::RetrievalProfileInvalid)?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let retriever = value.as_str().ok_or(RagError::RetrievalProfileInvalid)?;
            Ok(RetrievalRoute {
                route_id: default_route_id(corpus_id, retriever, index),
                corpus_id: corpus_id.to_string(),
                retriever: retriever.to_string(),
                top_k: 20,
                weight: 1.0,
                filters: json!({}),
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .and_then(|routes| validate_route_corpora(profile, routes))
}

fn validate_route_corpora(
    profile: &RagProfileRecord,
    routes: Vec<RetrievalRoute>,
) -> Result<Vec<RetrievalRoute>, RagError> {
    if routes
        .iter()
        .all(|route| profile.corpus_ids.contains(&route.corpus_id))
    {
        Ok(routes)
    } else {
        Err(RagError::RetrievalProfileInvalid)
    }
}

fn default_route_id(corpus_id: &str, retriever: &str, index: usize) -> String {
    match retriever_strategy(retriever).map(|strategy| strategy.kind) {
        Ok(RetrieverKind::Fulltext) => format!("{corpus_id}-fulltext"),
        Ok(RetrieverKind::Vector) => format!("{corpus_id}-vector"),
        _ => format!("{corpus_id}-route-{index}"),
    }
}

fn parse_context_strategies(config: &Value) -> Result<Vec<String>, RagError> {
    let supported = config
        .get("contextPolicy")
        .and_then(Value::as_object)
        .and_then(|policy| policy.get("supportedStrategies"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            vec![
                "RESULT_LIST".into(),
                "TOKEN_BUDGET_PACK".into(),
                "MEMORY_CONTEXT_PACK".into(),
                "DOC_QA_CONTEXT_PACK".into(),
            ]
        });
    if supported.is_empty()
        || supported
            .iter()
            .any(|strategy| !is_supported_context_strategy(strategy))
    {
        return Err(RagError::RetrievalProfileInvalid);
    }
    Ok(supported)
}

fn is_supported_context_strategy(strategy: &str) -> bool {
    matches!(
        strategy,
        "RESULT_LIST" | "TOKEN_BUDGET_PACK" | "MEMORY_CONTEXT_PACK" | "DOC_QA_CONTEXT_PACK"
    )
}

fn parse_fusion_policy(config: &Value) -> Result<FusionPolicy, RagError> {
    let Some(policy) = config.get("fusionPolicy").and_then(Value::as_object) else {
        return Ok(FusionPolicy::SumScore);
    };
    match policy
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("SUM_SCORE")
    {
        "SUM_SCORE" => Ok(FusionPolicy::SumScore),
        "RRF" => {
            let rank_constant = policy
                .get("rankConstant")
                .and_then(Value::as_f64)
                .filter(|value| *value > 0.0)
                .unwrap_or(60.0);
            Ok(FusionPolicy::Rrf { rank_constant })
        }
        _ => Err(RagError::RetrievalProfileInvalid),
    }
}

fn parse_dedup_policy(config: &Value) -> Result<DedupPolicy, RagError> {
    let Some(policy) = config.get("dedupPolicy").and_then(Value::as_object) else {
        return Ok(DedupPolicy::TextUnit);
    };
    match policy
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("TEXT_UNIT")
    {
        "TEXT_UNIT" => Ok(DedupPolicy::TextUnit),
        "CANONICAL_TEXT_UNIT" => Ok(DedupPolicy::CanonicalTextUnit),
        _ => Err(RagError::RetrievalProfileInvalid),
    }
}

fn parse_topic_policy(config: &Value) -> Result<TopicPolicy, RagError> {
    let Some(policy) = config.get("topicPolicy").and_then(Value::as_object) else {
        return Ok(TopicPolicy::Disabled);
    };
    if !policy
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(TopicPolicy::Disabled);
    }
    match policy.get("type").and_then(Value::as_str) {
        Some("EMBEDDING_MMR_V1") => {}
        _ => return Err(RagError::RetrievalProfileInvalid),
    }
    if policy
        .get("applyStage")
        .and_then(Value::as_str)
        .is_some_and(|stage| stage != "BEFORE_RERANK")
    {
        return Err(RagError::RetrievalProfileInvalid);
    }
    if policy
        .get("fallback")
        .and_then(Value::as_str)
        .is_some_and(|fallback| fallback != "KEEP_ORDER")
    {
        return Err(RagError::RetrievalProfileInvalid);
    }
    let candidate_window = policy
        .get("candidateWindow")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .filter(|value| *value > 0)
        .unwrap_or(50);
    let lambda = policy
        .get("lambda")
        .and_then(Value::as_f64)
        .filter(|value| (0.0..=1.0).contains(value))
        .unwrap_or(0.72);
    let similarity_threshold = policy
        .get("similarityThreshold")
        .and_then(Value::as_f64)
        .filter(|value| (0.0..=1.0).contains(value))
        .unwrap_or(0.82);
    let max_per_topic = policy
        .get("maxPerTopic")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(0);
    Ok(TopicPolicy::EmbeddingMmr(EmbeddingMmrPolicy {
        candidate_window,
        lambda,
        similarity_threshold,
        max_per_topic,
        fallback: TopicFallback::KeepOrder,
    }))
}

fn string_config(config: &Value, field: &str) -> Option<String> {
    config
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
