use std::collections::{HashMap, HashSet};

use crate::domain::graph::{
    GraphEdge, GraphFilterOperator, GraphPath, GraphPropertyFilter, GraphPropertyValue,
    GraphSearchTraversal, GraphTraversalClause, GraphTraversalDirection, GraphTraverseQuery,
    ResolvedGraphScope,
};

use super::grafeo_graph_records::{
    StoredGraphEdgeRecord, StoredGraphNodeRecord, visible_scope_match,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct NodeKey {
    pub node_code: String,
    pub workspace_code: String,
}

impl NodeKey {
    fn new(node_code: &str, workspace_code: &str) -> Self {
        Self {
            node_code: node_code.to_owned(),
            workspace_code: workspace_code.to_owned(),
        }
    }
}

pub(super) struct VisibleGraph {
    nodes: HashMap<NodeKey, StoredGraphNodeRecord>,
    preferred_keys_by_code: HashMap<String, NodeKey>,
    edges: Vec<StoredGraphEdgeRecord>,
    edge_indices_by_code: HashMap<String, usize>,
    outgoing: HashMap<NodeKey, Vec<usize>>,
    incoming: HashMap<NodeKey, Vec<usize>>,
}

impl VisibleGraph {
    pub(super) fn new(
        scope: &ResolvedGraphScope,
        nodes: Vec<StoredGraphNodeRecord>,
        edges: Vec<StoredGraphEdgeRecord>,
    ) -> Self {
        let rank = scope
            .visible_workspace_codes
            .iter()
            .enumerate()
            .map(|(index, code)| (code.clone(), index))
            .collect::<HashMap<_, _>>();

        let mut node_map = HashMap::new();
        let mut preferred_keys_by_code = HashMap::new();
        for node in nodes {
            let key = NodeKey::new(&node.node.node_code, &node.workspace_code);
            let current_rank = rank.get(&node.workspace_code).copied().unwrap_or(0);
            let replace = preferred_keys_by_code
                .get(&node.node.node_code)
                .and_then(|existing: &NodeKey| rank.get(&existing.workspace_code).copied())
                .is_none_or(|existing_rank| current_rank >= existing_rank);
            if replace {
                preferred_keys_by_code.insert(node.node.node_code.clone(), key.clone());
            }
            node_map.insert(key, node);
        }

        let mut stored_edges = Vec::new();
        let mut edge_indices_by_code = HashMap::new();
        let mut outgoing: HashMap<NodeKey, Vec<usize>> = HashMap::new();
        let mut incoming: HashMap<NodeKey, Vec<usize>> = HashMap::new();
        for edge in edges {
            let source = NodeKey::new(&edge.edge.source_node_code, &edge.source_workspace_code);
            let target = NodeKey::new(&edge.edge.target_node_code, &edge.target_workspace_code);
            if !node_map.contains_key(&source) || !node_map.contains_key(&target) {
                continue;
            }

            let index = stored_edges.len();
            edge_indices_by_code.insert(edge.edge.edge_code.clone(), index);
            outgoing.entry(source).or_default().push(index);
            incoming.entry(target).or_default().push(index);
            stored_edges.push(edge);
        }

        Self {
            nodes: node_map,
            preferred_keys_by_code,
            edges: stored_edges,
            edge_indices_by_code,
            outgoing,
            incoming,
        }
    }

    pub(super) fn from_scope(
        scope: &ResolvedGraphScope,
        nodes: Vec<StoredGraphNodeRecord>,
        edges: Vec<StoredGraphEdgeRecord>,
    ) -> Self {
        let visible_nodes = nodes
            .into_iter()
            .filter(|record| visible_scope_match(record, scope))
            .collect::<Vec<_>>();
        let visible_edges = edges
            .into_iter()
            .filter(|record| visible_scope_match(record, scope))
            .collect::<Vec<_>>();
        Self::new(scope, visible_nodes, visible_edges)
    }

    pub(super) fn preferred_node(&self, node_code: &str) -> Option<&StoredGraphNodeRecord> {
        self.preferred_keys_by_code
            .get(node_code)
            .and_then(|key| self.nodes.get(key))
    }

    pub(super) fn preferred_edge(&self, edge_code: &str) -> Option<&StoredGraphEdgeRecord> {
        self.edge_indices_by_code
            .get(edge_code)
            .and_then(|index| self.edges.get(*index))
    }

    pub(super) fn all_preferred_codes(&self) -> Vec<String> {
        self.preferred_keys_by_code.keys().cloned().collect()
    }

    pub(super) fn search_traversal(&self, traversal: &GraphSearchTraversal) -> Vec<String> {
        let mut current = traversal
            .start_node_codes
            .iter()
            .filter_map(|code| self.preferred_keys_by_code.get(code).cloned())
            .collect::<Vec<_>>();
        for clause in &traversal.clauses {
            let mut next = HashSet::new();
            for key in &current {
                for expanded in self.expand_clause_endpoints(key, clause) {
                    next.insert(expanded);
                }
            }
            current = next.into_iter().collect();
        }

        current
            .into_iter()
            .filter_map(|key| self.nodes.get(&key))
            .map(|record| record.node.node_code.clone())
            .collect()
    }

    pub(super) fn traverse(&self, query: &GraphTraverseQuery) -> Vec<GraphPath> {
        let start_keys = query
            .start_node_codes
            .iter()
            .filter_map(|code| self.preferred_keys_by_code.get(code).cloned())
            .collect::<Vec<_>>();
        let mut states = start_keys
            .into_iter()
            .map(|key| PathState {
                node_keys: vec![key],
                edges: Vec::new(),
            })
            .collect::<Vec<_>>();

        for clause in &query.clauses {
            let mut next = Vec::new();
            for state in states {
                next.extend(self.expand_clause_paths(state, clause));
                if next.len() >= query.limit.value() {
                    break;
                }
            }
            states = next;
            if states.is_empty() {
                break;
            }
        }

        states
            .into_iter()
            .filter_map(|state| {
                let last = state.node_keys.last()?;
                let last_record = self.nodes.get(last)?;
                filters_match(&last_record.node.properties, &query.filters).then(|| GraphPath {
                    nodes: state
                        .node_keys
                        .iter()
                        .filter_map(|key| self.nodes.get(key))
                        .map(|record| record.node.clone())
                        .collect(),
                    edges: state.edges,
                })
            })
            .take(query.limit.value())
            .collect()
    }

    fn expand_clause_endpoints(
        &self,
        start: &NodeKey,
        clause: &GraphTraversalClause,
    ) -> Vec<NodeKey> {
        self.expand_clause_paths(
            PathState {
                node_keys: vec![start.clone()],
                edges: Vec::new(),
            },
            clause,
        )
        .into_iter()
        .filter_map(|state| state.node_keys.last().cloned())
        .collect()
    }

    fn expand_clause_paths(
        &self,
        state: PathState,
        clause: &GraphTraversalClause,
    ) -> Vec<PathState> {
        let mut results = Vec::new();
        self.expand_clause_paths_inner(state, clause, 0, &mut results);
        results
    }

    fn expand_clause_paths_inner(
        &self,
        state: PathState,
        clause: &GraphTraversalClause,
        depth: usize,
        results: &mut Vec<PathState>,
    ) {
        if depth >= clause.min_depth && depth <= clause.max_depth {
            results.push(state.clone());
        }
        if depth == clause.max_depth {
            return;
        }

        let Some(current) = state.node_keys.last() else {
            return;
        };
        for (edge, next_key) in self.next_hops(current, clause) {
            let mut next_state = state.clone();
            next_state.edges.push(edge.clone());
            next_state.node_keys.push(next_key);
            self.expand_clause_paths_inner(next_state, clause, depth + 1, results);
        }
    }

    fn next_hops(
        &self,
        current: &NodeKey,
        clause: &GraphTraversalClause,
    ) -> Vec<(GraphEdge, NodeKey)> {
        let mut hops = Vec::new();
        if matches!(
            clause.direction,
            GraphTraversalDirection::Outgoing | GraphTraversalDirection::Any
        ) {
            if let Some(indices) = self.outgoing.get(current) {
                for index in indices {
                    let edge = &self.edges[*index];
                    if relationship_match(edge, clause) {
                        hops.push((
                            edge.edge.clone(),
                            NodeKey::new(&edge.edge.target_node_code, &edge.target_workspace_code),
                        ));
                    }
                }
            }
        }
        if matches!(
            clause.direction,
            GraphTraversalDirection::Incoming | GraphTraversalDirection::Any
        ) {
            if let Some(indices) = self.incoming.get(current) {
                for index in indices {
                    let edge = &self.edges[*index];
                    if relationship_match(edge, clause) {
                        hops.push((
                            edge.edge.clone(),
                            NodeKey::new(&edge.edge.source_node_code, &edge.source_workspace_code),
                        ));
                    }
                }
            }
        }

        hops
    }
}

#[derive(Clone)]
struct PathState {
    node_keys: Vec<NodeKey>,
    edges: Vec<GraphEdge>,
}

pub(super) fn labels_match(labels: &[String], required: &[String]) -> bool {
    required
        .iter()
        .all(|label| labels.iter().any(|value| value == label))
}

pub(super) fn full_text_match(full_text: Option<&str>, required: Option<&str>) -> bool {
    let Some(required) = required else {
        return true;
    };
    full_text
        .map(|value| value.to_lowercase().contains(&required.to_lowercase()))
        .unwrap_or(false)
}

pub(super) fn filters_match(
    properties: &std::collections::BTreeMap<String, GraphPropertyValue>,
    filters: &[GraphPropertyFilter],
) -> bool {
    filters.iter().all(|filter| {
        let Some(value) = properties.get(&filter.property_name) else {
            return false;
        };
        compare_property_values(value, &filter.operator, &filter.value)
    })
}

fn compare_property_values(
    left: &GraphPropertyValue,
    operator: &GraphFilterOperator,
    right: &GraphPropertyValue,
) -> bool {
    match operator {
        GraphFilterOperator::Equals => left == right,
        GraphFilterOperator::NotEquals => left != right,
        GraphFilterOperator::Contains => match (left, right) {
            (GraphPropertyValue::Text(left), GraphPropertyValue::Text(right)) => {
                left.to_lowercase().contains(&right.to_lowercase())
            }
            _ => false,
        },
        GraphFilterOperator::GreaterThan => compare_numeric(left, right, |l, r| l > r),
        GraphFilterOperator::GreaterThanOrEquals => compare_numeric(left, right, |l, r| l >= r),
        GraphFilterOperator::LessThan => compare_numeric(left, right, |l, r| l < r),
        GraphFilterOperator::LessThanOrEquals => compare_numeric(left, right, |l, r| l <= r),
    }
}

fn compare_numeric(
    left: &GraphPropertyValue,
    right: &GraphPropertyValue,
    comparator: impl Fn(f64, f64) -> bool,
) -> bool {
    match (left, right) {
        (GraphPropertyValue::Integer(left), GraphPropertyValue::Integer(right)) => {
            comparator(*left as f64, *right as f64)
        }
        (GraphPropertyValue::Integer(left), GraphPropertyValue::Decimal(right)) => {
            comparator(*left as f64, *right)
        }
        (GraphPropertyValue::Decimal(left), GraphPropertyValue::Integer(right)) => {
            comparator(*left, *right as f64)
        }
        (GraphPropertyValue::Decimal(left), GraphPropertyValue::Decimal(right)) => {
            comparator(*left, *right)
        }
        _ => false,
    }
}

fn relationship_match(edge: &StoredGraphEdgeRecord, clause: &GraphTraversalClause) -> bool {
    clause.relationships.is_empty()
        || clause
            .relationships
            .iter()
            .any(|relationship| relationship == &edge.edge.relationship)
}
