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
