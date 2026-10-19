//! Graph use cases shared by ownership-authorized HTTP and runtime-authorized Agent tools.
mod browse;
mod ordinary;
mod seeds;
mod service;
mod sources;
mod structure;
mod view;
mod window;
pub use view::*;

use super::*;
use crate::application::rag::{AuthorizedRagRange, RagRetrievalStage, RagSearchRow};
use crate::domain::rag::graph::{GraphDirection, RagGraphSupport, stable_id};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

pub use service::GraphQueryError;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct GraphReadOptions {
    pub seed_count: usize,
    pub incoming_depth: u8,
    pub outgoing_depth: u8,
    pub node_limit: usize,
    pub edge_limit: usize,
    pub relationship_types: BTreeSet<String>,
    pub include_structure: bool,
}
impl Default for GraphReadOptions {
    fn default() -> Self {
        Self {
            seed_count: 10,
            incoming_depth: 3,
            outgoing_depth: 3,
            node_limit: 100,
            edge_limit: 200,
            relationship_types: BTreeSet::new(),
            include_structure: false,
        }
    }
}
impl GraphReadOptions {
    pub fn validate(&self) -> Result<(), GraphQueryError> {
        if !(1..=100).contains(&self.seed_count)
            || self.incoming_depth > 6
            || self.outgoing_depth > 6
            || !(1..=500).contains(&self.node_limit)
            || !(1..=1000).contains(&self.edge_limit)
            || self.relationship_types.len() > 32
            || self
                .relationship_types
                .iter()
                .any(|v| v.trim().is_empty() || v.len() > 128 || v.chars().any(char::is_control))
        {
            return Err(GraphQueryError::Invalid);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GraphSearchInput {
    pub query: String,
    pub corpus_ids: Vec<String>,
    pub filters: Value,
    pub options: GraphReadOptions,
}
#[derive(Clone, Debug)]
pub struct GraphBrowseInput {
    pub corpus_ids: Vec<String>,
    pub filters: Value,
    pub options: GraphReadOptions,
}
#[derive(Clone, Debug)]
pub struct GraphExpandInput {
    pub node_ids: Vec<String>,
    pub corpus_ids: Vec<String>,
    pub filters: Value,
    pub options: GraphReadOptions,
    pub cursor: Option<String>,
}

pub struct GraphQueryOutput {
    pub snapshot_id: String,
    pub contexts: Vec<GraphRecallContext>,
    pub references: Vec<GraphSourceReference>,
    pub stages: Vec<RagRetrievalStage>,
    pub next_cursor: Option<String>,
    pub expires_at: Option<i64>,
    pub unfinished: Vec<GraphUnfinishedPosition>,
    pub truncated_by: BTreeSet<String>,
    pub duration_ms: u64,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphSourceReference {
    pub id: String,
    pub corpus_id: String,
    pub source_code: String,
    pub source_version: String,
    pub generation_id: String,
    pub chunk_code: String,
    pub file_name: Option<String>,
    pub document_id: Option<String>,
    pub resource_code: Option<String>,
    pub workspace_code: String,
    pub quote: String,
    pub location: Value,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphUnfinishedPosition {
    pub node_id: String,
    pub direction: Option<GraphDirection>,
    pub depth: Option<u8>,
    pub phase: GraphContinuationPhase,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub enum GraphContinuationPhase {
    Traversal,
    Provenance,
}

#[derive(Clone)]
pub(super) struct AuthorizedGraphSnapshot {
    ranges: Vec<AuthorizedRagRange>,
    manifests: Vec<GraphGenerationManifest>,
    stamp: String,
}
impl AuthorizedGraphSnapshot {
    fn generation_ids(&self) -> BTreeSet<String> {
        self.manifests.iter().map(|g| g.id.clone()).collect()
    }
    fn corpus_label(&self) -> String {
        let corpora = self
            .manifests
            .iter()
            .map(|g| g.partition.corpus_id.as_str())
            .collect::<BTreeSet<_>>();
        if corpora.len() == 1 {
            corpora.into_iter().next().unwrap().into()
        } else {
            String::new()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct GraphWindowState {
    partition: GraphPartition,
    generations: BTreeSet<String>,
    seeds: Vec<GraphRecallSeed>,
    expansion: GraphExpansionState,
    pending_provenance: Option<PendingProvenance>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PendingProvenance {
    node_ids: BTreeSet<String>,
    after: Option<String>,
    hops: Vec<GraphRecallHop>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedGraphRequest {
    query: Option<String>,
    corpus_ids: Vec<String>,
    filters: Value,
    options: GraphReadOptions,
    stamp: String,
}
#[derive(Serialize, Deserialize)]
struct SavedTraversal {
    windows: Vec<GraphWindowState>,
}

#[derive(Default)]
pub(super) struct NodeRecall {
    seeds: Vec<GraphRecallSeed>,
    stages: Vec<RagRetrievalStage>,
    warnings: Vec<String>,
}
#[derive(Default)]
pub(super) struct GraphWindowRead {
    contexts: Vec<GraphRecallContext>,
    pending: Vec<GraphWindowState>,
    supports: Vec<(GraphPartition, RagGraphSupport)>,
    truncated_by: BTreeSet<String>,
}

pub fn graph_reference_id(partition: &GraphPartition, support: &RagGraphSupport) -> String {
    stable_id(
        "GRAPH_REFERENCE",
        &[
            &partition.key(),
            &support.generation_id,
            &support.chunk_code,
            &support.quote,
        ],
    )
}

pub(crate) use sources::source_query;
