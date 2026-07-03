use std::{
    collections::{BTreeMap, BTreeSet, HashMap, VecDeque},
    sync::Arc,
};

use crate::{
    application::{graph::ScopeResolver, workspace::WorkspaceService},
    domain::{
        graph::{
            GraphEdge, GraphError, GraphNode, GraphPropertyValue, GraphScopeCommand,
            GraphScopeError, ResolvedGraphScope, STORAGE_ABSENT_SENTINEL,
        },
        workspace::WorkspaceStatus,
    },
};

const NO_GRAPH_NODE_LIMIT: usize = usize::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphExplorerScopeMode {
    Space,
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphExplorerInheritanceMode {
    WorkspaceLineage,
}

#[derive(Debug, Clone)]
pub struct GraphExplorerQueryCommand {
    pub workspace_code: String,
    pub namespace: String,
    pub scope_modes: Vec<GraphExplorerScopeMode>,
    pub inheritance_mode: GraphExplorerInheritanceMode,
    pub query: Option<String>,
    pub labels: Vec<String>,
    pub relationships: Vec<String>,
    pub user_code: String,
}

#[derive(Debug, Clone)]
pub struct GraphExplorerExpandCommand {
    pub workspace_code: String,
    pub namespace: String,
    pub scope_modes: Vec<GraphExplorerScopeMode>,
    pub inheritance_mode: GraphExplorerInheritanceMode,
    pub node_code: String,
    pub depth: usize,
    pub relationships: Vec<String>,
    pub limit: usize,
    pub user_code: String,
}

#[derive(Debug, Clone)]
pub struct GraphExplorerResolvedScope {
    pub graph: ResolvedGraphScope,
    pub workspace_names: HashMap<String, String>,
    pub scope_modes: Vec<GraphExplorerScopeMode>,
    pub inheritance_mode: GraphExplorerInheritanceMode,
    pub visible_user_codes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GraphExplorerNamespaceScope {
    pub root_workspace_code: String,
    pub visible_workspace_codes: Vec<String>,
    pub visible_user_codes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GraphExplorerSnapshot {
    pub scope: GraphExplorerScope,
    pub window: GraphExplorerWindow,
    pub metrics: GraphExplorerMetrics,
    pub namespaces: Vec<String>,
    pub facets: Vec<GraphExplorerFacet>,
    pub nodes: Vec<GraphExplorerNode>,
    pub edges: Vec<GraphExplorerEdge>,
    pub paths: Vec<GraphExplorerPath>,
}

#[derive(Debug, Clone)]
pub struct GraphExplorerScope {
    pub app_code: String,
    pub current_workspace_code: String,
    pub named_graph: String,
    pub namespace: String,
    pub root_workspace_code: String,
    pub schema: String,
    pub user_code: String,
    pub visible_workspace_codes: Vec<String>,
    pub visible_user_codes: Vec<String>,
    pub scope_modes: Vec<GraphExplorerScopeMode>,
    pub inheritance_mode: GraphExplorerInheritanceMode,
    pub workspace_code: String,
}

#[derive(Debug, Clone)]
pub struct GraphExplorerWindow {
    pub limit: usize,
    pub node_total: usize,
    pub edge_total: usize,
    pub returned_node_count: usize,
    pub returned_edge_count: usize,
    pub has_more: bool,
}

#[derive(Debug, Clone)]
pub struct GraphExplorerMetrics {
    pub node_count: usize,
    pub edge_count: usize,
    pub namespace_count: usize,
}

#[derive(Debug, Clone)]
pub struct GraphExplorerNode {
    pub node: GraphNode,
    pub scope_type: GraphExplorerScopeType,
    pub origin_workspace_code: String,
    pub origin_workspace_name: Option<String>,
    pub origin_user_code: Option<String>,
    pub inherited: bool,
    pub editable: bool,
}

#[derive(Debug, Clone)]
pub struct GraphExplorerEdge {
    pub edge: GraphEdge,
    pub scope_type: GraphExplorerScopeType,
    pub origin_workspace_code: String,
    pub origin_workspace_name: Option<String>,
    pub origin_user_code: Option<String>,
    pub inherited: bool,
    pub editable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphExplorerScopeType {
    Space,
    User,
}

pub type GraphExplorerFacet = serde_json::Value;
pub type GraphExplorerPath = serde_json::Value;

pub trait GraphExplorerStore: Send + Sync {
    fn list_namespaces(
        &self,
        scope: &GraphExplorerNamespaceScope,
    ) -> Result<Vec<String>, GraphError>;

    fn visible_graph(
        &self,
        scope: &GraphExplorerResolvedScope,
    ) -> Result<GraphExplorerVisibleGraph, GraphError>;
}

#[derive(Debug, Clone, Default)]
pub struct GraphExplorerVisibleGraph {
    pub nodes: Vec<GraphExplorerNode>,
    pub edges: Vec<GraphExplorerEdge>,
}

#[derive(Clone)]
pub struct GraphExplorerService {
    store: Arc<dyn GraphExplorerStore>,
    scope_resolver: ScopeResolver,
    workspaces: WorkspaceService,
}

impl GraphExplorerService {
    pub fn new(
        store: impl GraphExplorerStore + 'static,
        scope_resolver: ScopeResolver,
        workspaces: WorkspaceService,
    ) -> Self {
        Self {
            store: Arc::new(store),
            scope_resolver,
            workspaces,
        }
    }

    pub fn list_namespaces(
        &self,
        workspace_code: String,
        user_code: String,
    ) -> Result<Vec<String>, GraphError> {
        let workspace = self
            .workspaces
            .get_workspace(&workspace_code)
            .map_err(|_| GraphError::Scope(GraphScopeError::WorkspaceResolutionFailed))?;
        if workspace.status != WorkspaceStatus::Active {
            return Err(GraphError::Scope(GraphScopeError::ArchivedWorkspace));
        }

        let root_workspace_code = workspace
            .path
            .first()
            .ok_or(GraphError::Scope(GraphScopeError::EmptyWorkspaceLineage))?
            .workspace_code
            .clone();
        let visible_workspace_codes = workspace
            .path
            .iter()
            .map(|item| item.workspace_code.clone())
            .collect::<Vec<_>>();
        let scope_modes = default_scope_modes();

        self.store.list_namespaces(&GraphExplorerNamespaceScope {
            root_workspace_code,
            visible_workspace_codes,
            visible_user_codes: visible_user_codes(&scope_modes, &user_code),
        })
    }

    pub fn snapshot(
        &self,
        workspace_code: String,
        namespace: String,
        user_code: String,
    ) -> Result<GraphExplorerSnapshot, GraphError> {
        self.query(GraphExplorerQueryCommand {
            workspace_code,
            namespace,
            scope_modes: default_scope_modes(),
            inheritance_mode: GraphExplorerInheritanceMode::WorkspaceLineage,
            query: None,
            labels: Vec::new(),
            relationships: Vec::new(),
            user_code,
        })
    }

    pub fn query(
        &self,
        command: GraphExplorerQueryCommand,
    ) -> Result<GraphExplorerSnapshot, GraphError> {
        let resolved = self.resolve_scope(
            &command.workspace_code,
            &command.namespace,
            command.user_code.clone(),
            command.scope_modes,
            command.inheritance_mode,
        )?;
        let visible = self.store.visible_graph(&resolved)?;
        let nodes = visible
            .nodes
            .into_iter()
            .filter(|node| node_matches(node, command.query.as_deref(), &command.labels))
            .collect::<Vec<_>>();
        let node_codes = nodes
            .iter()
            .map(|node| node.node.node_code.clone())
            .collect::<BTreeSet<_>>();
        let relationship_filter = command
            .relationships
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let edges = visible
            .edges
            .into_iter()
            .filter(|edge| {
                relationship_filter.is_empty()
                    || relationship_filter.contains(&edge.edge.relationship)
            })
            .filter(|edge| {
                node_codes.contains(&edge.edge.source_node_code)
                    && node_codes.contains(&edge.edge.target_node_code)
            })
            .collect::<Vec<_>>();
        Ok(build_snapshot(resolved, nodes, edges, NO_GRAPH_NODE_LIMIT))
    }

    pub fn expand(
        &self,
        command: GraphExplorerExpandCommand,
    ) -> Result<GraphExplorerSnapshot, GraphError> {
        let resolved = self.resolve_scope(
            &command.workspace_code,
            &command.namespace,
            command.user_code.clone(),
            command.scope_modes,
            command.inheritance_mode,
        )?;
        let visible = self.store.visible_graph(&resolved)?;
        let relationship_filter = command
            .relationships
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let node_map = visible
            .nodes
            .into_iter()
            .map(|node| (node.node.node_code.clone(), node))
            .collect::<BTreeMap<_, _>>();
        if !node_map.contains_key(&command.node_code) {
            return Ok(build_snapshot(
                resolved,
                Vec::new(),
                Vec::new(),
                command.limit,
            ));
        }

        let mut selected_nodes = BTreeSet::from([command.node_code.clone()]);
        let mut selected_edges = BTreeSet::new();
        let mut queue = VecDeque::from([(command.node_code, 0usize)]);
        while let Some((node_code, depth)) = queue.pop_front() {
            if depth >= command.depth {
                continue;
            }
            for edge in &visible.edges {
                if !relationship_filter.is_empty()
                    && !relationship_filter.contains(&edge.edge.relationship)
                {
                    continue;
                }
                let next = if edge.edge.source_node_code == node_code {
                    Some(edge.edge.target_node_code.clone())
                } else if edge.edge.target_node_code == node_code {
                    Some(edge.edge.source_node_code.clone())
                } else {
                    None
                };
                let Some(next) = next else {
                    continue;
                };
                if !node_map.contains_key(&next) {
                    continue;
                }
                selected_edges.insert(edge.edge.edge_code.clone());
                if selected_nodes.insert(next.clone()) {
                    queue.push_back((next, depth + 1));
                }
                if selected_nodes.len() >= command.limit {
                    break;
                }
            }
            if selected_nodes.len() >= command.limit {
                break;
            }
        }

        let nodes = node_map
            .into_iter()
            .filter_map(|(code, node)| selected_nodes.contains(&code).then_some(node))
            .collect::<Vec<_>>();
        let edges = visible
            .edges
            .into_iter()
            .filter(|edge| selected_edges.contains(&edge.edge.edge_code))
            .collect::<Vec<_>>();
        Ok(build_snapshot(resolved, nodes, edges, command.limit))
    }

    fn resolve_scope(
        &self,
        workspace_code: &str,
        namespace: &str,
        user_code: String,
        scope_modes: Vec<GraphExplorerScopeMode>,
        inheritance_mode: GraphExplorerInheritanceMode,
    ) -> Result<GraphExplorerResolvedScope, GraphError> {
        let graph = self
            .scope_resolver
            .resolve(GraphScopeCommand {
                namespace: namespace.to_owned(),
                workspace_code: workspace_code.to_owned(),
                app_code: None,
                user_code: None,
            })
            .map_err(GraphError::from)?;
        let workspace = self.workspaces.get_workspace(workspace_code).map_err(|_| {
            GraphError::Scope(crate::domain::graph::GraphScopeError::WorkspaceResolutionFailed)
        })?;
        let workspace_names = workspace
            .path
            .iter()
            .map(|item| (item.workspace_code.clone(), item.name.clone()))
            .collect::<HashMap<_, _>>();
        let visible_user_codes = visible_user_codes(&scope_modes, &user_code);

        Ok(GraphExplorerResolvedScope {
            graph,
            workspace_names,
            scope_modes,
            inheritance_mode,
            visible_user_codes,
        })
    }
}

pub fn default_scope_modes() -> Vec<GraphExplorerScopeMode> {
    vec![GraphExplorerScopeMode::Space, GraphExplorerScopeMode::User]
}

pub fn graph_property_to_json(value: &GraphPropertyValue) -> serde_json::Value {
    match value {
        GraphPropertyValue::Text(value) => serde_json::Value::String(value.clone()),
        GraphPropertyValue::Integer(value) => serde_json::Value::Number((*value).into()),
        GraphPropertyValue::Decimal(value) => serde_json::Number::from_f64(*value)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        GraphPropertyValue::Boolean(value) => serde_json::Value::Bool(*value),
    }
}

fn build_snapshot(
    resolved: GraphExplorerResolvedScope,
    mut nodes: Vec<GraphExplorerNode>,
    mut edges: Vec<GraphExplorerEdge>,
    limit: usize,
) -> GraphExplorerSnapshot {
    nodes.sort_by(|left, right| left.node.node_code.cmp(&right.node.node_code));
    edges.sort_by(|left, right| left.edge.edge_code.cmp(&right.edge.edge_code));
    let node_total = nodes.len();
    let edge_total = edges.len();
    let limited = limit != NO_GRAPH_NODE_LIMIT;
    let window_limit = if limited { limit.max(1) } else { node_total };
    if limited {
        nodes.truncate(window_limit);
    }
    let returned_node_codes = nodes
        .iter()
        .map(|node| node.node.node_code.clone())
        .collect::<BTreeSet<_>>();
    edges.retain(|edge| {
        returned_node_codes.contains(&edge.edge.source_node_code)
            && returned_node_codes.contains(&edge.edge.target_node_code)
    });
    if limited {
        edges.truncate(window_limit);
    }
    let returned_node_count = nodes.len();
    let returned_edge_count = edges.len();
    let user_code = resolved
        .visible_user_codes
        .iter()
        .find(|code| code.as_str() != STORAGE_ABSENT_SENTINEL)
        .cloned()
        .unwrap_or_else(|| STORAGE_ABSENT_SENTINEL.to_owned());

    GraphExplorerSnapshot {
        scope: GraphExplorerScope {
            app_code: resolved.graph.storage_dimensions.app_code.clone(),
            current_workspace_code: resolved.graph.current_workspace_code.clone(),
            named_graph: resolved.graph.named_graph.clone(),
            namespace: resolved.graph.storage_dimensions.namespace.clone(),
            root_workspace_code: resolved.graph.root_workspace_code.clone(),
            schema: resolved.graph.schema.clone(),
            user_code,
            visible_workspace_codes: resolved.graph.visible_workspace_codes.clone(),
            visible_user_codes: resolved.visible_user_codes.clone(),
            scope_modes: resolved.scope_modes,
            inheritance_mode: resolved.inheritance_mode,
            workspace_code: resolved.graph.current_workspace_code,
        },
        window: GraphExplorerWindow {
            limit: window_limit,
            node_total,
            edge_total,
            returned_node_count,
            returned_edge_count,
            has_more: node_total > returned_node_count || edge_total > returned_edge_count,
        },
        metrics: GraphExplorerMetrics {
            node_count: node_total,
            edge_count: edge_total,
            namespace_count: 1,
        },
        namespaces: vec![resolved.graph.storage_dimensions.namespace],
        facets: Vec::new(),
        nodes,
        edges,
        paths: Vec::new(),
    }
}

fn visible_user_codes(scope_modes: &[GraphExplorerScopeMode], user_code: &str) -> Vec<String> {
    let mut values = Vec::new();
    if scope_modes.contains(&GraphExplorerScopeMode::Space) {
        values.push(STORAGE_ABSENT_SENTINEL.to_owned());
    }
    if scope_modes.contains(&GraphExplorerScopeMode::User) {
        values.push(user_code.to_owned());
    }
    values
}

fn node_matches(node: &GraphExplorerNode, query: Option<&str>, labels: &[String]) -> bool {
    let labels_match = labels.is_empty()
        || labels
            .iter()
            .any(|label| node.node.labels.iter().any(|value| value == label));
    if !labels_match {
        return false;
    }
    let Some(query) = query.map(str::trim).filter(|value| !value.is_empty()) else {
        return true;
    };
    let query = query.to_lowercase();
    node.node.node_code.to_lowercase().contains(&query)
        || node
            .node
            .properties
            .get("title")
            .is_some_and(|value| property_value_contains(value, &query))
}

fn property_value_contains(value: &GraphPropertyValue, query: &str) -> bool {
    match value {
        GraphPropertyValue::Text(value) => value.to_lowercase().contains(query),
        GraphPropertyValue::Integer(value) => value.to_string().contains(query),
        GraphPropertyValue::Decimal(value) => value.to_string().contains(query),
        GraphPropertyValue::Boolean(value) => value.to_string().contains(query),
    }
}
