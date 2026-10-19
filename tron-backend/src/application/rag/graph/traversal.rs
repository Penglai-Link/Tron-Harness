//! Direction-specific BFS with resumable edge positions; never enumerates paths.
use super::{GraphAdjacencyQuery, RagGraphStore};
use crate::domain::rag::graph::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug)]
pub struct GraphTraversalLimits {
    pub node_limit: usize,
    pub edge_limit: usize,
}
impl Default for GraphTraversalLimits {
    fn default() -> Self {
        Self {
            node_limit: 100,
            edge_limit: 200,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Frontier {
    seed: String,
    node: String,
    depth: u8,
    direction: GraphDirection,
    after: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphExpansionState {
    pub partition: GraphPartition,
    pub generations: BTreeSet<String>,
    pub relationship_types: BTreeSet<String>,
    seeds: Vec<String>,
    incoming_depth: u8,
    outgoing_depth: u8,
    incoming: VecDeque<Frontier>,
    outgoing: VecDeque<Frontier>,
    visited: BTreeSet<(String, GraphDirection, String)>,
    next_incoming: bool,
    include_structure: bool,
}
#[derive(Clone, Debug, Default)]
pub struct GraphExpansionWindow {
    pub node_ids: BTreeSet<String>,
    pub edges: Vec<RagGraphEdge>,
    /// Per seed and direction, shortest hop count for each returned node.
    pub distances: BTreeMap<(String, GraphDirection, String), u8>,
    pub has_more: bool,
    pub truncated_by: Vec<String>,
}
impl GraphExpansionState {
    pub fn new(
        partition: GraphPartition,
        generations: BTreeSet<String>,
        seeds: &[String],
        relationship_types: BTreeSet<String>,
        incoming_depth: u8,
        outgoing_depth: u8,
    ) -> Result<Self, String> {
        if incoming_depth > 6 || outgoing_depth > 6 || seeds.is_empty() || seeds.len() > 100 {
            return Err("invalid graph seed or depth budget".into());
        }
        let mut state = Self {
            partition,
            generations,
            relationship_types,
            seeds: seeds
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            incoming_depth,
            outgoing_depth,
            incoming: VecDeque::new(),
            outgoing: VecDeque::new(),
            visited: BTreeSet::new(),
            next_incoming: true,
            include_structure: false,
        };
        for seed in seeds.iter().collect::<BTreeSet<_>>() {
            for direction in [GraphDirection::Incoming, GraphDirection::Outgoing] {
                state
                    .visited
                    .insert((seed.clone(), direction, seed.clone()));
                if state.depth(direction) > 0 {
                    state.queue(direction).push_back(Frontier {
                        seed: seed.clone(),
                        node: seed.clone(),
                        depth: 0,
                        direction,
                        after: None,
                    });
                }
            }
        }
        Ok(state)
    }
    pub fn with_structure(mut self, enabled: bool) -> Self {
        self.include_structure = enabled;
        self
    }
    pub fn has_more(&self) -> bool {
        !self.incoming.is_empty() || !self.outgoing.is_empty()
    }
    pub fn positions(&self) -> Vec<(String, GraphDirection, u8)> {
        self.incoming
            .iter()
            .chain(&self.outgoing)
            .map(|p| (p.node.clone(), p.direction, p.depth))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    fn depth(&self, direction: GraphDirection) -> u8 {
        match direction {
            GraphDirection::Incoming => self.incoming_depth,
            GraphDirection::Outgoing => self.outgoing_depth,
        }
    }
    fn queue(&mut self, direction: GraphDirection) -> &mut VecDeque<Frontier> {
        match direction {
            GraphDirection::Incoming => &mut self.incoming,
            GraphDirection::Outgoing => &mut self.outgoing,
        }
    }
    /// The caller persists this state behind an opaque cursor. Never accept it from a client.
    /// Revalidate support publication and authorization before entering every next hop.
    pub fn expand(
        &mut self,
        store: &dyn RagGraphStore,
        limits: GraphTraversalLimits,
        mut authorize: impl FnMut(&RagGraphSupport) -> Result<bool, String>,
    ) -> Result<GraphExpansionWindow, String> {
        self.expand_checked(store, limits, || Ok(()), &mut authorize)
    }
    pub fn expand_checked(
        &mut self,
        store: &dyn RagGraphStore,
        limits: GraphTraversalLimits,
        mut verify_layer: impl FnMut() -> Result<(), String>,
        mut authorize: impl FnMut(&RagGraphSupport) -> Result<bool, String>,
    ) -> Result<GraphExpansionWindow, String> {
        if !(1..=500).contains(&limits.node_limit) || !(1..=1000).contains(&limits.edge_limit) {
            return Err("invalid graph window budget".into());
        }
        // Work on a clone: errors never consume an existing continuation position.
        let mut next = self.clone();
        let mut result = GraphExpansionWindow::default();
        if self.seeds.len() > limits.node_limit {
            return Err("graph seed count exceeds node window budget".into());
        }
        // Isolated seeds and depth-zero searches are still real graph results.
        result.node_ids.extend(self.seeds.iter().cloned());
        for seed in &self.seeds {
            for direction in [GraphDirection::Incoming, GraphDirection::Outgoing] {
                result
                    .distances
                    .insert((seed.clone(), direction, seed.clone()), 0);
            }
        }
        let mut seen_edges = BTreeSet::new();
        let mut verified_layers = BTreeSet::new();
        while !next.incoming.is_empty() || !next.outgoing.is_empty() {
            let preferred = if next.next_incoming {
                GraphDirection::Incoming
            } else {
                GraphDirection::Outgoing
            };
            next.next_incoming = !next.next_incoming;
            let direction = if next.queue(preferred).is_empty() {
                match preferred {
                    GraphDirection::Incoming => GraphDirection::Outgoing,
                    GraphDirection::Outgoing => GraphDirection::Incoming,
                }
            } else {
                preferred
            };
            let mut frontier = next.queue(direction).pop_front().unwrap();
            if verified_layers.insert((direction, frontier.depth)) {
                verify_layer()?;
            }
            let page = store.adjacent(&GraphAdjacencyQuery {
                partition: next.partition.clone(),
                node_id: frontier.node.clone(),
                direction,
                generations: next.generations.clone(),
                relationship_types: next.relationship_types.clone(),
                include_structural: next.include_structure,
                after_edge_id: frontier.after.clone(),
                limit: (limits.edge_limit - result.edges.len()).max(1).min(1000),
            })?;
            let mut paused = false;
            for edge in page.edges {
                if !authorize(&edge.support)? {
                    // Snapshot changes must be handled by the resolver; invisible supports never bridge a hop.
                    frontier.after = Some(edge.id.clone());
                    continue;
                }
                let other = match direction {
                    GraphDirection::Incoming => &edge.from,
                    GraphDirection::Outgoing => &edge.to,
                };
                let missing = [&frontier.node, other]
                    .into_iter()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .filter(|id| !result.node_ids.contains(*id))
                    .count();
                if result.node_ids.len() + missing > limits.node_limit {
                    result.truncated_by.push("NodeBudget".into());
                    paused = true;
                    break;
                }
                if !seen_edges.contains(&edge.id) && result.edges.len() >= limits.edge_limit {
                    result.truncated_by.push("EdgeBudget".into());
                    paused = true;
                    break;
                }
                result.node_ids.insert(frontier.node.clone());
                result.node_ids.insert(other.clone());
                result.distances.insert(
                    (frontier.seed.clone(), direction, frontier.node.clone()),
                    frontier.depth,
                );
                let key = (frontier.seed.clone(), direction, other.clone());
                let depth = frontier.depth + 1;
                result
                    .distances
                    .entry(key.clone())
                    .and_modify(|d| *d = (*d).min(depth))
                    .or_insert(depth);
                if next.visited.insert(key) && depth < next.depth(direction) {
                    next.queue(direction).push_back(Frontier {
                        seed: frontier.seed.clone(),
                        node: other.clone(),
                        depth,
                        direction,
                        after: None,
                    });
                }
                frontier.after = Some(edge.id.clone());
                if seen_edges.insert(edge.id.clone()) {
                    result.edges.push(edge);
                }
            }
            if paused || page.has_more {
                next.queue(direction).push_front(frontier);
            }
            if paused {
                break;
            }
        }
        result.has_more = !next.incoming.is_empty() || !next.outgoing.is_empty();
        result.truncated_by.sort();
        result.truncated_by.dedup();
        *self = next;
        Ok(result)
    }
}
