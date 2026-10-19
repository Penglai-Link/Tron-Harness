use crate::application::rag::graph::{
    GraphEdgeView, GraphNodeView, GraphPresentation, GraphQueryOutput, GraphSeedView,
    GraphSourceReference, GraphStageView, GraphUnfinishedPosition,
};
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct GraphSearchView {
    snapshot_id: String,
    seeds: Vec<GraphSeedView>,
    nodes: Vec<GraphNodeView>,
    edges: Vec<GraphEdgeView>,
    references: Vec<GraphSourceReference>,
    stages: Vec<GraphStageView>,
    next_cursor: Option<String>,
    expires_at: Option<i64>,
    unfinished: Vec<GraphUnfinishedPosition>,
    truncated_by: Vec<String>,
    duration_ms: u64,
}
impl From<GraphQueryOutput> for GraphSearchView {
    fn from(output: GraphQueryOutput) -> Self {
        let v = GraphPresentation::from(output);
        Self {
            snapshot_id: v.snapshot_id,
            seeds: v.seeds,
            nodes: v.nodes,
            edges: v.edges,
            references: v.references,
            stages: v.stages,
            next_cursor: v.next_cursor,
            expires_at: v.expires_at,
            unfinished: v.unfinished,
            truncated_by: v.truncated_by,
            duration_ms: v.duration_ms,
        }
    }
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct GraphExpandView {
    snapshot_id: String,
    seeds: Vec<GraphSeedView>,
    nodes: Vec<GraphNodeView>,
    edges: Vec<GraphEdgeView>,
    references: Vec<GraphSourceReference>,
    stages: Vec<GraphStageView>,
    next_cursor: Option<String>,
    expires_at: Option<i64>,
    unfinished: Vec<GraphUnfinishedPosition>,
    truncated_by: Vec<String>,
    duration_ms: u64,
}
impl From<GraphQueryOutput> for GraphExpandView {
    fn from(output: GraphQueryOutput) -> Self {
        let v = GraphPresentation::from(output);
        Self {
            snapshot_id: v.snapshot_id,
            seeds: v.seeds,
            nodes: v.nodes,
            edges: v.edges,
            references: v.references,
            stages: v.stages,
            next_cursor: v.next_cursor,
            expires_at: v.expires_at,
            unfinished: v.unfinished,
            truncated_by: v.truncated_by,
            duration_ms: v.duration_ms,
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct GraphBrowseView {
    snapshot_id: String,
    seeds: Vec<GraphSeedView>,
    nodes: Vec<GraphNodeView>,
    edges: Vec<GraphEdgeView>,
    references: Vec<GraphSourceReference>,
    stages: Vec<GraphStageView>,
    next_cursor: Option<String>,
    expires_at: Option<i64>,
    unfinished: Vec<GraphUnfinishedPosition>,
    truncated_by: Vec<String>,
    duration_ms: u64,
}
impl From<GraphQueryOutput> for GraphBrowseView {
    fn from(output: GraphQueryOutput) -> Self {
        let v = GraphPresentation::from(output);
        Self {
            snapshot_id: v.snapshot_id,
            seeds: v.seeds,
            nodes: v.nodes,
            edges: v.edges,
            references: v.references,
            stages: v.stages,
            next_cursor: v.next_cursor,
            expires_at: v.expires_at,
            unfinished: v.unfinished,
            truncated_by: v.truncated_by,
            duration_ms: v.duration_ms,
        }
    }
}
