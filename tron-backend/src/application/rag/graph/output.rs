//! Ordinary RAG exposes graph facts only with the original evidence it returns.
use super::{GraphRecallContext, GraphRecallHop};
use crate::{
    application::rag::RagEvidenceResult,
    domain::rag::graph::{GraphDirection, RagGraphSupport},
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(crate) fn retain_returned_evidence(result: &mut RagEvidenceResult) {
    let visible = result
        .hits
        .iter()
        .flat_map(|hit| {
            std::iter::once(hit).chain(hit.bundle.iter().flat_map(|bundle| bundle.members.iter()))
        })
        .map(|hit| {
            (
                hit.row.corpus_id.clone(),
                hit.row.source_code.clone(),
                hit.row.source_version.clone(),
                hit.row.scope_type.clone(),
                hit.row.scope_code.clone(),
                hit.row.target_code.clone(),
            )
        })
        .collect::<BTreeSet<_>>();
    for context in &mut result.graph_context {
        let supports = |s: &RagGraphSupport| {
            visible.contains(&(
                context.partition.corpus_id.clone(),
                s.source_code.clone(),
                s.source_version.clone(),
                context.partition.scope_type.clone(),
                context.partition.scope_code.clone(),
                s.chunk_code.clone(),
            ))
        };
        context.edges.retain(|e| supports(&e.support));
        context.provenance.retain(|e| supports(&e.support));
        let anchored = context
            .provenance
            .iter()
            .map(|e| e.to.clone())
            .collect::<BTreeSet<_>>();
        context.seeds.retain(|s| anchored.contains(&s.id));
        retain_connected_facts(context);
    }
    result.graph_context.retain(|c| !c.seeds.is_empty());
}

fn retain_connected_facts(context: &mut GraphRecallContext) {
    let mut retained_edges = BTreeSet::new();
    let mut retained_nodes = BTreeSet::new();
    let mut hops = Vec::new();
    for seed in &context.seeds {
        for direction in [GraphDirection::Incoming, GraphDirection::Outgoing] {
            let mut visited = BTreeMap::from([(seed.id.clone(), 0u8)]);
            let mut queue = VecDeque::from([seed.id.clone()]);
            while let Some(id) = queue.pop_front() {
                let depth = visited[&id];
                retained_nodes.insert(id.clone());
                hops.push(GraphRecallHop {
                    seed_id: seed.id.clone(),
                    node_id: id.clone(),
                    direction,
                    hops: depth,
                });
                if depth == 3 {
                    continue;
                }
                for edge in &context.edges {
                    let next = match direction {
                        GraphDirection::Incoming if edge.to == id => &edge.from,
                        GraphDirection::Outgoing if edge.from == id => &edge.to,
                        _ => continue,
                    };
                    retained_edges.insert(edge.id.clone());
                    if !visited.contains_key(next) {
                        visited.insert(next.clone(), depth + 1);
                        queue.push_back(next.clone());
                    }
                }
            }
        }
    }
    context.edges.retain(|e| retained_edges.contains(&e.id));
    context.nodes.retain(|n| retained_nodes.contains(&n.id));
    context
        .provenance
        .retain(|e| retained_nodes.contains(&e.to));
    let contributions = context
        .provenance
        .iter()
        .filter(|e| e.relationship == "DESCRIBES")
        .map(|e| e.from.as_str())
        .collect::<BTreeSet<_>>();
    context
        .contributions
        .retain(|n| contributions.contains(n.id.as_str()));
    context.hops = hops;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::rag::{
        RagEvidence, RagSearchRow,
        graph::{GraphExpansionState, GraphRecallSeed},
    };
    use crate::domain::rag::graph::{GraphPartition, RagGraphEdge, RagGraphNode, RagGraphNodeKind};
    use serde_json::json;

    fn evidence(source: &str) -> RagEvidence {
        RagEvidence {
            context_text: None,
            recall_ranks: Default::default(),
            bundle: None,
            selected_blocks: Default::default(),
            row: RagSearchRow {
                corpus_id: "manual".into(),
                source_code: source.into(),
                source_version: "v1".into(),
                target_code: source.into(),
                scope_type: "USER".into(),
                scope_code: "owner".into(),
                title: None,
                text: "Original quote".into(),
                search_text: "Original quote".into(),
                heading_path: vec![],
                ordinal: 0,
                token_count: 2,
                embedding: None,
                storage_score: None,
                source_metadata: json!({}),
                text_metadata: json!({}),
                field_values: Default::default(),
            },
            channels: BTreeSet::from(["graph".into()]),
            score: 1.,
        }
    }
    fn edge(id: &str, from: &str, to: &str, source: &str, relationship: &str) -> RagGraphEdge {
        RagGraphEdge {
            id: id.into(),
            logical_relation_id: id.into(),
            from: from.into(),
            to: to.into(),
            relationship: relationship.into(),
            structural: relationship == "MENTIONS",
            support: RagGraphSupport {
                generation_id: "g".into(),
                source_code: source.into(),
                source_version: "v1".into(),
                task_id: "task".into(),
                chunk_code: source.into(),
                quote: "Original quote".into(),
                location: json!({}),
            },
        }
    }
    #[test]
    fn omitted_original_evidence_cannot_bridge_nodes_or_leave_aliases_and_hop_counts() {
        let partition = GraphPartition {
            corpus_id: "manual".into(),
            scope_type: "USER".into(),
            scope_code: "owner".into(),
        };
        let generations = BTreeSet::from(["g".into()]);
        let context = GraphRecallContext {
            partition: partition.clone(),
            seeds: vec![GraphRecallSeed {
                id: "A".into(),
                score: 1.,
                channels: BTreeSet::new(),
            }],
            nodes: ["A", "B", "C"]
                .into_iter()
                .map(|id| RagGraphNode {
                    id: id.into(),
                    kind: RagGraphNodeKind::Entity,
                    generation_id: String::new(),
                    name: id.into(),
                    entity_type: Some("Concept".into()),
                    properties: json!({}),
                })
                .collect(),
            contributions: vec![],
            edges: vec![
                edge("ab", "A", "B", "doc-b", "LINKS"),
                edge("bc", "B", "C", "doc-c", "LINKS"),
            ],
            provenance: vec![
                edge("ma", "chunk-a", "A", "doc-a", "MENTIONS"),
                edge("mc", "chunk-c", "C", "doc-c", "MENTIONS"),
            ],
            hops: vec![],
            truncated_by: vec![],
            expansion: GraphExpansionState::new(
                partition,
                generations.clone(),
                &["A".into()],
                BTreeSet::new(),
                3,
                3,
            )
            .unwrap(),
            generations,
        };
        let mut result = RagEvidenceResult {
            hits: vec![evidence("doc-a"), evidence("doc-c")],
            graph_context: vec![context],
            ..Default::default()
        };
        retain_returned_evidence(&mut result);
        let context = &result.graph_context[0];
        assert!(context.edges.is_empty());
        assert_eq!(
            context
                .nodes
                .iter()
                .map(|n| n.id.as_str())
                .collect::<Vec<_>>(),
            vec!["A"]
        );
        assert_eq!(context.provenance.len(), 1);
        assert!(context.hops.iter().all(|h| h.node_id == "A" && h.hops == 0));
        result.hits.clear();
        retain_returned_evidence(&mut result);
        assert!(result.graph_context.is_empty());
    }
}
