//! Bounded graph windows and provenance continuations, without full-graph loading.
use super::*;
use crate::application::rag::{RagSearchRepository, RagService};
use crate::domain::rag::graph::{GraphDirection, RagGraphNodeKind};
use std::collections::{BTreeMap, VecDeque};

const PROVENANCE_WINDOW: usize = 256;

impl<R: RagSearchRepository + Clone, E: Clone> RagService<R, E> {
    pub(super) fn graph_windows(
        &self,
        snapshot: &AuthorizedGraphSnapshot,
        seeds: &[GraphRecallSeed],
        options: &GraphReadOptions,
        require_all: bool,
    ) -> Result<Vec<GraphWindowState>, GraphQueryError> {
        if seeds.is_empty() {
            return Ok(vec![]);
        }
        let infra = self
            .graph
            .as_ref()
            .ok_or_else(|| GraphQueryError::Unavailable("graph infrastructure".into()))?;
        if seeds.len() > options.node_limit
            || ((options.incoming_depth > 0 || options.outgoing_depth > 0)
                && options.node_limit < seeds.len() + 2)
        {
            return Err(GraphQueryError::Invalid);
        }
        let mut groups = BTreeMap::<GraphPartition, BTreeSet<String>>::new();
        for manifest in &snapshot.manifests {
            groups
                .entry(manifest.partition.clone())
                .or_default()
                .insert(manifest.id.clone());
        }
        let ids = seeds.iter().map(|s| s.id.clone()).collect::<Vec<_>>();
        let mut windows = Vec::new();
        let mut found = BTreeSet::new();
        for (partition, generations) in groups {
            let nodes = infra
                .store
                .nodes(&partition, &ids, &generations)
                .map_err(GraphQueryError::Storage)?;
            let visible = nodes
                .into_iter()
                .filter(|n| {
                    n.kind != RagGraphNodeKind::EntityContribution
                        && (options.include_structure || n.kind == RagGraphNodeKind::Entity)
                })
                .map(|n| n.id)
                .collect::<BTreeSet<_>>();
            let selected = seeds
                .iter()
                .filter(|s| visible.contains(&s.id))
                .cloned()
                .collect::<Vec<_>>();
            if selected.is_empty() {
                continue;
            }
            found.extend(selected.iter().map(|s| s.id.clone()));
            let expansion = GraphExpansionState::new(
                partition.clone(),
                generations.clone(),
                &selected.iter().map(|s| s.id.clone()).collect::<Vec<_>>(),
                options.relationship_types.clone(),
                options.incoming_depth,
                options.outgoing_depth,
            )
            .map_err(|_| GraphQueryError::Invalid)?
            .with_structure(options.include_structure);
            windows.push(GraphWindowState {
                partition,
                generations,
                seeds: selected,
                expansion,
                pending_provenance: None,
            });
        }
        if require_all && found.len() != seeds.len() {
            return Err(GraphQueryError::NodeNotFound);
        }
        let rank = seeds
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.as_str(), i))
            .collect::<BTreeMap<_, _>>();
        windows.sort_by_key(|w| {
            w.seeds
                .iter()
                .filter_map(|s| rank.get(s.id.as_str()).copied())
                .min()
                .unwrap_or(usize::MAX)
        });
        Ok(windows)
    }

    pub(super) fn read_graph_windows(
        &self,
        windows: Vec<GraphWindowState>,
        options: &GraphReadOptions,
        verify: &dyn Fn() -> Result<(), GraphQueryError>,
    ) -> Result<GraphWindowRead, GraphQueryError> {
        if windows.is_empty() {
            verify()?;
            return Ok(Default::default());
        }
        let infra = self
            .graph
            .as_ref()
            .ok_or_else(|| GraphQueryError::Unavailable("graph infrastructure".into()))?;
        let mut queue = VecDeque::from(windows);
        let mut deferred = Vec::new();
        let mut continued = Vec::new();
        let mut result = GraphWindowRead::default();
        let mut nodes_left = options.node_limit;
        let mut edges_left = options.edge_limit;
        let mut provenance_left = PROVENANCE_WINDOW;
        verify()?;
        while let Some(mut state) = queue.pop_front() {
            let required = state
                .pending_provenance
                .as_ref()
                .map(|p| p.node_ids.len())
                .unwrap_or(state.seeds.len());
            if required > nodes_left || provenance_left == 0 {
                result.truncated_by.insert(
                    if provenance_left == 0 {
                        "ProvenanceBudget"
                    } else {
                        "NodeBudget"
                    }
                    .into(),
                );
                deferred.push(state);
                continue;
            }
            let mut truncated = BTreeSet::new();
            let (window, after) = if let Some(pending) = state.pending_provenance.take() {
                let distances = pending
                    .hops
                    .into_iter()
                    .map(|h| ((h.seed_id, h.direction, h.node_id), h.hops))
                    .collect();
                (
                    GraphExpansionWindow {
                        node_ids: pending.node_ids,
                        distances,
                        has_more: state.expansion.has_more(),
                        ..Default::default()
                    },
                    pending.after,
                )
            } else {
                // Reserve other partitions' seed identities so the initial page
                // can expose all chosen seeds without starving their continuations.
                let reserve = queue
                    .iter()
                    .filter(|w| w.pending_provenance.is_none())
                    .map(|w| w.seeds.len())
                    .sum::<usize>();
                let budget = nodes_left.saturating_sub(reserve).max(state.seeds.len());
                if edges_left == 0 {
                    truncated.insert("EdgeBudget".into());
                    (seed_window(&state), None)
                } else {
                    let failure = std::cell::RefCell::new(None);
                    let expanded = state.expansion.expand_checked(
                        infra.store.as_ref(),
                        GraphTraversalLimits {
                            node_limit: budget,
                            edge_limit: edges_left,
                        },
                        || {
                            verify().map_err(|error| {
                                *failure.borrow_mut() = Some(error.clone());
                                "graph authorization snapshot changed".to_string()
                            })
                        },
                        |support| Ok(state.generations.contains(&support.generation_id)),
                    );
                    let window = expanded.map_err(|error| {
                        failure
                            .into_inner()
                            .unwrap_or(GraphQueryError::Storage(error))
                    })?;
                    (window, None)
                }
            };
            nodes_left -= window.node_ids.len();
            edges_left -= window.edges.len();
            truncated.extend(window.truncated_by.iter().cloned());
            verify()?;
            let evidence = infra
                .store
                .entity_evidence(
                    &state.partition,
                    &window.node_ids.iter().cloned().collect::<Vec<_>>(),
                    &state.generations,
                    after.as_deref(),
                    provenance_left.min(1000),
                )
                .map_err(GraphQueryError::Storage)?;
            provenance_left -= evidence.edges.len();
            if evidence.has_more {
                let hops = window
                    .distances
                    .iter()
                    .map(|((seed_id, direction, node_id), hops)| GraphRecallHop {
                        seed_id: seed_id.clone(),
                        direction: *direction,
                        node_id: node_id.clone(),
                        hops: *hops,
                    })
                    .collect();
                state.pending_provenance = Some(PendingProvenance {
                    node_ids: window.node_ids.clone(),
                    after: evidence.edges.last().map(|e| e.id.clone()),
                    hops,
                });
                truncated.insert("ProvenanceBudget".into());
            }
            let mut node_ids = window.node_ids.clone();
            node_ids.extend(
                evidence
                    .edges
                    .iter()
                    .filter(|e| e.relationship == "DESCRIBES")
                    .map(|e| e.from.clone()),
            );
            let all_nodes = infra
                .store
                .nodes(
                    &state.partition,
                    &node_ids.into_iter().collect::<Vec<_>>(),
                    &state.generations,
                )
                .map_err(GraphQueryError::Storage)?;
            let (contributions, nodes): (Vec<_>, Vec<_>) = all_nodes
                .into_iter()
                .partition(|n| n.kind == RagGraphNodeKind::EntityContribution);
            let hops = window
                .distances
                .into_iter()
                .map(|((seed_id, direction, node_id), hops)| GraphRecallHop {
                    seed_id,
                    node_id,
                    direction,
                    hops,
                })
                .collect();
            let context = GraphRecallContext {
                partition: state.partition.clone(),
                seeds: state.seeds.clone(),
                nodes,
                contributions,
                edges: window.edges,
                provenance: evidence.edges,
                hops,
                truncated_by: truncated.iter().cloned().collect(),
                expansion: state.expansion.clone(),
                generations: state.generations.clone(),
            };
            collect_supports(&context, &mut result.supports);
            result.contexts.push(context);
            result.truncated_by.extend(truncated);
            if state.expansion.has_more() || state.pending_provenance.is_some() {
                continued.push(state);
            }
        }
        deferred.extend(continued);
        result.pending = deferred;
        if !result.pending.is_empty() && result.truncated_by.is_empty() {
            result.truncated_by.insert("GraphWindowBudget".into());
        }
        verify()?;
        Ok(result)
    }
}

fn seed_window(state: &GraphWindowState) -> GraphExpansionWindow {
    let mut window = GraphExpansionWindow {
        node_ids: state.seeds.iter().map(|s| s.id.clone()).collect(),
        has_more: state.expansion.has_more(),
        ..Default::default()
    };
    for seed in &state.seeds {
        for direction in [GraphDirection::Incoming, GraphDirection::Outgoing] {
            window
                .distances
                .insert((seed.id.clone(), direction, seed.id.clone()), 0);
        }
    }
    window
}
fn collect_supports(
    context: &GraphRecallContext,
    output: &mut Vec<(GraphPartition, RagGraphSupport)>,
) {
    output.extend(
        context
            .edges
            .iter()
            .chain(&context.provenance)
            .map(|edge| (context.partition.clone(), edge.support.clone())),
    );
    // Aliases are separately quoted source declarations, not inferred synonyms.
    for contribution in &context.contributions {
        let Some(support) = context
            .provenance
            .iter()
            .find(|e| e.relationship == "DESCRIBES" && e.from == contribution.id)
            .map(|e| &e.support)
        else {
            continue;
        };
        if let Some(aliases) = contribution.properties["aliases"].as_array() {
            for alias in aliases {
                if let Some(quote) = alias["evidence"].as_str() {
                    let mut source = support.clone();
                    source.quote = quote.into();
                    output.push((context.partition.clone(), source));
                }
            }
        }
    }
}

pub(super) fn unfinished(windows: &[GraphWindowState]) -> Vec<GraphUnfinishedPosition> {
    let mut result = Vec::new();
    for state in windows {
        result.extend(state.expansion.positions().into_iter().map(
            |(node_id, direction, depth)| GraphUnfinishedPosition {
                node_id,
                direction: Some(direction),
                depth: Some(depth),
                phase: GraphContinuationPhase::Traversal,
            },
        ));
        if let Some(pending) = &state.pending_provenance {
            result.extend(
                pending
                    .node_ids
                    .iter()
                    .map(|node_id| GraphUnfinishedPosition {
                        node_id: node_id.clone(),
                        direction: None,
                        depth: None,
                        phase: GraphContinuationPhase::Provenance,
                    }),
            );
        }
    }
    result
}
