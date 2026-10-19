//! RAG graph facts. Storage and authorization are supplied by application ports.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphPartition {
    pub corpus_id: String,
    pub scope_type: String,
    pub scope_code: String,
}
impl GraphPartition {
    pub fn key(&self) -> String {
        stable_id(
            "PARTITION",
            &[&self.corpus_id, &self.scope_type, &self.scope_code],
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RagGraphNodeKind {
    Entity,
    EntityContribution,
    SourceVersion,
    Section,
    Chunk,
    SourceBlock,
    Table,
    Image,
    Asset,
    SourceCaption,
    OcrText,
    GeneratedDescription,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagGraphNode {
    pub id: String,
    pub kind: RagGraphNodeKind,
    /// Empty only for shared entity identities. Descriptions live in contributions.
    pub generation_id: String,
    pub name: String,
    pub entity_type: Option<String>,
    pub properties: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagGraphSupport {
    pub generation_id: String,
    pub source_code: String,
    pub source_version: String,
    pub task_id: String,
    pub chunk_code: String,
    pub quote: String,
    pub location: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagGraphEdge {
    /// Support identity, not merely the logical relationship identity.
    pub id: String,
    pub logical_relation_id: String,
    pub from: String,
    pub to: String,
    pub relationship: String,
    pub structural: bool,
    pub support: RagGraphSupport,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagGraphGeneration {
    pub partition: GraphPartition,
    pub id: String,
    pub source_code: String,
    pub source_version: String,
    pub task_id: String,
    pub nodes: Vec<RagGraphNode>,
    pub edges: Vec<RagGraphEdge>,
}
impl RagGraphGeneration {
    /// Validate before any graph write; quotes are checked against immutable chunk input.
    pub fn validate(&self, chunks: &BTreeMap<String, String>) -> Result<(), String> {
        if [
            &self.id,
            &self.source_code,
            &self.source_version,
            &self.task_id,
            &self.partition.corpus_id,
            &self.partition.scope_type,
            &self.partition.scope_code,
        ]
        .iter()
        .any(|s| s.trim().is_empty())
        {
            return Err("graph generation identity is empty".into());
        }
        let mut ids = BTreeSet::new();
        for node in &self.nodes {
            if node.id.is_empty() || !ids.insert(node.id.as_str()) || !node.properties.is_object() {
                return Err("graph node identity or properties are invalid".into());
            }
            if node.kind == RagGraphNodeKind::Entity {
                if !node.generation_id.is_empty()
                    || node.entity_type.as_deref().is_none_or(str::is_empty)
                    || node.properties.as_object().is_some_and(|p| !p.is_empty())
                {
                    return Err("shared entities may contain identity only".into());
                }
            } else if node.generation_id != self.id {
                return Err("graph node belongs to another generation".into());
            }
        }
        let mut edge_ids = BTreeSet::new();
        for edge in &self.edges {
            let support = &edge.support;
            if edge.id.is_empty()
                || !edge_ids.insert(&edge.id)
                || edge.logical_relation_id.is_empty()
                || !ids.contains(edge.from.as_str())
                || !ids.contains(edge.to.as_str())
                || edge.relationship.trim().is_empty()
                || support.generation_id != self.id
                || support.source_code != self.source_code
                || support.source_version != self.source_version
                || support.task_id != self.task_id
                || !support.location.is_object()
            {
                return Err("graph relationship identity or support is invalid".into());
            }
            let Some(text) = chunks.get(&support.chunk_code) else {
                return Err("graph support chunk is absent".into());
            };
            if !edge.structural && (support.quote.is_empty() || !text.contains(&support.quote)) {
                return Err("graph relationship has no exact source quote".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GraphDirection {
    Incoming,
    Outgoing,
}

/// Length-prefixed components avoid separator collisions in user-provided names.
pub fn stable_id(prefix: &str, parts: &[&str]) -> String {
    let mut hash = Sha256::new();
    for part in parts {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    format!("{prefix}_{:x}", hash.finalize())
}
