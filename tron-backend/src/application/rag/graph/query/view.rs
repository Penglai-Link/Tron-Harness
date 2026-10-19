//! Public graph concepts shared by protocol adapters. Storage properties and vectors stay private.
use super::*;
use crate::domain::rag::graph::RagGraphNodeKind;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEvidenceView {
    pub corpus_id: String,
    pub seeds: Vec<GraphSeedView>,
    pub nodes: Vec<GraphNodeView>,
    pub edges: Vec<GraphEdgeView>,
    pub references: Vec<GraphSourceReference>,
    pub truncated_by: Vec<String>,
}

pub(crate) fn evidence_graph_view(
    result: &crate::application::rag::RagEvidenceResult,
) -> Result<Vec<GraphEvidenceView>, GraphQueryError> {
    let rows = result
        .hits
        .iter()
        .flat_map(|hit| {
            std::iter::once(&hit.row).chain(
                hit.bundle
                    .iter()
                    .flat_map(|bundle| bundle.members.iter().map(|member| &member.row)),
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    result
        .graph_context
        .iter()
        .map(|context| {
            let supports = context
                .edges
                .iter()
                .chain(&context.provenance)
                .map(|edge| (context.partition.clone(), edge.support.clone()))
                .collect::<Vec<_>>();
            let references = super::sources::references_from_rows(&rows, &supports)?;
            let public = GraphPresentation::from(GraphQueryOutput {
                snapshot_id: String::new(),
                contexts: vec![context.clone()],
                references,
                stages: vec![],
                next_cursor: None,
                expires_at: None,
                unfinished: vec![],
                truncated_by: context.truncated_by.iter().cloned().collect(),
                duration_ms: 0,
            });
            Ok(GraphEvidenceView {
                corpus_id: context.partition.corpus_id.clone(),
                seeds: public.seeds,
                nodes: public.nodes,
                edges: public.edges,
                references: public.references,
                truncated_by: public.truncated_by,
            })
        })
        .collect()
}

#[derive(Clone, Debug, Serialize)]
pub enum GraphNodeKindView {
    Entity,
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
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphTextView {
    pub text: String,
    pub reference_id: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeView {
    pub id: String,
    pub corpus_id: String,
    pub kind: GraphNodeKindView,
    pub name: String,
    pub entity_type: Option<String>,
    pub descriptions: Vec<GraphTextView>,
    pub aliases: Vec<GraphTextView>,
    pub reference_ids: Vec<String>,
    pub hops: Vec<GraphRecallHop>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdgeView {
    pub id: String,
    pub from: String,
    pub to: String,
    pub relationship: String,
    pub structural: bool,
    pub reference_ids: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphSeedView {
    pub id: String,
    pub score: f64,
    pub channels: Vec<String>,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub enum GraphStageStatus {
    Completed,
    Truncated,
    Failed,
    Invalidated,
    Skipped,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphStageView {
    pub corpus_id: String,
    pub channel: String,
    pub status: GraphStageStatus,
    pub hit_count: usize,
    pub duration_ms: u64,
    pub error_key: Option<String>,
}
/// Endpoint adapters own their root response objects; this is a mapping result, not an HTTP response.
pub struct GraphPresentation {
    pub snapshot_id: String,
    pub seeds: Vec<GraphSeedView>,
    pub nodes: Vec<GraphNodeView>,
    pub edges: Vec<GraphEdgeView>,
    pub references: Vec<GraphSourceReference>,
    pub stages: Vec<GraphStageView>,
    pub next_cursor: Option<String>,
    pub expires_at: Option<i64>,
    pub unfinished: Vec<GraphUnfinishedPosition>,
    pub truncated_by: Vec<String>,
    pub duration_ms: u64,
}
impl From<GraphQueryOutput> for GraphPresentation {
    fn from(output: GraphQueryOutput) -> Self {
        let reference_ids = output
            .references
            .iter()
            .map(|r| r.id.as_str())
            .collect::<BTreeSet<_>>();
        let mut nodes = BTreeMap::<String, GraphNodeView>::new();
        let mut seeds = BTreeMap::<String, GraphSeedView>::new();
        let mut edges = BTreeMap::<String, GraphEdgeView>::new();
        for context in &output.contexts {
            for node in &context.nodes {
                let kind = match node.kind {
                    RagGraphNodeKind::Entity => GraphNodeKindView::Entity,
                    RagGraphNodeKind::EntityContribution => continue,
                    RagGraphNodeKind::SourceVersion => GraphNodeKindView::SourceVersion,
                    RagGraphNodeKind::Section => GraphNodeKindView::Section,
                    RagGraphNodeKind::Chunk => GraphNodeKindView::Chunk,
                    RagGraphNodeKind::SourceBlock => GraphNodeKindView::SourceBlock,
                    RagGraphNodeKind::Table => GraphNodeKindView::Table,
                    RagGraphNodeKind::Image => GraphNodeKindView::Image,
                    RagGraphNodeKind::Asset => GraphNodeKindView::Asset,
                    RagGraphNodeKind::SourceCaption => GraphNodeKindView::SourceCaption,
                    RagGraphNodeKind::OcrText => GraphNodeKindView::OcrText,
                    RagGraphNodeKind::GeneratedDescription => {
                        GraphNodeKindView::GeneratedDescription
                    }
                };
                let view = nodes
                    .entry(node.id.clone())
                    .or_insert_with(|| GraphNodeView {
                        id: node.id.clone(),
                        corpus_id: context.partition.corpus_id.clone(),
                        kind,
                        name: node.name.clone(),
                        entity_type: node.entity_type.clone(),
                        descriptions: vec![],
                        aliases: vec![],
                        reference_ids: vec![],
                        hops: vec![],
                    });
                view.hops.extend(
                    context
                        .hops
                        .iter()
                        .filter(|h| h.node_id == node.id)
                        .cloned(),
                );
                for edge in context
                    .edges
                    .iter()
                    .chain(&context.provenance)
                    .filter(|e| e.from == node.id || e.to == node.id)
                {
                    let reference_id = graph_reference_id(&context.partition, &edge.support);
                    if reference_ids.contains(reference_id.as_str()) {
                        view.reference_ids.push(reference_id);
                    }
                }
                if matches!(
                    node.kind,
                    RagGraphNodeKind::SourceCaption
                        | RagGraphNodeKind::OcrText
                        | RagGraphNodeKind::GeneratedDescription
                ) {
                    if let (Some(text), Some(reference_id)) =
                        (node.properties["text"].as_str(), view.reference_ids.first())
                    {
                        view.descriptions.push(GraphTextView {
                            text: text.into(),
                            reference_id: reference_id.clone(),
                        });
                    }
                }
                for contribution in context
                    .contributions
                    .iter()
                    .filter(|n| n.properties["entityId"].as_str() == Some(&node.id))
                {
                    let Some(support) = context
                        .provenance
                        .iter()
                        .find(|e| {
                            e.relationship == "DESCRIBES"
                                && e.from == contribution.id
                                && e.to == node.id
                        })
                        .map(|e| &e.support)
                    else {
                        continue;
                    };
                    let reference_id = graph_reference_id(&context.partition, support);
                    if !reference_ids.contains(reference_id.as_str()) {
                        continue;
                    }
                    if let Some(text) = contribution.properties["description"].as_str() {
                        view.descriptions.push(GraphTextView {
                            text: text.into(),
                            reference_id,
                        });
                    }
                    for alias in contribution.properties["aliases"]
                        .as_array()
                        .into_iter()
                        .flatten()
                    {
                        let (Some(name), Some(quote)) =
                            (alias["name"].as_str(), alias["evidence"].as_str())
                        else {
                            continue;
                        };
                        let mut quoted = support.clone();
                        quoted.quote = quote.into();
                        let reference_id = graph_reference_id(&context.partition, &quoted);
                        if reference_ids.contains(reference_id.as_str()) {
                            view.aliases.push(GraphTextView {
                                text: name.into(),
                                reference_id,
                            });
                        }
                    }
                }
            }
        }
        for context in &output.contexts {
            for seed in &context.seeds {
                if nodes.contains_key(&seed.id) {
                    seeds
                        .entry(seed.id.clone())
                        .or_insert_with(|| GraphSeedView {
                            id: seed.id.clone(),
                            score: seed.score,
                            channels: seed.channels.iter().cloned().collect(),
                        });
                }
            }
            // Provenance is citation metadata. Only traversed relationships count
            // toward the returned edge window, including structural expansion.
            for edge in &context.edges {
                if !nodes.contains_key(&edge.from) || !nodes.contains_key(&edge.to) {
                    continue;
                }
                let reference_id = graph_reference_id(&context.partition, &edge.support);
                if !reference_ids.contains(reference_id.as_str()) {
                    continue;
                }
                edges
                    .entry(edge.logical_relation_id.clone())
                    .or_insert_with(|| GraphEdgeView {
                        id: edge.logical_relation_id.clone(),
                        from: edge.from.clone(),
                        to: edge.to.clone(),
                        relationship: edge.relationship.clone(),
                        structural: edge.structural,
                        reference_ids: vec![],
                    })
                    .reference_ids
                    .push(reference_id);
            }
        }
        for node in nodes.values_mut() {
            node.reference_ids.sort();
            node.reference_ids.dedup();
            for texts in [&mut node.descriptions, &mut node.aliases] {
                texts.sort_by(|a, b| (&a.reference_id, &a.text).cmp(&(&b.reference_id, &b.text)));
                texts.dedup_by(|a, b| a.reference_id == b.reference_id && a.text == b.text);
            }
            node.hops.sort_by(|a, b| {
                (&a.seed_id, a.direction, a.hops).cmp(&(&b.seed_id, b.direction, b.hops))
            });
            node.hops
                .dedup_by(|a, b| a.seed_id == b.seed_id && a.direction == b.direction);
        }
        for edge in edges.values_mut() {
            edge.reference_ids.sort();
            edge.reference_ids.dedup();
        }
        let mut seeds = seeds.into_values().collect::<Vec<_>>();
        seeds.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.id.cmp(&b.id)));
        let stages = output
            .stages
            .into_iter()
            .map(|s| GraphStageView {
                corpus_id: s.corpus_id,
                channel: s.channel,
                status: match s.status.as_str() {
                    "COMPLETED" => GraphStageStatus::Completed,
                    "TRUNCATED" => GraphStageStatus::Truncated,
                    "INVALIDATED" => GraphStageStatus::Invalidated,
                    "SKIPPED" => GraphStageStatus::Skipped,
                    _ => GraphStageStatus::Failed,
                },
                hit_count: s.hit_count,
                duration_ms: s.duration_ms,
                error_key: s.error.map(|_| "tron.rag.graph.channel_unavailable".into()),
            })
            .collect();
        Self {
            snapshot_id: output.snapshot_id,
            seeds,
            nodes: nodes.into_values().collect(),
            edges: edges.into_values().collect(),
            references: output.references,
            stages,
            next_cursor: output.next_cursor,
            expires_at: output.expires_at,
            unfinished: output.unfinished,
            truncated_by: output.truncated_by.into_iter().collect(),
            duration_ms: output.duration_ms,
        }
    }
}
