//! Document-local source evidence, independent of semantic entity traversal.
//! Section and asset nodes are leaves for grouping: neither a chapter nor a
//! repeated logo may merge otherwise unrelated original chunks.
use super::*;
use crate::application::rag::{
    RagEvidence, RagEvidenceBundle, RagEvidenceResult, RagSearchRepository, RagService,
};
use crate::domain::rag::graph::{RagGraphNode, RagGraphNodeKind};
use std::collections::{BTreeMap, VecDeque};
use std::time::Instant;

const NODE_BUDGET: usize = 500;
const EDGE_BUDGET: usize = 1000;

fn grouped(kind: RagGraphNodeKind) -> bool {
    matches!(
        kind,
        RagGraphNodeKind::SourceBlock
            | RagGraphNodeKind::Table
            | RagGraphNodeKind::Image
            | RagGraphNodeKind::SourceCaption
            | RagGraphNodeKind::OcrText
            | RagGraphNodeKind::GeneratedDescription
    )
}
fn original(kind: RagGraphNodeKind) -> bool {
    matches!(
        kind,
        RagGraphNodeKind::SourceBlock | RagGraphNodeKind::Table | RagGraphNodeKind::Image
    )
}

impl<R: RagSearchRepository + Clone, E: Clone> RagService<R, E> {
    pub(crate) fn assemble_source_bundles(
        &self,
        ranges: &[AuthorizedRagRange],
        result: &mut RagEvidenceResult,
    ) {
        if self.graph.is_none() || !ranges.iter().any(|r| r.graph) || result.hits.is_empty() {
            return;
        }
        let start = Instant::now();
        match self.source_bundles(ranges, &result.hits) {
            Ok((hits, limited)) => {
                result.hits = hits;
                if limited {
                    result.warnings.push(
                        crate::shared::i18n_message::MessageText::new(
                            "rag.graph.relation_budget_reached",
                        )
                        .encode()
                        .into(),
                    );
                }
                result.stages.push(RagRetrievalStage {
                    corpus_id: String::new(),
                    channel: "source.structure".into(),
                    status: if limited { "TRUNCATED" } else { "COMPLETED" }.into(),
                    hit_count: result.hits.iter().filter(|h| h.bundle.is_some()).count(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: None,
                });
            }
            Err(error) => {
                result.warnings.push(
                    crate::shared::i18n_message::MessageText::new(
                        "rag.graph.relation_temporarily_unavailable",
                    )
                    .arg("detail", error.to_string())
                    .encode(),
                );
                result.stages.push(RagRetrievalStage {
                    corpus_id: String::new(),
                    channel: "source.structure".into(),
                    status: "FAILED".into(),
                    hit_count: 0,
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: Some(error.to_string()),
                });
            }
        }
    }

    pub(crate) fn active_bundle_generations(
        &self,
        ranges: &[AuthorizedRagRange],
    ) -> Result<BTreeSet<String>, GraphQueryError> {
        Ok(self
            .graph_source_snapshot(ranges.to_vec())?
            .generation_ids())
    }

    fn source_bundles(
        &self,
        ranges: &[AuthorizedRagRange],
        hits: &[RagEvidence],
    ) -> Result<(Vec<RagEvidence>, bool), GraphQueryError> {
        let snapshot = self.graph_source_snapshot(ranges.to_vec())?;
        let infra = self
            .graph
            .as_ref()
            .ok_or_else(|| GraphQueryError::Unavailable("source graph".into()))?;
        let mut by_partition = BTreeMap::<GraphPartition, Vec<(&RagEvidence, String)>>::new();
        for hit in hits {
            if let Some(manifest) = snapshot.manifests.iter().find(|g| {
                g.partition.corpus_id == hit.row.corpus_id
                    && g.partition.scope_type == hit.row.scope_type
                    && g.partition.scope_code == hit.row.scope_code
                    && g.source_code == hit.row.source_code
                    && g.source_version == hit.row.source_version
            }) {
                by_partition
                    .entry(manifest.partition.clone())
                    .or_default()
                    .push((
                        hit,
                        stable_id("CHUNK", &[&manifest.id, &hit.row.target_code]),
                    ));
            }
        }
        let mut assigned = BTreeSet::new();
        let mut output = Vec::new();
        let mut limited = false;
        for (partition, anchors) in by_partition {
            let generations = snapshot
                .manifests
                .iter()
                .filter(|g| g.partition == partition)
                .map(|g| g.id.clone())
                .collect::<BTreeSet<_>>();
            let seeds = anchors
                .iter()
                .take(NODE_BUDGET / 2)
                .map(|(_, id)| id.clone())
                .collect::<BTreeSet<_>>();
            limited |= anchors.len() > NODE_BUDGET / 2;
            let mut nodes = BTreeMap::<String, RagGraphNode>::new();
            let mut edges = BTreeMap::new();
            let mut frontier = seeds.clone();
            let mut visited = BTreeSet::new();
            let mut local_limited = false;
            for depth in 0..=6 {
                if frontier.is_empty() {
                    break;
                }
                for batch in frontier
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .chunks(NODE_BUDGET)
                {
                    for node in infra
                        .store
                        .nodes(&partition, batch, &generations)
                        .map_err(GraphQueryError::Storage)?
                    {
                        nodes.insert(node.id.clone(), node);
                    }
                }
                let mut next = BTreeSet::new();
                for direction in [GraphDirection::Outgoing, GraphDirection::Incoming] {
                    let selected = frontier
                        .iter()
                        .filter(|id| {
                            nodes.get(*id).is_some_and(|node| match direction {
                                GraphDirection::Outgoing => {
                                    original(node.kind)
                                        || (depth == 0 && node.kind == RagGraphNodeKind::Chunk)
                                }
                                GraphDirection::Incoming => {
                                    original(node.kind) || node.kind == RagGraphNodeKind::Section
                                }
                            })
                        })
                        .cloned()
                        .collect::<BTreeSet<_>>();
                    if selected.is_empty() {
                        continue;
                    }
                    let remaining = EDGE_BUDGET.saturating_sub(edges.len());
                    if remaining == 0 {
                        local_limited = true;
                        break;
                    }
                    let relationships = if direction == GraphDirection::Outgoing {
                        [
                            "COVERS",
                            "REFERENCES_FIGURE",
                            "REFERENCES_TABLE",
                            "CONTAINS",
                            "HAS_IMAGE",
                            "HAS_ASSET",
                            "HAS_CAPTION",
                            "HAS_OCR",
                            "HAS_DESCRIPTION",
                        ]
                        .into_iter()
                        .map(str::to_owned)
                        .collect()
                    } else {
                        [
                            "COVERS",
                            "REFERENCES_FIGURE",
                            "REFERENCES_TABLE",
                            "CONTAINS",
                            "HAS_IMAGE",
                        ]
                        .into_iter()
                        .map(str::to_owned)
                        .collect()
                    };
                    let page = infra
                        .store
                        .structural_adjacent(&GraphStructuralQuery {
                            partition: partition.clone(),
                            node_ids: selected,
                            direction,
                            generations: generations.clone(),
                            relationships,
                            after_edge_id: None,
                            limit: remaining,
                        })
                        .map_err(GraphQueryError::Storage)?;
                    local_limited |= page.has_more;
                    for edge in page.edges {
                        let endpoint = if direction == GraphDirection::Outgoing {
                            &edge.to
                        } else {
                            &edge.from
                        };
                        if !visited.contains(endpoint) && !frontier.contains(endpoint) {
                            if nodes.len() + next.len() < NODE_BUDGET {
                                next.insert(endpoint.clone());
                            } else {
                                local_limited = true;
                                continue;
                            }
                        }
                        edges.insert(edge.id.clone(), edge);
                    }
                }
                visited.extend(frontier);
                if depth == 6 && !next.is_empty() {
                    local_limited = true;
                }
                frontier = next;
            }
            // Final endpoints reached at the depth boundary are visible leaves.
            for batch in frontier
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .chunks(NODE_BUDGET)
            {
                for node in infra
                    .store
                    .nodes(&partition, batch, &generations)
                    .map_err(GraphQueryError::Storage)?
                {
                    nodes.insert(node.id.clone(), node);
                }
            }
            edges.retain(|_, edge| nodes.contains_key(&edge.from) && nodes.contains_key(&edge.to));
            let supports = edges
                .values()
                .map(|edge| (partition.clone(), edge.support.clone()))
                .collect::<Vec<_>>();
            let originals = self.graph_originals(&snapshot, &supports)?;
            sources::references_from_rows(&originals, &supports)?;
            let mut originals_by_node = BTreeMap::new();
            for row in originals {
                if let Some(manifest) = snapshot.manifests.iter().find(|g| {
                    g.partition == partition
                        && g.source_code == row.source_code
                        && g.source_version == row.source_version
                }) {
                    originals_by_node
                        .insert(stable_id("CHUNK", &[&manifest.id, &row.target_code]), row);
                }
            }
            let mut adjacency = BTreeMap::<String, BTreeSet<String>>::new();
            for edge in edges.values() {
                if nodes.get(&edge.from).is_some_and(|n| grouped(n.kind))
                    && nodes.get(&edge.to).is_some_and(|n| grouped(n.kind))
                {
                    adjacency
                        .entry(edge.from.clone())
                        .or_default()
                        .insert(edge.to.clone());
                    adjacency
                        .entry(edge.to.clone())
                        .or_default()
                        .insert(edge.from.clone());
                }
            }
            let mut groups = BTreeMap::<Vec<String>, usize>::new();
            for (anchor, id) in &anchors {
                if !seeds.contains(id) {
                    continue;
                }
                // An anchor's own coverage is evidence. Other overlapping windows
                // may contribute fragments, but can never widen this block set.
                let mut component = edges
                    .values()
                    .filter(|edge| edge.from == *id && edge.relationship == "COVERS")
                    .map(|edge| edge.to.clone())
                    .collect::<BTreeSet<_>>();
                if component.is_empty() {
                    continue;
                }
                let mut queue = VecDeque::from_iter(component.iter().cloned());
                while let Some(current) = queue.pop_front() {
                    for next in adjacency.get(&current).into_iter().flatten() {
                        if component.insert(next.clone()) {
                            queue.push_back(next.clone());
                        }
                    }
                }
                let selected = component
                    .iter()
                    .filter_map(|id| nodes.get(id))
                    .filter(|node| original(node.kind))
                    .filter_map(|node| node.properties["sourceBlockId"].as_str().map(str::to_owned))
                    .collect::<BTreeSet<_>>();
                let mut grouped_hit = (*anchor).clone();
                grouped_hit.row.embedding = None;
                if !grouped_hit.select_blocks(selected.clone()) {
                    local_limited = true;
                    continue;
                }
                let core = component
                    .iter()
                    .filter(|id| {
                        nodes
                            .get(*id)
                            .is_none_or(|node| node.properties["blockKind"] != "heading")
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let key = if core.is_empty() {
                    component.iter().cloned().collect()
                } else {
                    core
                };
                assigned.insert(super::super::super::bundles::row_identity(&anchor.row));
                if let Some(index) = groups.get(&key).copied() {
                    let existing: &mut RagEvidence = &mut output[index];
                    existing.channels.extend(anchor.channels.iter().cloned());
                    for (channel, rank) in &anchor.recall_ranks {
                        existing
                            .recall_ranks
                            .entry(channel.clone())
                            .and_modify(|value| *value = (*value).min(*rank))
                            .or_insert(*rank);
                    }
                    if !existing.recall_ranks.is_empty() {
                        existing.score = existing
                            .recall_ranks
                            .values()
                            .map(|rank| 1. / (61. + *rank as f64))
                            .sum();
                    }
                    continue;
                }
                let chunk_ids = edges
                    .values()
                    .filter(|edge| edge.relationship == "COVERS" && component.contains(&edge.to))
                    .map(|edge| edge.from.clone())
                    .collect::<BTreeSet<_>>();
                let mut members = Vec::new();
                for member_id in &chunk_ids {
                    if member_id == id {
                        continue;
                    }
                    if let Some(row) = originals_by_node.get(member_id) {
                        let mut member = anchors
                            .iter()
                            .find(|(_, candidate)| candidate == member_id)
                            .map(|(hit, _)| (*hit).clone())
                            .unwrap_or_else(|| RagEvidence::associated(row.clone()));
                        member.row.embedding = None;
                        member.bundle = None;
                        if member.select_blocks(selected.clone()) {
                            members.push(member);
                        } else {
                            local_limited = true;
                        }
                    }
                }
                members.sort_by(|a, b| {
                    a.row
                        .ordinal
                        .cmp(&b.row.ordinal)
                        .then(a.row.target_code.cmp(&b.row.target_code))
                });
                let relevant = edges
                    .values()
                    .filter(|edge| component.contains(&edge.from) || component.contains(&edge.to))
                    .cloned()
                    .collect::<Vec<_>>();
                let node_ids = relevant
                    .iter()
                    .flat_map(|edge| [&edge.from, &edge.to])
                    .cloned()
                    .chain(component.iter().cloned())
                    .collect::<BTreeSet<_>>();
                let facts = node_ids
                    .iter()
                    .filter_map(|id| nodes.get(id).cloned())
                    .collect();
                let generation_ids = relevant
                    .iter()
                    .map(|edge| edge.support.generation_id.clone())
                    .collect();
                grouped_hit.bundle = Some(RagEvidenceBundle {
                    members,
                    display: vec![],
                    partition: partition.clone(),
                    generations: generation_ids,
                    nodes: facts,
                    edges: relevant,
                    truncated_by: if local_limited {
                        BTreeSet::from(["SourceStructureBudget".into()])
                    } else {
                        BTreeSet::new()
                    },
                });
                groups.insert(key, output.len());
                output.push(grouped_hit);
            }
            limited |= local_limited;
        }
        output.extend(
            hits.iter()
                .filter(|hit| {
                    !assigned.contains(&super::super::super::bundles::row_identity(&hit.row))
                })
                .cloned(),
        );
        output.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then(a.row.target_code.cmp(&b.row.target_code))
        });
        if self.graph_source_snapshot(ranges.to_vec())?.stamp != snapshot.stamp {
            return Err(GraphQueryError::SnapshotChanged);
        }
        Ok((output, limited))
    }
}
