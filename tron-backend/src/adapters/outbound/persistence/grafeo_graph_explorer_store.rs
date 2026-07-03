use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::{
    grafeo_core::{GrafeoStoreCore, decode_schema_name, encode_graph_name},
    grafeo_graph_records::{
        StoredGraphEdgeRecord, StoredGraphNodeRecord, load_edges, load_nodes, storage_error,
    },
};
use crate::{
    application::graph::{
        GraphExplorerEdge, GraphExplorerNamespaceScope, GraphExplorerNode,
        GraphExplorerResolvedScope, GraphExplorerScopeType, GraphExplorerStore,
        GraphExplorerVisibleGraph,
    },
    domain::graph::{GraphError, STORAGE_ABSENT_SENTINEL},
};

#[derive(Clone)]
pub struct GrafeoGraphExplorerStore {
    core: GrafeoStoreCore,
}

impl GrafeoGraphExplorerStore {
    pub fn new(core: GrafeoStoreCore) -> Self {
        Self { core }
    }
}

impl GraphExplorerStore for GrafeoGraphExplorerStore {
    fn list_namespaces(
        &self,
        scope: &GraphExplorerNamespaceScope,
    ) -> Result<Vec<String>, GraphError> {
        let session = self.core.session();
        let root_graph = encode_graph_name(&scope.root_workspace_code);
        let mut namespaces = Vec::new();

        for schema in self.core.schema_names().map_err(storage_error)? {
            let Some(namespace) = decode_schema_name(&schema) else {
                continue;
            };

            session
                .execute(&format!("SESSION SET SCHEMA {schema}"))
                .map_err(storage_error)?;
            let graphs = session.execute("SHOW GRAPHS").map_err(storage_error)?;
            if !graphs.rows().iter().any(|row| {
                row.first().and_then(|value| value.as_str()) == Some(root_graph.as_str())
            }) {
                continue;
            }

            session
                .execute(&format!("USE GRAPH {root_graph}"))
                .map_err(storage_error)?;
            if namespace_has_visible_data(
                &namespace,
                load_nodes(&session)?,
                load_edges(&session)?,
                scope,
            ) {
                namespaces.push(namespace);
            }
        }

        namespaces.sort();
        namespaces.dedup();
        Ok(namespaces)
    }

    fn visible_graph(
        &self,
        scope: &GraphExplorerResolvedScope,
    ) -> Result<GraphExplorerVisibleGraph, GraphError> {
        let session = self
            .core
            .session_for_scope(&scope.graph)
            .map_err(storage_error)?;
        let workspace_rank = scope
            .graph
            .visible_workspace_codes
            .iter()
            .enumerate()
            .map(|(index, code)| (code.clone(), index))
            .collect::<HashMap<_, _>>();
        let visible_users = scope
            .visible_user_codes
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();

        let nodes = preferred_nodes(
            load_nodes(&session)?,
            scope,
            &workspace_rank,
            &visible_users,
        );
        let visible_node_codes = nodes.keys().cloned().collect::<BTreeSet<_>>();
        let edges = preferred_edges(
            load_edges(&session)?,
            scope,
            &workspace_rank,
            &visible_users,
            &visible_node_codes,
        );

        Ok(GraphExplorerVisibleGraph {
            nodes: nodes.into_values().collect(),
            edges: edges.into_values().collect(),
        })
    }
}

fn namespace_has_visible_data(
    namespace: &str,
    nodes: Vec<StoredGraphNodeRecord>,
    edges: Vec<StoredGraphEdgeRecord>,
    scope: &GraphExplorerNamespaceScope,
) -> bool {
    nodes.into_iter().any(|record| {
        namespace_record_visible(
            namespace,
            &record.namespace,
            &record.workspace_code,
            &record.app_code,
            &record.user_code,
            scope,
        )
    }) || edges.into_iter().any(|record| {
        namespace_record_visible(
            namespace,
            &record.namespace,
            &record.workspace_code,
            &record.app_code,
            &record.user_code,
            scope,
        )
    })
}

fn namespace_record_visible(
    namespace: &str,
    record_namespace: &str,
    workspace_code: &str,
    app_code: &str,
    user_code: &str,
    scope: &GraphExplorerNamespaceScope,
) -> bool {
    record_namespace == namespace
        && app_code == STORAGE_ABSENT_SENTINEL
        && scope
            .visible_workspace_codes
            .iter()
            .any(|code| code == workspace_code)
        && scope
            .visible_user_codes
            .iter()
            .any(|code| code == user_code)
}

fn preferred_nodes(
    records: Vec<StoredGraphNodeRecord>,
    scope: &GraphExplorerResolvedScope,
    workspace_rank: &HashMap<String, usize>,
    visible_users: &BTreeSet<String>,
) -> BTreeMap<String, GraphExplorerNode> {
    let mut selected = BTreeMap::<String, (RecordPriority, GraphExplorerNode)>::new();
    for record in records {
        if !record_visible(
            &record.namespace,
            &record.workspace_code,
            &record.app_code,
            &record.user_code,
            scope,
            visible_users,
        ) {
            continue;
        }
        let priority = record_priority(&record.workspace_code, &record.user_code, workspace_rank);
        let node = map_node(record, scope);
        selected
            .entry(node.node.node_code.clone())
            .and_modify(|(existing_priority, existing)| {
                if priority > *existing_priority {
                    *existing_priority = priority;
                    *existing = node.clone();
                }
            })
            .or_insert((priority, node));
    }

    selected
        .into_iter()
        .map(|(code, (_, node))| (code, node))
        .collect()
}

fn preferred_edges(
    records: Vec<StoredGraphEdgeRecord>,
    scope: &GraphExplorerResolvedScope,
    workspace_rank: &HashMap<String, usize>,
    visible_users: &BTreeSet<String>,
    visible_node_codes: &BTreeSet<String>,
) -> BTreeMap<String, GraphExplorerEdge> {
    let mut selected = BTreeMap::<String, (RecordPriority, GraphExplorerEdge)>::new();
    for record in records {
        if !record_visible(
            &record.namespace,
            &record.workspace_code,
            &record.app_code,
            &record.user_code,
            scope,
            visible_users,
        ) {
            continue;
        }
        if !visible_node_codes.contains(&record.edge.source_node_code)
            || !visible_node_codes.contains(&record.edge.target_node_code)
        {
            continue;
        }
        let priority = record_priority(&record.workspace_code, &record.user_code, workspace_rank);
        let edge = map_edge(record, scope);
        selected
            .entry(edge.edge.edge_code.clone())
            .and_modify(|(existing_priority, existing)| {
                if priority > *existing_priority {
                    *existing_priority = priority;
                    *existing = edge.clone();
                }
            })
            .or_insert((priority, edge));
    }

    selected
        .into_iter()
        .map(|(code, (_, edge))| (code, edge))
        .collect()
}

fn record_visible(
    namespace: &str,
    workspace_code: &str,
    app_code: &str,
    user_code: &str,
    scope: &GraphExplorerResolvedScope,
    visible_users: &BTreeSet<String>,
) -> bool {
    namespace == scope.graph.storage_dimensions.namespace
        && app_code == STORAGE_ABSENT_SENTINEL
        && scope
            .graph
            .visible_workspace_codes
            .iter()
            .any(|code| code == workspace_code)
        && visible_users.contains(user_code)
}

fn map_node(
    record: StoredGraphNodeRecord,
    scope: &GraphExplorerResolvedScope,
) -> GraphExplorerNode {
    let scope_type = scope_type(&record.user_code);
    let inherited = record.workspace_code != scope.graph.current_workspace_code;
    GraphExplorerNode {
        node: record.node,
        scope_type,
        origin_workspace_name: scope.workspace_names.get(&record.workspace_code).cloned(),
        origin_workspace_code: record.workspace_code,
        origin_user_code: (scope_type == GraphExplorerScopeType::User).then_some(record.user_code),
        inherited,
        editable: editable(scope_type, inherited),
    }
}

fn map_edge(
    record: StoredGraphEdgeRecord,
    scope: &GraphExplorerResolvedScope,
) -> GraphExplorerEdge {
    let scope_type = scope_type(&record.user_code);
    let inherited = record.workspace_code != scope.graph.current_workspace_code;
    GraphExplorerEdge {
        edge: record.edge,
        scope_type,
        origin_workspace_name: scope.workspace_names.get(&record.workspace_code).cloned(),
        origin_workspace_code: record.workspace_code,
        origin_user_code: (scope_type == GraphExplorerScopeType::User).then_some(record.user_code),
        inherited,
        editable: editable(scope_type, inherited),
    }
}

fn scope_type(user_code: &str) -> GraphExplorerScopeType {
    if user_code == STORAGE_ABSENT_SENTINEL {
        GraphExplorerScopeType::Space
    } else {
        GraphExplorerScopeType::User
    }
}

fn editable(scope_type: GraphExplorerScopeType, inherited: bool) -> bool {
    scope_type == GraphExplorerScopeType::User || !inherited
}

fn record_priority(
    workspace_code: &str,
    user_code: &str,
    workspace_rank: &HashMap<String, usize>,
) -> RecordPriority {
    RecordPriority {
        scope_rank: if user_code == STORAGE_ABSENT_SENTINEL {
            0
        } else {
            1
        },
        workspace_rank: workspace_rank
            .get(workspace_code)
            .copied()
            .unwrap_or_default(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct RecordPriority {
    scope_rank: usize,
    workspace_rank: usize,
}
