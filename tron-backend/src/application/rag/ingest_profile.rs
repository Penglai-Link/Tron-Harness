use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::domain::rag::{RagCorpusRecord, RagError, RagProfileRecord, RagProfileType, RagStatus};

use super::{
    CompiledRagPipeline, RagPipelineEdgeInput, RagPipelineInput, RagPipelineNodeInput,
    RagStrategyRegistry, StrategyRegistrationStatus, execution_stage_for_stage_type,
    stage_type_for_execution_stage, validate_pipeline,
};

const DEFAULT_PROJECT_DOC_RESOLVER_MAX_BYTES: usize = 1_048_576;
const DEFAULT_MARKDOWN_HEADING_DEPTH: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledRagIngestProfile {
    pub profile_id: String,
    pub corpus_id: String,
    pub stage_order: Vec<String>,
    pub pipeline: Option<CompiledRagPipeline>,
    pub stage_metadata: BTreeMap<String, CompiledRagIngestStage>,
    pub payload_strategy_id: String,
    pub split_strategy_id: String,
    pub supported_source_types: Vec<String>,
    pub source_resolver_strategy_id: Option<String>,
    pub document_parser_strategy_id: Option<String>,
    pub source_resolver_max_bytes: usize,
    pub record_resolver_info: bool,
    pub record_parser_info: bool,
    pub markdown_heading_depth: usize,
    pub include_heading_in_index_text: bool,
    pub document_format: String,
    pub document_formats: Vec<String>,
    pub degrade_policy: String,
    pub embedding_model_code: Option<String>,
    pub embedding_version: String,
    pub graph_write_policy: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledRagIngestStage {
    pub node_id: String,
    pub stage_type: String,
    pub strategy_id: String,
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

    let (pipeline_input, stages) = if let Some(pipeline) = profile.config.get("pipeline") {
        let pipeline_input = serde_json::from_value::<RagPipelineInput>(pipeline.clone())
            .map_err(|_| RagError::IngestProfileInvalid)?;
        let stages = stages_from_pipeline(&pipeline_input);
        (pipeline_input, stages)
    } else {
        let stages = profile
            .config
            .get("stages")
            .and_then(Value::as_object)
            .cloned()
            .ok_or(RagError::IngestProfileInvalid)?;
        (legacy_pipeline_input(&profile.config, &stages)?, stages)
    };
    let compiled_pipeline = validate_pipeline(&pipeline_input, RagProfileType::Ingest)
        .map_err(|_| RagError::IngestProfileInvalid)?;
    if compiled_pipeline.nodes.is_empty()
        || !compiled_pipeline
            .nodes
            .iter()
            .any(|node| node.stage_type == "PERSIST_SOURCE")
    {
        return Err(RagError::IngestProfileInvalid);
    }

    let mut stage_metadata = BTreeMap::new();
    for node in &compiled_pipeline.nodes {
        let stage_key = node.execution_stage_id.as_str();
        let stage = stages
            .get(stage_key)
            .and_then(Value::as_object)
            .ok_or(RagError::IngestProfileInvalid)?;
        let strategy_id = stage
            .get("strategyId")
            .and_then(Value::as_str)
            .ok_or(RagError::IngestProfileInvalid)?;
        if registry.resolve_stage(strategy_id).status != StrategyRegistrationStatus::Enabled {
            return Err(RagError::IngestProfileInvalid);
        }
        stage_metadata.insert(
            node.node_id.clone(),
            CompiledRagIngestStage {
                node_id: stage
                    .get("nodeId")
                    .and_then(Value::as_str)
                    .unwrap_or(node.node_id.as_str())
                    .to_string(),
                stage_type: stage
                    .get("stageType")
                    .and_then(Value::as_str)
                    .unwrap_or(node.stage_type.as_str())
                    .to_string(),
                strategy_id: strategy_id.to_string(),
            },
        );
        stage_metadata.insert(
            node.execution_stage_id.clone(),
            CompiledRagIngestStage {
                node_id: node.node_id.clone(),
                stage_type: node.stage_type.clone(),
                strategy_id: node.strategy_id.clone(),
            },
        );
    }
    let payload_strategy_id = payload_strategy_id(&profile.config, &stages)?;
    if registry
        .resolve_payload_strategy(&payload_strategy_id)
        .status
        != StrategyRegistrationStatus::Enabled
    {
        return Err(RagError::IngestProfileInvalid);
    }
    let split_strategy_id =
        optional_stage_strategy_id(&stages, "splitText")?.ok_or(RagError::IngestProfileInvalid)?;
    validate_summary_tree_config(&stages)?;
    let supported_source_types = supported_source_types(&profile.config)?;
    let source_resolver_strategy_id = optional_stage_strategy_id(&stages, "resolveSource")?;
    let document_parser_strategy_id = optional_stage_strategy_id(&stages, "parseDocument")?;

    let ordered = compiled_pipeline
        .execution_levels
        .iter()
        .flat_map(|level| level.iter().cloned())
        .collect::<Vec<_>>();
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
    let source_resolver_max_bytes = source_resolver_max_bytes(&profile.config, &stages)?;
    let (record_resolver_info, record_parser_info) = metadata_policy_flags(&profile.config);
    let markdown_heading_depth =
        if uses_markdown_heading_split(&payload_strategy_id, &split_strategy_id) {
            markdown_heading_depth(&profile.config, &stages)?
        } else {
            DEFAULT_MARKDOWN_HEADING_DEPTH
        };
    let include_heading_in_index_text =
        include_heading_in_index_text(&profile.config, &stages, &split_strategy_id);
    let document_formats = document_formats(
        &profile.config,
        &stages,
        document_parser_strategy_id.as_deref(),
    );
    let document_format = document_formats
        .first()
        .cloned()
        .unwrap_or_else(|| "MARKDOWN".into());

    Ok(CompiledRagIngestProfile {
        profile_id: profile.profile_id.clone(),
        corpus_id: corpus.corpus_id.clone(),
        stage_order: ordered,
        pipeline: Some(compiled_pipeline),
        stage_metadata,
        payload_strategy_id,
        split_strategy_id,
        supported_source_types,
        source_resolver_strategy_id,
        document_parser_strategy_id,
        source_resolver_max_bytes,
        record_resolver_info,
        record_parser_info,
        markdown_heading_depth,
        include_heading_in_index_text,
        document_format,
        document_formats,
        degrade_policy: profile.degrade_policy.clone(),
        embedding_model_code,
        embedding_version,
        graph_write_policy: profile
            .config
            .get("graphWritePolicy")
            .cloned()
            .unwrap_or(Value::Null),
    })
}

impl CompiledRagIngestProfile {
    pub fn supports_document_format(&self, format: &str) -> bool {
        self.document_formats
            .iter()
            .any(|candidate| candidate == format)
            || self.document_format == format
    }
}

fn legacy_pipeline_input(
    config: &Value,
    stages: &serde_json::Map<String, Value>,
) -> Result<RagPipelineInput, RagError> {
    let dag = config
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
    let edges = dag
        .get("edges")
        .and_then(Value::as_array)
        .ok_or(RagError::IngestProfileInvalid)?
        .iter()
        .map(parse_edge)
        .collect::<Result<Vec<_>, _>>()?;
    let node_set = nodes.iter().cloned().collect::<BTreeSet<_>>();
    for (from, to) in &edges {
        if !node_set.contains(from) || !node_set.contains(to) {
            return Err(RagError::IngestProfileInvalid);
        }
    }
    let nodes = nodes
        .iter()
        .map(|node| {
            let stage = stages
                .get(node)
                .and_then(Value::as_object)
                .ok_or(RagError::IngestProfileInvalid)?;
            let stage_type = stage
                .get("stageType")
                .and_then(Value::as_str)
                .or_else(|| stage_type_for_execution_stage(node))
                .ok_or(RagError::IngestProfileInvalid)?;
            let strategy_id = stage
                .get("strategyId")
                .and_then(Value::as_str)
                .ok_or(RagError::IngestProfileInvalid)?;
            Ok(RagPipelineNodeInput {
                node_id: node.clone(),
                stage_type: stage_type.to_string(),
                strategy_id: strategy_id.to_string(),
                enabled: true,
                join_policy: stage
                    .get("joinPolicy")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                failure_policy: stage
                    .get("failurePolicy")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                config: stage.get("config").cloned(),
            })
        })
        .collect::<Result<Vec<_>, RagError>>()?;
    Ok(RagPipelineInput {
        mode: "DAG".into(),
        nodes,
        edges: edges
            .into_iter()
            .map(|(from, to)| RagPipelineEdgeInput { from, to })
            .collect(),
    })
}

fn stages_from_pipeline(pipeline: &RagPipelineInput) -> serde_json::Map<String, Value> {
    pipeline
        .nodes
        .iter()
        .filter_map(|node| {
            execution_stage_for_stage_type(&node.stage_type).map(|execution_stage_id| {
                let mut stage = serde_json::Map::new();
                stage.insert("nodeId".into(), Value::String(node.node_id.clone()));
                stage.insert("stageType".into(), Value::String(node.stage_type.clone()));
                stage.insert("strategyId".into(), Value::String(node.strategy_id.clone()));
                if let Some(config) = node.config.clone() {
                    stage.insert("config".into(), config);
                }
                (execution_stage_id.to_string(), Value::Object(stage))
            })
        })
        .collect()
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

fn uses_markdown_heading_split(payload_strategy_id: &str, split_strategy_id: &str) -> bool {
    match split_strategy_id {
        "RAG_SPLIT_MARKDOWN_HEADING_V1" => true,
        "RAG_SPLIT_PLAIN_TEXT_V1" => false,
        "RAG_SPLIT_TEXT" => payload_strategy_id == "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1",
        _ => false,
    }
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

fn include_heading_in_index_text(
    config: &Value,
    stages: &serde_json::Map<String, Value>,
    split_strategy_id: &str,
) -> bool {
    stage_config(stages, "splitText")
        .and_then(|config| config.get("splitter"))
        .and_then(Value::as_object)
        .and_then(|splitter| splitter.get("includeHeadingInIndexText"))
        .and_then(Value::as_bool)
        .or_else(|| {
            config
                .get("splitter")
                .and_then(Value::as_object)
                .and_then(|splitter| splitter.get("includeHeadingInIndexText"))
                .and_then(Value::as_bool)
        })
        .unwrap_or(split_strategy_id == "RAG_SPLIT_MARKDOWN_HEADING_V1")
}

fn validate_summary_tree_config(stages: &serde_json::Map<String, Value>) -> Result<(), RagError> {
    if optional_stage_strategy_id(stages, "summarizeTree")?.as_deref()
        != Some("RAG_TREE_LLM_SUMMARY_V1")
    {
        return Ok(());
    }
    let config = stage_config(stages, "summarizeTree")
        .and_then(Value::as_object)
        .ok_or(RagError::IngestProfileInvalid)?;
    if config.keys().any(|key| key != "modelCode") {
        return Err(RagError::IngestProfileInvalid);
    }
    config
        .get("modelCode")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|_| ())
        .ok_or(RagError::IngestProfileInvalid)
}

fn document_formats(
    config: &Value,
    stages: &serde_json::Map<String, Value>,
    parser_strategy_id: Option<&str>,
) -> Vec<String> {
    if let Some(formats) = stage_config(stages, "parseDocument")
        .and_then(|config| config.get("supportedFormats"))
        .and_then(Value::as_array)
        .or_else(|| {
            config
                .get("parserPolicy")
                .and_then(Value::as_object)
                .and_then(|policy| policy.get("supportedFormats"))
                .and_then(Value::as_array)
        })
    {
        let values = formats
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if !values.is_empty() {
            return values;
        }
    }
    if parser_strategy_id == Some("RAG_PARSE_MINERU_DOCUMENT") {
        return vec!["PDF".into(), "DOCX".into()];
    }
    vec![
        stage_config(stages, "parseDocument")
            .and_then(|config| config.get("format"))
            .and_then(Value::as_str)
            .unwrap_or("MARKDOWN")
            .to_string(),
    ]
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
