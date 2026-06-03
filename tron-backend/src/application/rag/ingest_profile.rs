use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::domain::rag::{RagCorpusRecord, RagError, RagProfileRecord, RagProfileType, RagStatus};

use super::{RagStrategyRegistry, StrategyRegistrationStatus};

const DEFAULT_PROJECT_DOC_RESOLVER_MAX_BYTES: usize = 1_048_576;
const DEFAULT_MARKDOWN_HEADING_DEPTH: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledRagIngestProfile {
    pub profile_id: String,
    pub corpus_id: String,
    pub stage_order: Vec<String>,
    pub payload_strategy_id: String,
    pub supported_source_types: Vec<String>,
    pub source_resolver_strategy_id: Option<String>,
    pub document_parser_strategy_id: Option<String>,
    pub source_resolver_max_bytes: usize,
    pub record_resolver_info: bool,
    pub record_parser_info: bool,
    pub markdown_heading_depth: usize,
    pub document_format: String,
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
    let payload_strategy_id = payload_strategy_id(&profile.config, stages)?;
    if registry
        .resolve_payload_strategy(&payload_strategy_id)
        .status
        != StrategyRegistrationStatus::Enabled
    {
        return Err(RagError::IngestProfileInvalid);
    }
    let supported_source_types = supported_source_types(&profile.config)?;
    let source_resolver_strategy_id = optional_stage_strategy_id(stages, "resolveSource")?;
    let document_parser_strategy_id = optional_stage_strategy_id(stages, "parseDocument")?;

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
    let source_resolver_max_bytes = source_resolver_max_bytes(&profile.config, stages)?;
    let (record_resolver_info, record_parser_info) = metadata_policy_flags(&profile.config);
    let markdown_heading_depth = if matches!(
        payload_strategy_id.as_str(),
        "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1"
    ) {
        markdown_heading_depth(&profile.config, stages)?
    } else {
        DEFAULT_MARKDOWN_HEADING_DEPTH
    };
    let document_format = document_format(&profile.config, stages);

    Ok(CompiledRagIngestProfile {
        profile_id: profile.profile_id.clone(),
        corpus_id: corpus.corpus_id.clone(),
        stage_order: ordered,
        payload_strategy_id,
        supported_source_types,
        source_resolver_strategy_id,
        document_parser_strategy_id,
        source_resolver_max_bytes,
        record_resolver_info,
        record_parser_info,
        markdown_heading_depth,
        document_format,
        degrade_policy: profile.degrade_policy.clone(),
        embedding_model_code,
        embedding_version,
    })
}

fn payload_strategy_id(
    _config: &Value,
    stages: &serde_json::Map<String, Value>,
) -> Result<String, RagError> {
    if let Some(strategy_id) = optional_stage_strategy_id(stages, "preparePayload")? {
        return Ok(strategy_id);
    }
    Err(RagError::IngestProfileInvalid)
}

fn supported_source_types(config: &Value) -> Result<Vec<String>, RagError> {
    let Some(values) = config
        .get("sourceInputPolicy")
        .and_then(Value::as_object)
        .and_then(|policy| policy.get("supportedSourceTypes"))
        .and_then(Value::as_array)
    else {
        return Ok(Vec::new());
    };
    if values.is_empty() {
        return Err(RagError::IngestProfileInvalid);
    }
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .ok_or(RagError::IngestProfileInvalid)
        })
        .collect()
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

fn source_resolver_max_bytes(
    config: &Value,
    stages: &serde_json::Map<String, Value>,
) -> Result<usize, RagError> {
    let max_bytes = stage_config(stages, "resolveSource")
        .and_then(|config| config.get("maxBytes"))
        .and_then(Value::as_u64)
        .or_else(|| {
            config
                .get("resolverPolicy")
                .and_then(Value::as_object)
                .and_then(|policy| policy.get("maxBytes"))
                .and_then(Value::as_u64)
        });
    let Some(max_bytes) = max_bytes else {
        return Ok(DEFAULT_PROJECT_DOC_RESOLVER_MAX_BYTES);
    };
    if max_bytes == 0 {
        return Err(RagError::IngestProfileInvalid);
    }
    usize::try_from(max_bytes).map_err(|_| RagError::IngestProfileInvalid)
}

fn markdown_heading_depth(
    config: &Value,
    stages: &serde_json::Map<String, Value>,
) -> Result<usize, RagError> {
    let depth = stage_config(stages, "splitText")
        .and_then(|config| config.get("splitter"))
        .and_then(Value::as_object)
        .and_then(|splitter| splitter.get("headingDepth"))
        .and_then(Value::as_u64)
        .or_else(|| {
            config
                .get("splitter")
                .and_then(Value::as_object)
                .and_then(|splitter| splitter.get("headingDepth"))
                .and_then(Value::as_u64)
        });
    let Some(depth) = depth else {
        return Ok(DEFAULT_MARKDOWN_HEADING_DEPTH);
    };
    if !(1..=6).contains(&depth) {
        return Err(RagError::IngestProfileInvalid);
    }
    usize::try_from(depth).map_err(|_| RagError::IngestProfileInvalid)
}

fn document_format(config: &Value, stages: &serde_json::Map<String, Value>) -> String {
    stage_config(stages, "parseDocument")
        .and_then(|config| config.get("format"))
        .and_then(Value::as_str)
        .or_else(|| {
            config
                .get("parserPolicy")
                .and_then(Value::as_object)
                .and_then(|policy| policy.get("supportedFormats"))
                .and_then(Value::as_array)
                .and_then(|formats| formats.first())
                .and_then(Value::as_str)
        })
        .unwrap_or("MARKDOWN")
        .to_string()
}

fn stage_config<'a>(
    stages: &'a serde_json::Map<String, Value>,
    stage_id: &str,
) -> Option<&'a Value> {
    stages
        .get(stage_id)
        .and_then(Value::as_object)
        .and_then(|stage| stage.get("config"))
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
