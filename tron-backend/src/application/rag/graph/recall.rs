//! Authorized node recall and real Grafeo expansion, shared by retrieval callers.
use super::*;
use crate::application::rag::{RagRetrievalStage, RagSearchRow};
use crate::domain::rag::graph::{RagGraphEdge, RagGraphNode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type QueryEmbeddingCache = BTreeMap<
    (
        Option<String>,
        Option<usize>,
        Option<String>,
        Option<String>,
    ),
    Result<Vec<f32>, String>,
>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphRecallSeed {
    pub id: String,
    pub score: f64,
    pub channels: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphRecallHop {
    pub seed_id: String,
    pub node_id: String,
    pub direction: crate::domain::rag::graph::GraphDirection,
    pub hops: u8,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphRecallContext {
    pub partition: GraphPartition,
    pub seeds: Vec<GraphRecallSeed>,
    pub nodes: Vec<RagGraphNode>,
    pub contributions: Vec<RagGraphNode>,
    pub edges: Vec<RagGraphEdge>,
    pub provenance: Vec<RagGraphEdge>,
    pub hops: Vec<GraphRecallHop>,
    pub truncated_by: Vec<String>,
    /// Kept server-side until bound to an opaque, identity-specific snapshot.
    #[serde(skip)]
    pub expansion: GraphExpansionState,
    #[serde(skip)]
    pub generations: BTreeSet<String>,
}

#[derive(Default)]
pub(crate) struct GraphRangeRecall {
    pub rows: Vec<RagSearchRow>,
    pub contexts: Vec<GraphRecallContext>,
    pub stages: Vec<RagRetrievalStage>,
    pub warnings: Vec<String>,
}
