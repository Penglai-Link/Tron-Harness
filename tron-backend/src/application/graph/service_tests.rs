use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use crate::{
    application::graph::{GraphScopeResolver, GraphService, GraphStore},
    domain::graph::{
        GraphEdge, GraphEdgeWrite, GraphError, GraphFilterOperator, GraphNode, GraphNodeWrite,
        GraphPath, GraphPropertyFilter, GraphPropertyValue, GraphQueryLimit, GraphScopeCommand,
        GraphSearchHit, GraphSearchQuery, GraphStorageDimensions, GraphTraversalClause,
        GraphTraversalDirection, GraphTraverseQuery, ResolvedGraphScope,
    },
};

#[derive(Clone)]
struct FakeScopeResolver {
    scope: ResolvedGraphScope,
    calls: Arc<Mutex<Vec<GraphScopeCommand>>>,
}

impl FakeScopeResolver {
    fn new(scope: ResolvedGraphScope) -> Self {
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
enum StoreCall {
    CreateNode {
        scope: ResolvedGraphScope,
        command: GraphNodeWrite,
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
        command: GraphEdgeWrite,
    },
    Search {
        scope: ResolvedGraphScope,
        query: GraphSearchQuery,
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
struct FakeStore {
    state: Arc<Mutex<FakeStoreState>>,
}

impl FakeStore {
    fn with_nodes(nodes: &[&str]) -> Self {
        let store = Self::default();
        store
            .state
            .lock()
            .expect("store state mutex poisoned")
            .existing_nodes
            .extend(nodes.iter().map(|code| (*code).to_owned()));
        store
    }

    fn calls(&self) -> Vec<StoreCall> {
        self.state
            .lock()
            .expect("store state mutex poisoned")
            .calls
            .clone()
    }

    fn set_next_node_result(&self, result: Result<GraphNode, GraphError>) {
        self.state
            .lock()
            .expect("store state mutex poisoned")
            .next_node_result = Some(result);
    }

    fn set_next_edge_result(&self, result: Result<GraphEdge, GraphError>) {
        self.state
            .lock()
            .expect("store state mutex poisoned")
            .next_edge_result = Some(result);
    }

    fn mark_edge_missing(&self, edge_code: &str) {
        self.state
            .lock()
            .expect("store state mutex poisoned")
            .missing_edges
            .insert(edge_code.to_owned());
    }
}

impl GraphStore for FakeStore {
    fn create_node(
        &mut self,
        scope: &ResolvedGraphScope,
        command: GraphNodeWrite,
    ) -> Result<GraphNode, GraphError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::CreateNode {
            scope: scope.clone(),
            command: command.clone(),
        });
        if let Some(result) = state.next_node_result.take() {
            return result;
        }
        Ok(node(&command.node_code))
    }

    fn get_node(
        &mut self,
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

    fn get_edge(
        &mut self,
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
        &mut self,
        scope: &ResolvedGraphScope,
        command: GraphEdgeWrite,
    ) -> Result<GraphEdge, GraphError> {
        let mut state = self.state.lock().expect("store state mutex poisoned");
        state.calls.push(StoreCall::CreateEdge {
            scope: scope.clone(),
            command: command.clone(),
        });
        if let Some(result) = state.next_edge_result.take() {
            return result;
        }
        Ok(edge(&command.edge_code))
    }

    fn search(
        &mut self,
        scope: &ResolvedGraphScope,
        query: GraphSearchQuery,
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
        &mut self,
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

#[test]
fn duplicate_and_missing_store_outcomes_return_typed_graph_errors() {
    let scope = resolved_scope();
    let resolver = FakeScopeResolver::new(scope.clone());
    let store = FakeStore::with_nodes(&["NODE-A", "NODE-B"]);
    let service = GraphService::new(store.clone(), resolver);

    store.set_next_node_result(Err(GraphError::DuplicateNode {
        node_code: "NODE-A".to_owned(),
    }));
    assert_eq!(
        service.create_node(scope_command(), node_write("NODE-A")),
        Err(GraphError::DuplicateNode {
            node_code: "NODE-A".to_owned()
        })
    );

    store.set_next_edge_result(Err(GraphError::DuplicateEdge {
        edge_code: "EDGE-A".to_owned(),
    }));
    assert_eq!(
        service.create_edge(scope_command(), edge_write("EDGE-A", "NODE-A", "NODE-B")),
        Err(GraphError::DuplicateEdge {
            edge_code: "EDGE-A".to_owned()
        })
    );

    assert_eq!(
        service.get_node(scope_command(), "NODE-MISSING".to_owned()),
        Err(GraphError::MissingNode {
            node_code: "NODE-MISSING".to_owned()
        })
    );

    store.mark_edge_missing("EDGE-MISSING");
    assert_eq!(
        service.get_edge(scope_command(), "EDGE-MISSING".to_owned()),
        Err(GraphError::MissingEdge {
            edge_code: "EDGE-MISSING".to_owned()
        })
    );
}

#[test]
fn edge_writes_require_source_and_target_nodes_in_same_resolved_scope() {
    let scope = resolved_scope();
    let resolver = FakeScopeResolver::new(scope.clone());
    let store = FakeStore::with_nodes(&["NODE-A"]);
    let service = GraphService::new(store.clone(), resolver);

    assert_eq!(
        service.create_edge(scope_command(), edge_write("EDGE-A", "NODE-A", "NODE-B")),
        Err(GraphError::MissingEndpoint {
            node_code: "NODE-B".to_owned()
        })
    );
    assert_eq!(
        store.calls(),
        vec![
            StoreCall::GetNode {
                scope: scope.clone(),
                node_code: "NODE-A".to_owned(),
            },
            StoreCall::GetNode {
                scope,
                node_code: "NODE-B".to_owned(),
            },
        ]
    );
}

#[test]
fn store_port_receives_resolved_scope_and_typed_lpg_requests() {
    let scope = resolved_scope();
    let resolver = FakeScopeResolver::new(scope.clone());
    let store = FakeStore::with_nodes(&["NODE-A", "NODE-B"]);
    let service = GraphService::new(store.clone(), resolver);
    let search = search_query();
    let traversal = traversal_query();

    service
        .create_node(scope_command(), node_write(" NODE-A "))
        .expect("create node should pass");
    service
        .create_edge(scope_command(), edge_write("EDGE-A", "NODE-A", "NODE-B"))
        .expect("create edge should pass");
    service
        .search(scope_command(), search.clone())
        .expect("search should pass");
    service
        .traverse(scope_command(), traversal.clone())
        .expect("traverse should pass");

    assert_eq!(
        store.calls(),
        vec![
            StoreCall::CreateNode {
                scope: scope.clone(),
                command: node_write("NODE-A")
                    .validate()
                    .expect("node should validate"),
            },
            StoreCall::GetNode {
                scope: scope.clone(),
                node_code: "NODE-A".to_owned(),
            },
            StoreCall::GetNode {
                scope: scope.clone(),
                node_code: "NODE-B".to_owned(),
            },
            StoreCall::CreateEdge {
                scope: scope.clone(),
                command: edge_write("EDGE-A", "NODE-A", "NODE-B")
                    .validate()
                    .expect("edge should validate"),
            },
            StoreCall::Search {
                scope: scope.clone(),
                query: search.validate().expect("search should validate"),
            },
            StoreCall::Traverse {
                scope,
                query: traversal.validate().expect("traversal should validate"),
            },
        ]
    );
}

fn scope_command() -> GraphScopeCommand {
    GraphScopeCommand {
        namespace: "knowledge_graph".to_owned(),
        workspace_code: "WS-CHILD".to_owned(),
        app_code: Some("APP-A".to_owned()),
        user_code: Some("USER-A".to_owned()),
    }
}

fn resolved_scope() -> ResolvedGraphScope {
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

fn node_write(node_code: &str) -> GraphNodeWrite {
    GraphNodeWrite {
        node_code: node_code.to_owned(),
        labels: vec!["Memory".to_owned()],
        properties: BTreeMap::from([(
            "displayName".to_owned(),
            GraphPropertyValue::Text("Memory".to_owned()),
        )]),
    }
}

fn edge_write(edge_code: &str, source_node_code: &str, target_node_code: &str) -> GraphEdgeWrite {
    GraphEdgeWrite {
        edge_code: edge_code.to_owned(),
        source_node_code: source_node_code.to_owned(),
        target_node_code: target_node_code.to_owned(),
        relationship: "REFERENCES".to_owned(),
        properties: BTreeMap::new(),
    }
}

fn search_query() -> GraphSearchQuery {
    GraphSearchQuery {
        filters: vec![GraphPropertyFilter {
            property_name: "kind".to_owned(),
            operator: GraphFilterOperator::Equals,
            value: GraphPropertyValue::Text("memory".to_owned()),
        }],
        full_text: Some("release".to_owned()),
        vector_text: Some("semantic memory lookup".to_owned()),
        labels: vec!["Memory".to_owned()],
        limit: GraphQueryLimit::new(10).expect("positive limit should be valid"),
        min_score: None,
    }
}

fn traversal_query() -> GraphTraverseQuery {
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

fn node(node_code: &str) -> GraphNode {
    GraphNode {
        node_code: node_code.to_owned(),
        labels: vec!["Memory".to_owned()],
        properties: BTreeMap::new(),
    }
}

fn edge(edge_code: &str) -> GraphEdge {
    GraphEdge {
        edge_code: edge_code.to_owned(),
        source_node_code: "NODE-A".to_owned(),
        target_node_code: "NODE-B".to_owned(),
        relationship: "REFERENCES".to_owned(),
        properties: BTreeMap::new(),
    }
}
