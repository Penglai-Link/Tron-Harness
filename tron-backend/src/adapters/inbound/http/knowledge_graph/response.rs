use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::application::graph::{
    GraphExplorerEdge, GraphExplorerFacet, GraphExplorerInheritanceMode, GraphExplorerMetrics,
    GraphExplorerNode, GraphExplorerPath, GraphExplorerScope, GraphExplorerScopeMode,
    GraphExplorerScopeType, GraphExplorerSnapshot, GraphExplorerWindow, graph_property_to_json,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeGraphExplorerResponse {
    pub scope: KnowledgeGraphScopeResponse,
    pub window: KnowledgeGraphWindowResponse,
    pub metrics: KnowledgeGraphMetricsResponse,
    pub namespaces: Vec<String>,
    pub facets: Vec<GraphExplorerFacet>,
    pub nodes: Vec<KnowledgeGraphNodeResponse>,
    pub edges: Vec<KnowledgeGraphEdgeResponse>,
    pub paths: Vec<GraphExplorerPath>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeGraphScopeResponse {
    pub app_code: String,
    pub current_workspace_code: String,
    pub named_graph: String,
    pub namespace: String,
    pub root_workspace_code: String,
    pub schema: String,
    pub user_code: String,
    pub visible_workspace_codes: Vec<String>,
    pub visible_user_codes: Vec<String>,
    pub scope_modes: Vec<KnowledgeGraphScopeModeResponse>,
    pub inheritance_mode: KnowledgeGraphInheritanceModeResponse,
    pub workspace_code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeGraphWindowResponse {
    pub limit: usize,
    pub node_total: usize,
    pub edge_total: usize,
    pub returned_node_count: usize,
    pub returned_edge_count: usize,
    pub has_more: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeGraphMetricsResponse {
    pub node_count: usize,
    pub edge_count: usize,
    pub namespace_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeGraphNodeResponse {
    pub node_code: String,
    pub labels: Vec<String>,
    pub properties: BTreeMap<String, Value>,
    pub full_text: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub scope_type: KnowledgeGraphScopeTypeResponse,
    pub origin_workspace_code: String,
    pub origin_workspace_name: Option<String>,
    pub origin_user_code: Option<String>,
    pub inherited: bool,
    pub editable: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeGraphEdgeResponse {
    pub edge_code: String,
    pub source_node_code: String,
    pub target_node_code: String,
    pub relationship: String,
    pub properties: BTreeMap<String, Value>,
    pub full_text: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub scope_type: KnowledgeGraphScopeTypeResponse,
    pub origin_workspace_code: String,
    pub origin_workspace_name: Option<String>,
    pub origin_user_code: Option<String>,
    pub inherited: bool,
    pub editable: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KnowledgeGraphScopeModeResponse {
    Space,
    User,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KnowledgeGraphInheritanceModeResponse {
    WorkspaceLineage,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KnowledgeGraphScopeTypeResponse {
    Space,
    User,
}

impl From<GraphExplorerSnapshot> for KnowledgeGraphExplorerResponse {
    fn from(snapshot: GraphExplorerSnapshot) -> Self {
        Self {
            scope: KnowledgeGraphScopeResponse::from(snapshot.scope),
            window: KnowledgeGraphWindowResponse::from(snapshot.window),
            metrics: KnowledgeGraphMetricsResponse::from(snapshot.metrics),
            namespaces: snapshot.namespaces,
            facets: snapshot.facets,
            nodes: snapshot
                .nodes
                .into_iter()
                .map(KnowledgeGraphNodeResponse::from)
                .collect(),
            edges: snapshot
                .edges
                .into_iter()
                .map(KnowledgeGraphEdgeResponse::from)
                .collect(),
            paths: snapshot.paths,
        }
    }
}

impl From<GraphExplorerScope> for KnowledgeGraphScopeResponse {
    fn from(scope: GraphExplorerScope) -> Self {
        Self {
            app_code: scope.app_code,
            current_workspace_code: scope.current_workspace_code,
            named_graph: scope.named_graph,
            namespace: scope.namespace,
            root_workspace_code: scope.root_workspace_code,
            schema: scope.schema,
            user_code: scope.user_code,
            visible_workspace_codes: scope.visible_workspace_codes,
            visible_user_codes: scope.visible_user_codes,
            scope_modes: scope
                .scope_modes
                .into_iter()
                .map(KnowledgeGraphScopeModeResponse::from)
                .collect(),
            inheritance_mode: KnowledgeGraphInheritanceModeResponse::from(scope.inheritance_mode),
            workspace_code: scope.workspace_code,
        }
    }
}

impl From<GraphExplorerWindow> for KnowledgeGraphWindowResponse {
    fn from(window: GraphExplorerWindow) -> Self {
        Self {
            limit: window.limit,
            node_total: window.node_total,
            edge_total: window.edge_total,
            returned_node_count: window.returned_node_count,
            returned_edge_count: window.returned_edge_count,
            has_more: window.has_more,
        }
    }
}

impl From<GraphExplorerMetrics> for KnowledgeGraphMetricsResponse {
    fn from(metrics: GraphExplorerMetrics) -> Self {
        Self {
            node_count: metrics.node_count,
            edge_count: metrics.edge_count,
            namespace_count: metrics.namespace_count,
        }
    }
}

impl From<GraphExplorerNode> for KnowledgeGraphNodeResponse {
    fn from(node: GraphExplorerNode) -> Self {
        Self {
            node_code: node.node.node_code,
            labels: node.node.labels,
            properties: map_properties(node.node.properties),
            full_text: node.node.full_text,
            created_at: node.node.created_at,
            updated_at: node.node.updated_at,
            scope_type: KnowledgeGraphScopeTypeResponse::from(node.scope_type),
            origin_workspace_code: node.origin_workspace_code,
            origin_workspace_name: node.origin_workspace_name,
            origin_user_code: node.origin_user_code,
            inherited: node.inherited,
            editable: node.editable,
        }
    }
}

impl From<GraphExplorerEdge> for KnowledgeGraphEdgeResponse {
    fn from(edge: GraphExplorerEdge) -> Self {
        Self {
            edge_code: edge.edge.edge_code,
            source_node_code: edge.edge.source_node_code,
            target_node_code: edge.edge.target_node_code,
            relationship: edge.edge.relationship,
            properties: map_properties(edge.edge.properties),
            full_text: edge.edge.full_text,
            created_at: edge.edge.created_at,
            updated_at: edge.edge.updated_at,
            scope_type: KnowledgeGraphScopeTypeResponse::from(edge.scope_type),
            origin_workspace_code: edge.origin_workspace_code,
            origin_workspace_name: edge.origin_workspace_name,
            origin_user_code: edge.origin_user_code,
            inherited: edge.inherited,
            editable: edge.editable,
        }
    }
}

impl From<GraphExplorerScopeMode> for KnowledgeGraphScopeModeResponse {
    fn from(mode: GraphExplorerScopeMode) -> Self {
        match mode {
            GraphExplorerScopeMode::Space => Self::Space,
            GraphExplorerScopeMode::User => Self::User,
        }
    }
}

impl From<GraphExplorerInheritanceMode> for KnowledgeGraphInheritanceModeResponse {
    fn from(mode: GraphExplorerInheritanceMode) -> Self {
        match mode {
            GraphExplorerInheritanceMode::WorkspaceLineage => Self::WorkspaceLineage,
        }
    }
}

impl From<GraphExplorerScopeType> for KnowledgeGraphScopeTypeResponse {
    fn from(scope_type: GraphExplorerScopeType) -> Self {
        match scope_type {
            GraphExplorerScopeType::Space => Self::Space,
            GraphExplorerScopeType::User => Self::User,
        }
    }
}

fn map_properties(
    properties: BTreeMap<String, crate::domain::graph::GraphPropertyValue>,
) -> BTreeMap<String, Value> {
    properties
        .into_iter()
        .filter(|(key, _)| !is_embedding_vector_property(key))
        .map(|(key, value)| (key, graph_property_to_json(&value)))
        .collect()
}

fn is_embedding_vector_property(property_name: &str) -> bool {
    let normalized = property_name.to_ascii_lowercase();
    normalized == "embedding" || normalized.ends_with("embeddingjson")
}
