use serde_json::Value;

use crate::domain::rag::{RagCorpusRecord, RagError, RagProfileRecord, RagProfileType, RagStatus};

use super::{RagStrategyRegistry, StrategyRegistrationStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledRetrievalProfile {
    pub profile_id: String,
    pub corpus_id: String,
    pub required_source_version: String,
    pub routes: Vec<RetrievalRoute>,
    pub supported_context_strategies: Vec<String>,
    pub embedding_model_code: Option<String>,
    pub rerank_model_code: Option<String>,
    pub rerank_enabled: bool,
    pub degrade_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievalRoute {
    pub route_id: String,
    pub retriever: String,
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
        required_source_version,
        routes,
        supported_context_strategies,
        embedding_model_code: string_config(&profile.config, "embeddingModelCode"),
        rerank_model_code: string_config(&profile.config, "rerankModelCode"),
        rerank_enabled,
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
                    retriever: retriever.to_string(),
                })
            })
            .collect();
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
                retriever: retriever.to_string(),
            })
        })
        .collect()
}

fn default_route_id(corpus_id: &str, retriever: &str, index: usize) -> String {
    let prefix = match corpus_id {
        "memory-card" => "memory",
        "project-doc" => "project-doc",
        _ => corpus_id,
    };
    match retriever {
        "FULLTEXT" => format!("{prefix}-fulltext"),
        "VECTOR" => format!("{prefix}-vector"),
        _ => format!("{prefix}-route-{index}"),
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

fn string_config(config: &Value, field: &str) -> Option<String> {
    config
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
