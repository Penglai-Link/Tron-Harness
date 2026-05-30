use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::domain::rag::{RagCorpusRecord, RagError, RagProfileRecord, RagProfileType, RagStatus};

use super::{RagStrategyRegistry, StrategyRegistrationStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledRagIngestProfile {
    pub profile_id: String,
    pub corpus_id: String,
    pub stage_order: Vec<String>,
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
        .map(ToOwned::to_owned);
    let embedding_version = profile
        .config
        .get("embeddingVersion")
        .and_then(Value::as_str)
        .unwrap_or("v1")
        .to_string();

    Ok(CompiledRagIngestProfile {
        profile_id: profile.profile_id.clone(),
        corpus_id: corpus.corpus_id.clone(),
        stage_order: ordered,
        degrade_policy: profile.degrade_policy.clone(),
        embedding_model_code,
        embedding_version,
    })
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
