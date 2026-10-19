//! Ports for real graph facts. SQLite controls publication and source authorization.
mod generation;
mod source_structure;
pub(crate) use generation::{graph_source_text, validate_graph_extraction};
mod cleanup;
mod indexing;
pub use cleanup::{GraphCleanupClaim, GraphCleanupReport, GraphHousekeepingResult};
pub(crate) use indexing::GraphComputationCheckpoint;
pub use indexing::{GraphIndexContext, GraphInfrastructure};
mod publication;
mod vector_checkpoint;
pub use vector_checkpoint::{
    GraphNodeVectorBatch, GraphNodeVectorCheckpoint, GraphNodeVectorManifest,
    graph_node_vector_batch_key,
};
mod snapshot;
pub use publication::{
    GraphGenerationManifest, GraphNodeMatch, GraphNodeSearchHit, GraphNodeSearchPage,
    GraphNodeSearchQuery, PreparedGraphProjection, RagGraphIndexRepository,
};
pub use snapshot::{
    GRAPH_SNAPSHOT_TTL_MS, GRAPH_SNAPSHOTS_PER_USER, GraphQuerySnapshot, GraphSnapshotDraft,
    GraphSnapshotError,
};
mod output;
mod query;
mod recall;
pub(crate) use output::retain_returned_evidence;
pub(crate) use query::evidence_graph_view;
pub use query::{
    GraphBrowseInput, GraphContinuationPhase, GraphEdgeView, GraphEvidenceView, GraphExpandInput,
    GraphNodeKindView, GraphNodeView, GraphPresentation, GraphQueryError, GraphQueryOutput,
    GraphReadOptions, GraphSearchInput, GraphSeedView, GraphSourceReference, GraphStageStatus,
    GraphStageView, GraphTextView, GraphUnfinishedPosition, graph_reference_id,
};
mod traversal;
use crate::domain::rag::graph::{
    GraphDirection, GraphPartition, RagGraphEdge, RagGraphGeneration, RagGraphNode,
};
pub use generation::{GraphNodeTextProjection, build_graph_generation};
pub(crate) use query::source_query as graph_source_query;
pub use recall::{GraphRecallContext, GraphRecallHop, GraphRecallSeed};
use std::collections::BTreeSet;
pub use traversal::{GraphExpansionState, GraphExpansionWindow, GraphTraversalLimits};

pub const INTERNAL_RAG_GRAPH_NAMESPACE: &str = "rag-internal";

#[derive(Clone, Debug)]
pub struct GraphAdjacencyQuery {
    pub partition: GraphPartition,
    pub node_id: String,
    pub direction: GraphDirection,
    /// Server-resolved, activated and authorized source generations only.
    pub generations: BTreeSet<String>,
    pub relationship_types: BTreeSet<String>,
    pub include_structural: bool,
    pub after_edge_id: Option<String>,
    pub limit: usize,
}
#[derive(Clone, Debug, Default)]
pub struct GraphAdjacencyPage {
    pub edges: Vec<RagGraphEdge>,
    pub has_more: bool,
}

/// A bounded batch of source-owned structural relationships. Generations are
/// resolved by the application from the current authorized publication snapshot.
#[derive(Clone, Debug)]
pub struct GraphStructuralQuery {
    pub partition: GraphPartition,
    pub node_ids: BTreeSet<String>,
    pub direction: GraphDirection,
    pub generations: BTreeSet<String>,
    pub relationships: BTreeSet<String>,
    pub after_edge_id: Option<String>,
    pub limit: usize,
}
impl GraphStructuralQuery {
    pub fn validate(&self) -> Result<(), String> {
        if self.node_ids.len() > 500
            || !(1..=1000).contains(&self.limit)
            || self.relationships.is_empty()
            || self.relationships.len() > 16
            || self.relationships.iter().any(|kind| {
                !matches!(
                    kind.as_str(),
                    "CONTAINS"
                        | "COVERS"
                        | "HAS_IMAGE"
                        | "HAS_ASSET"
                        | "HAS_DESCRIPTION"
                        | "HAS_CAPTION"
                        | "HAS_OCR"
                        | "REFERENCES_FIGURE"
                        | "REFERENCES_TABLE"
                )
            })
        {
            return Err("invalid source-structure query budget or relationship".into());
        }
        Ok(())
    }
}

pub trait RagGraphStore: Send + Sync {
    /// Caller validates quotes and fences its task before staging. This write is
    /// durable before return; it does not publish any source to readers.
    fn stage_generation(&self, generation: &RagGraphGeneration) -> Result<(), String>;
    fn nodes(
        &self,
        partition: &GraphPartition,
        ids: &[String],
        generations: &BTreeSet<String>,
    ) -> Result<Vec<RagGraphNode>, String>;
    fn adjacent(&self, query: &GraphAdjacencyQuery) -> Result<GraphAdjacencyPage, String>;
    fn structural_adjacent(
        &self,
        query: &GraphStructuralQuery,
    ) -> Result<GraphAdjacencyPage, String>;
    /// Batch provenance for selected entities. This reads real structural edges,
    /// never infers citations from lexical node matches or loads the whole graph.
    fn entity_evidence(
        &self,
        partition: &GraphPartition,
        ids: &[String],
        generations: &BTreeSet<String>,
        after: Option<&str>,
        limit: usize,
    ) -> Result<GraphAdjacencyPage, String>;
    /// Remove supports and source-owned nodes, then orphan shared identities.
    fn delete_generation(&self, partition: &GraphPartition, id: &str) -> Result<(), String>;
}
