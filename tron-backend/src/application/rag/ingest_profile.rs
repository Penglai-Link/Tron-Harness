use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::domain::rag::{RagCorpusRecord, RagError, RagProfileRecord, RagProfileType, RagStatus};

use super::{RagStrategyRegistry, StrategyRegistrationStatus};

const DEFAULT_PROJECT_DOC_RESOLVER_MAX_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledRagIngestProfile {
    pub profile_id: String,
    pub corpus_id: String,
    pub stage_order: Vec<String>,
    pub payload_strategy_id: String,
    pub source_resolver_strategy_id: Option<String>,
    pub document_parser_strategy_id: Option<String>,
    pub source_resolver_max_bytes: usize,
    pub record_resolver_info: bool,
    pub record_parser_info: bool,
    pub degrade_policy: String,
    pub embedding_model_code: Option<String>,
    pub embedding_version: String,
}

pub fn compile_ingest_profile<S>(
    profile: &RagProfileRecord,
    corpus: &RagCorpusRecord,
    registry: &S,
) -> Result<CompiledRagIngestProfile, RagError>
where
    S: RagStrategyRegistry,
{
    if profile.profile_type != RagProfileType::Ingest
        || profile.status != RagStatus::Enabled
        || profile.primary_corpus_id.as_deref() != Some(corpus.corpus_id.as_str())
        || !profile.corpus_ids.contains(&corpus.corpus_id)
    {
        return Err(RagError::IngestProfileInvalid);
    }

    let dag = profile
        .config
        .get("dag")
        .and_then(Value::as_object)
        .ok_or(RagError::IngestProfileInvalid)?;
    let nodes = dag
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or(RagError::IngestProfileInvalid)?
        .iter()
        .map(|value| value.as_str().map(ToOwned::to_owned))
        .collect::<Option<Vec<_>>>()
        .ok_or(RagError::IngestProfileInvalid)?;
    let stages = profile
        .config
        .get("stages")
        .and_then(Value::as_object)
        .ok_or(RagError::IngestProfileInvalid)?;
    if nodes.is_empty() || !nodes.iter().any(|node| node == "persistSource") {
        return Err(RagError::IngestProfileInvalid);
    }

    let node_set = nodes.iter().cloned().collect::<BTreeSet<_>>();
    for node in &nodes {
        let stage = stages
            .get(node)
            .and_then(Value::as_object)
            .ok_or(RagError::IngestProfileInvalid)?;
        let strategy_id = stage
            .get("strategyId")
            .and_then(Value::as_str)
            .ok_or(RagError::IngestProfileInvalid)?;
        if registry.resolve_stage(strategy_id).status != StrategyRegistrationStatus::Enabled {
            return Err(RagError::IngestProfileInvalid);
        }
    }
    let payload_strategy_id = payload_strategy_id(&profile.config)?;
    if registry
        .resolve_payload_strategy(&payload_strategy_id)
        .status
        != StrategyRegistrationStatus::Enabled
    {
        return Err(RagError::IngestProfileInvalid);
    }
    let source_resolver_strategy_id = optional_stage_strategy_id(stages, "resolveSource")?;
    let document_parser_strategy_id = optional_stage_strategy_id(stages, "parseDocument")?;
    if payload_strategy_id == "PROJECT_DOC_MARKDOWN_REF_PAYLOAD_V1" {
        validate_project_doc_ref_stages(
            &node_set,
            source_resolver_strategy_id.as_deref(),
            document_parser_strategy_id.as_deref(),
        )?;
    }

    let edges = dag
        .get("edges")
        .and_then(Value::as_array)
        .ok_or(RagError::IngestProfileInvalid)?
        .iter()
        .map(parse_edge)
        .collect::<Result<Vec<_>, _>>()?;
    for (from, to) in &edges {
        if !node_set.contains(from) || !node_set.contains(to) {
            return Err(RagError::IngestProfileInvalid);
        }
        let from_output = stage_artifact(stages, from, "outputArtifact")?;
        let to_input = stage_artifact(stages, to, "inputArtifact")?;
        if from_output != to_input {
            return Err(RagError::IngestProfileInvalid);
        }
    }

    let ordered = topological_order(&nodes, &edges)?;
    let embedding_model_code = profile
        .config
        .get("embeddingModelCode")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let embedding_version = profile
        .config
        .get("embeddingVersion")
        .and_then(Value::as_str)
        .unwrap_or("v1")
        .to_string();
    let source_resolver_max_bytes = source_resolver_max_bytes(&profile.config)?;
    let (record_resolver_info, record_parser_info) = metadata_policy_flags(&profile.config);

    Ok(CompiledRagIngestProfile {
        profile_id: profile.profile_id.clone(),
        corpus_id: corpus.corpus_id.clone(),
        stage_order: ordered,
        payload_strategy_id,
        source_resolver_strategy_id,
        document_parser_strategy_id,
        source_resolver_max_bytes,
        record_resolver_info,
        record_parser_info,
        degrade_policy: profile.degrade_policy.clone(),
        embedding_model_code,
        embedding_version,
    })
}

fn payload_strategy_id(config: &Value) -> Result<String, RagError> {
    if let Some(strategy_id) = config
        .get("payloadStrategyId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(strategy_id.to_string());
    }
    match config
        .get("splitter")
        .and_then(Value::as_object)
        .and_then(|splitter| splitter.get("type"))
        .and_then(Value::as_str)
    {
        Some("plainText") => Ok("MEMORY_PLAIN_TEXT_PAYLOAD_V1".into()),
        Some("markdownHeading") => Ok("PROJECT_DOC_MARKDOWN_REF_PAYLOAD_V1".into()),
        _ => Err(RagError::IngestProfileInvalid),
    }
}

fn validate_project_doc_ref_stages(
    node_set: &BTreeSet<String>,
    source_resolver_strategy_id: Option<&str>,
    document_parser_strategy_id: Option<&str>,
) -> Result<(), RagError> {
    if !node_set.contains("resolveSource") || !node_set.contains("parseDocument") {
        return Err(RagError::IngestProfileInvalid);
    }
    if source_resolver_strategy_id != Some("PROJECT_DOC_MARKDOWN_RESOLVE_SOURCE_V1")
        || document_parser_strategy_id != Some("PROJECT_DOC_MARKDOWN_PARSE_DOCUMENT_V1")
    {
        return Err(RagError::IngestProfileInvalid);
    }
    Ok(())
}

fn optional_stage_strategy_id(
    stages: &serde_json::Map<String, Value>,
    stage_id: &str,
) -> Result<Option<String>, RagError> {
    let Some(stage) = stages.get(stage_id) else {
        return Ok(None);
    };
    stage
        .as_object()
        .and_then(|stage| stage.get("strategyId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| Some(value.to_owned()))
        .ok_or(RagError::IngestProfileInvalid)
}

fn source_resolver_max_bytes(config: &Value) -> Result<usize, RagError> {
    let Some(max_bytes) = config
        .get("resolverPolicy")
        .and_then(Value::as_object)
        .and_then(|policy| policy.get("maxBytes"))
        .and_then(Value::as_u64)
    else {
        return Ok(DEFAULT_PROJECT_DOC_RESOLVER_MAX_BYTES);
    };
    if max_bytes == 0 {
        return Err(RagError::IngestProfileInvalid);
    }
    usize::try_from(max_bytes).map_err(|_| RagError::IngestProfileInvalid)
}

fn metadata_policy_flags(config: &Value) -> (bool, bool) {
    let Some(policy) = config.get("metadataPolicy").and_then(Value::as_object) else {
        return (false, false);
    };
    (
        policy
            .get("recordResolverInfo")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        policy
            .get("recordParserInfo")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    )
}

fn parse_edge(value: &Value) -> Result<(String, String), RagError> {
    let object = value.as_object().ok_or(RagError::IngestProfileInvalid)?;
    let from = object
        .get("from")
        .and_then(Value::as_str)
        .ok_or(RagError::IngestProfileInvalid)?;
    let to = object
        .get("to")
        .and_then(Value::as_str)
        .ok_or(RagError::IngestProfileInvalid)?;
    Ok((from.to_string(), to.to_string()))
}

fn stage_artifact(
    stages: &serde_json::Map<String, Value>,
    stage_id: &str,
    field: &str,
) -> Result<String, RagError> {
    stages
        .get(stage_id)
        .and_then(Value::as_object)
        .and_then(|stage| stage.get(field))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or(RagError::IngestProfileInvalid)
}

fn topological_order(
    nodes: &[String],
    edges: &[(String, String)],
) -> Result<Vec<String>, RagError> {
    let mut incoming = nodes
        .iter()
        .map(|node| (node.clone(), 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (from, to) in edges {
        *incoming.get_mut(to).ok_or(RagError::IngestProfileInvalid)? += 1;
        outgoing.entry(from.clone()).or_default().push(to.clone());
    }
    let mut ready = incoming
        .iter()
        .filter_map(|(node, count)| (*count == 0).then(|| node.clone()))
        .collect::<Vec<_>>();
    let mut ordered = Vec::new();
    while let Some(node) = ready.pop() {
        ordered.push(node.clone());
        if let Some(next_nodes) = outgoing.get(&node) {
            for next in next_nodes {
                let count = incoming
                    .get_mut(next)
                    .ok_or(RagError::IngestProfileInvalid)?;
                *count -= 1;
                if *count == 0 {
                    ready.push(next.clone());
                }
            }
        }
    }
    if ordered.len() != nodes.len() {
        return Err(RagError::IngestProfileInvalid);
    }
    Ok(ordered)
}
