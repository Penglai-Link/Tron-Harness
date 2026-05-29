use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use crate::{
    application::graph::{
        GraphScopeResolver, GraphStore, PreparedGraphEdgeWrite, PreparedGraphNodeWrite,
        PreparedGraphSearchQuery,
    },
    domain::graph::{
        GraphEdge, GraphEdgeWrite, GraphError, GraphFilterOperator, GraphNode, GraphNodeWrite,
        GraphPath, GraphPropertyFilter, GraphPropertyValue, GraphQueryLimit, GraphScopeCommand,
        GraphSearchHit, GraphSearchQuery, GraphSearchTraversal, GraphStorageDimensions,
        GraphTraversalClause, GraphTraversalDirection, GraphTraverseQuery, ResolvedGraphScope,
    },
};

#[derive(Clone)]
pub(super) struct FakeScopeResolver {
    scope: ResolvedGraphScope,
    calls: Arc<Mutex<Vec<GraphScopeCommand>>>,
}

impl FakeScopeResolver {
    pub(super) fn new(scope: ResolvedGraphScope) -> Self {
        Self {
            scope,
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl GraphScopeResolver for FakeScopeResolver {
    fn resolve_graph_scope(
        &self,
        command: GraphScopeCommand,
    ) -> Result<ResolvedGraphScope, GraphError> {
        self.calls
            .lock()
            .expect("resolver calls mutex poisoned")
            .push(command);
        Ok(self.scope.clone())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum StoreCall {
    CreateNode {
        scope: ResolvedGraphScope,
        command: PreparedGraphNodeWrite,
    },
    UpdateNode {
        scope: ResolvedGraphScope,
        command: PreparedGraphNodeWrite,
    },
    DeleteNode {
        scope: ResolvedGraphScope,
        node_code: String,
    },
    GetNode {
        scope: ResolvedGraphScope,
        node_code: String,
    },
    GetEdge {
        scope: ResolvedGraphScope,
        edge_code: String,
    },
    CreateEdge {
        scope: ResolvedGraphScope,
        command: PreparedGraphEdgeWrite,
    },
    UpdateEdge {
        scope: ResolvedGraphScope,
        command: PreparedGraphEdgeWrite,
    },
    DeleteEdge {
        scope: ResolvedGraphScope,
        edge_code: String,
    },
    Search {
        scope: ResolvedGraphScope,
        query: PreparedGraphSearchQuery,
    },
    Traverse {
        scope: ResolvedGraphScope,
        query: GraphTraverseQuery,
    },
}

#[derive(Default)]
struct FakeStoreState {
    calls: Vec<StoreCall>,
    existing_nodes: BTreeSet<String>,
    next_node_result: Option<Result<GraphNode, GraphError>>,
    next_edge_result: Option<Result<GraphEdge, GraphError>>,
    missing_edges: BTreeSet<String>,
}

#[derive(Clone, Default)]
pub(super) struct FakeStore {
    state: Arc<Mutex<FakeStoreState>>,
}

impl FakeStore {
    pub(super) fn with_nodes(nodes: &[&str]) -> Self {
        let store = Self::default();
        store
            .state
            .lock()
            .expect("store state mutex poisoned")
            .existing_nodes
            .extend(nodes.iter().map(|code| (*code).to_owned()));
        store
    }

    pub(super) fn calls(&self) -> Vec<StoreCall> {
        self.state
            .lock()
            .expect("store state mutex poisoned")
            .calls
            .clone()
    }

    pub(super) fn set_next_node_result(&self, result: Result<GraphNode, GraphError>) {
        self.state
            .lock()
            .expect("store state mutex poisoned")
            .next_node_result = Some(result);
    }

    pub(super) fn set_next_edge_result(&self, result: Result<GraphEdge, GraphError>) {
        self.state
            .lock()
            .expect("store state mutex poisoned")
            .next_edge_result = Some(result);
    }

    pub(super) fn mark_edge_missing(&self, edge_code: &str) {
        self.state
            .lock()
            .expect("store state mutex poisoned")
            .missing_edges
            .insert(edge_code.to_owned());
    }
}

impl GraphStore for FakeStore {
    fn create_node(
        &self,
        scope: &ResolvedGraphScope,
        command: PreparedGraphNodeWrite,
    ) -> Result<GraphNode, GraphError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::CreateNode {
            scope: scope.clone(),
            command: command.clone(),
        });
        if let Some(result) = state.next_node_result.take() {
            return result;
        }
        Ok(node(&command.command.node_code))
    }

    fn get_node(
        &self,
        scope: &ResolvedGraphScope,
        node_code: &str,
    ) -> Result<Option<GraphNode>, GraphError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::GetNode {
            scope: scope.clone(),
            node_code: node_code.to_owned(),
        });
        Ok(state
            .existing_nodes
            .contains(node_code)
            .then(|| node(node_code)))
    }

    fn update_node(
        &self,
        scope: &ResolvedGraphScope,
        command: PreparedGraphNodeWrite,
    ) -> Result<GraphNode, GraphError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::UpdateNode {
            scope: scope.clone(),
            command: command.clone(),
        });
        if let Some(result) = state.next_node_result.take() {
            return result;
        }
        Ok(node(&command.command.node_code))
    }

    fn delete_node(&self, scope: &ResolvedGraphScope, node_code: &str) -> Result<(), GraphError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::DeleteNode {
            scope: scope.clone(),
            node_code: node_code.to_owned(),
        });
        if !state.existing_nodes.contains(node_code) {
            return Err(GraphError::MissingNode {
                node_code: node_code.to_owned(),
            });
        }
        Ok(())
    }

    fn get_edge(
        &self,
        scope: &ResolvedGraphScope,
        edge_code: &str,
    ) -> Result<Option<GraphEdge>, GraphError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::GetEdge {
            scope: scope.clone(),
            edge_code: edge_code.to_owned(),
        });
        Ok((!state.missing_edges.contains(edge_code)).then(|| edge(edge_code)))
    }

    fn create_edge(
        &self,
        scope: &ResolvedGraphScope,
        command: PreparedGraphEdgeWrite,
    ) -> Result<GraphEdge, GraphError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::CreateEdge {
            scope: scope.clone(),
            command: command.clone(),
        });
        if let Some(result) = state.next_edge_result.take() {
            return result;
        }
        Ok(edge(&command.command.edge_code))
    }

    fn update_edge(
        &self,
        scope: &ResolvedGraphScope,
        command: PreparedGraphEdgeWrite,
    ) -> Result<GraphEdge, GraphError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::UpdateEdge {
            scope: scope.clone(),
            command: command.clone(),
        });
        if let Some(result) = state.next_edge_result.take() {
            return result;
        }
        Ok(edge(&command.command.edge_code))
    }

    fn delete_edge(&self, scope: &ResolvedGraphScope, edge_code: &str) -> Result<(), GraphError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::DeleteEdge {
            scope: scope.clone(),
            edge_code: edge_code.to_owned(),
        });
        if state.missing_edges.contains(edge_code) {
            return Err(GraphError::MissingEdge {
                edge_code: edge_code.to_owned(),
            });
        }
        Ok(())
    }

    fn search(
        &self,
        scope: &ResolvedGraphScope,
        query: PreparedGraphSearchQuery,
    ) -> Result<Vec<GraphSearchHit>, GraphError> {
        self.state
            .lock()
            .expect("store state mutex poisoned")
            .calls
            .push(StoreCall::Search {
                scope: scope.clone(),
                query,
            });
        Ok(Vec::new())
    }

    fn traverse(
        &self,
        scope: &ResolvedGraphScope,
        query: GraphTraverseQuery,
    ) -> Result<Vec<GraphPath>, GraphError> {
        self.state
            .lock()
            .expect("store state mutex poisoned")
            .calls
            .push(StoreCall::Traverse {
                scope: scope.clone(),
                query,
            });
        Ok(Vec::new())
    }
}

pub(super) fn scope_command() -> GraphScopeCommand {
    GraphScopeCommand {
        namespace: "knowledge_graph".to_owned(),
        workspace_code: "WS-CHILD".to_owned(),
        app_code: Some("APP-A".to_owned()),
        user_code: Some("USER-A".to_owned()),
    }
}

pub(super) fn resolved_scope() -> ResolvedGraphScope {
    ResolvedGraphScope {
        schema: "knowledge_graph".to_owned(),
        named_graph: "WS-ROOT".to_owned(),
        root_workspace_code: "WS-ROOT".to_owned(),
        current_workspace_code: "WS-CHILD".to_owned(),
        visible_workspace_codes: vec!["WS-ROOT".to_owned(), "WS-CHILD".to_owned()],
        storage_dimensions: GraphStorageDimensions {
            namespace: "knowledge_graph".to_owned(),
            workspace_code: "WS-CHILD".to_owned(),
            app_code: "APP-A".to_owned(),
            user_code: "USER-A".to_owned(),
        },
    }
}

pub(super) fn node_write(node_code: &str) -> GraphNodeWrite {
    GraphNodeWrite {
        node_code: node_code.to_owned(),
        labels: vec!["Memory".to_owned()],
        properties: BTreeMap::from([(
            "displayName".to_owned(),
            GraphPropertyValue::Text("Memory".to_owned()),
        )]),
        full_text: Some("memory body".to_owned()),
        embedding_text: None,
        embedding_model_code: None,
    }
}

pub(super) fn semantic_node_write(node_code: &str) -> GraphNodeWrite {
    GraphNodeWrite {
        embedding_text: Some("semantic memory body".to_owned()),
        embedding_model_code: Some("TEXT_EMBED_3_SMALL".to_owned()),
        ..node_write(node_code)
    }
}

pub(super) fn edge_write(
    edge_code: &str,
    source_node_code: &str,
    target_node_code: &str,
) -> GraphEdgeWrite {
    GraphEdgeWrite {
        edge_code: edge_code.to_owned(),
        source_node_code: source_node_code.to_owned(),
        target_node_code: target_node_code.to_owned(),
        relationship: "REFERENCES".to_owned(),
        properties: BTreeMap::new(),
        full_text: Some("edge body".to_owned()),
        embedding_text: None,
        embedding_model_code: None,
    }
}

#[allow(dead_code)]
pub(super) fn semantic_edge_write(
    edge_code: &str,
    source_node_code: &str,
    target_node_code: &str,
) -> GraphEdgeWrite {
    GraphEdgeWrite {
        embedding_text: Some("semantic edge body".to_owned()),
        embedding_model_code: Some("TEXT_EMBED_3_SMALL".to_owned()),
        ..edge_write(edge_code, source_node_code, target_node_code)
    }
}

pub(super) fn search_query() -> GraphSearchQuery {
    GraphSearchQuery {
        filters: vec![GraphPropertyFilter {
            property_name: "kind".to_owned(),
            operator: GraphFilterOperator::Equals,
            value: GraphPropertyValue::Text("memory".to_owned()),
        }],
        full_text: Some("release".to_owned()),
        vector_text: None,
        embedding_model_code: None,
        traversal: Some(GraphSearchTraversal {
            start_node_codes: vec!["NODE-A".to_owned()],
            clauses: vec![GraphTraversalClause {
                direction: GraphTraversalDirection::Outgoing,
                relationships: vec!["REFERENCES".to_owned()],
                min_depth: 1,
                max_depth: 2,
            }],
        }),
        labels: vec!["Memory".to_owned()],
        limit: GraphQueryLimit::new(10).expect("positive limit should be valid"),
        min_score: None,
    }
}

pub(super) fn vector_search_query() -> GraphSearchQuery {
    GraphSearchQuery {
        vector_text: Some("semantic memory lookup".to_owned()),
        embedding_model_code: Some("TEXT_EMBED_3_SMALL".to_owned()),
        ..search_query()
    }
}

pub(super) fn traversal_query() -> GraphTraverseQuery {
    GraphTraverseQuery {
        start_node_codes: vec!["NODE-A".to_owned()],
        clauses: vec![GraphTraversalClause {
            direction: GraphTraversalDirection::Outgoing,
            relationships: vec!["REFERENCES".to_owned()],
            min_depth: 1,
            max_depth: 2,
        }],
        filters: vec![],
        limit: GraphQueryLimit::new(10).expect("positive limit should be valid"),
    }
}

pub(super) fn plain_prepared_node(command: GraphNodeWrite) -> PreparedGraphNodeWrite {
    PreparedGraphNodeWrite::plain(command.validate().expect("node should validate"))
}

pub(super) fn plain_prepared_edge(command: GraphEdgeWrite) -> PreparedGraphEdgeWrite {
    PreparedGraphEdgeWrite::plain(command.validate().expect("edge should validate"))
}

pub(super) fn plain_prepared_search(query: GraphSearchQuery) -> PreparedGraphSearchQuery {
    PreparedGraphSearchQuery::plain(query.validate().expect("search should validate"))
}

fn node(node_code: &str) -> GraphNode {
    GraphNode {
        node_code: node_code.to_owned(),
        labels: vec!["Memory".to_owned()],
        properties: BTreeMap::new(),
        full_text: None,
        embedding_text: None,
        created_at: graph_timestamp(),
        updated_at: graph_timestamp(),
    }
}

fn edge(edge_code: &str) -> GraphEdge {
    GraphEdge {
        edge_code: edge_code.to_owned(),
        source_node_code: "NODE-A".to_owned(),
        target_node_code: "NODE-B".to_owned(),
        relationship: "REFERENCES".to_owned(),
        properties: BTreeMap::new(),
        full_text: None,
        embedding_text: None,
        created_at: graph_timestamp(),
        updated_at: graph_timestamp(),
    }
}

fn graph_timestamp() -> String {
    "2026-05-29T00:00:00Z".to_owned()
}
