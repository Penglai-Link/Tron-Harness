use std::collections::BTreeMap;

use crate::{
    application::graph::{
        GraphService,
        service_test_support::{
            FakeScopeResolver, FakeStore, StoreCall, edge_write, node_write, resolved_scope,
            scope_command, search_query, traversal_query,
        },
    },
    domain::graph::{GraphEdgeWrite, GraphError, GraphNodeWrite, GraphPropertyValue},
};

#[test]
fn duplicate_and_missing_store_outcomes_return_typed_graph_errors() {
    let scope = resolved_scope();
    let resolver = FakeScopeResolver::new(scope);
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
    store.set_next_node_result(Err(GraphError::MissingNode {
        node_code: "NODE-MISSING".to_owned(),
    }));
    assert_eq!(
        service.update_node(scope_command(), node_write("NODE-MISSING")),
        Err(GraphError::MissingNode {
            node_code: "NODE-MISSING".to_owned()
        })
    );
    assert_eq!(
        service.delete_node(scope_command(), "NODE-MISSING".to_owned()),
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
    store.set_next_edge_result(Err(GraphError::MissingEdge {
        edge_code: "EDGE-MISSING".to_owned(),
    }));
    assert_eq!(
        service.update_edge(
            scope_command(),
            edge_write("EDGE-MISSING", "NODE-A", "NODE-B"),
        ),
        Err(GraphError::MissingEdge {
            edge_code: "EDGE-MISSING".to_owned()
        })
    );
    assert_eq!(
        service.delete_edge(scope_command(), "EDGE-MISSING".to_owned()),
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
fn crud_methods_validate_commands_before_store_calls() {
    let resolver = FakeScopeResolver::new(resolved_scope());
    let store = FakeStore::with_nodes(&["NODE-A"]);
    let service = GraphService::new(store.clone(), resolver);

    assert_eq!(
        service.update_node(
            scope_command(),
            GraphNodeWrite {
                node_code: "NODE-A".to_owned(),
                labels: vec!["Memory".to_owned()],
                properties: BTreeMap::from([(
                    "workspaceCode".to_owned(),
                    GraphPropertyValue::Text("WS-ROOT".to_owned()),
                )]),
                full_text: None,
                embedding_text: None,
            },
        ),
        Err(GraphError::ReservedPropertyName)
    );
    assert_eq!(
        service.delete_node(scope_command(), "bad code".to_owned()),
        Err(GraphError::InvalidGraphCode)
    );
    assert_eq!(
        service.update_edge(
            scope_command(),
            GraphEdgeWrite {
                edge_code: "EDGE-A".to_owned(),
                source_node_code: "NODE-A".to_owned(),
                target_node_code: "NODE-B".to_owned(),
                relationship: " ".to_owned(),
                properties: BTreeMap::new(),
                full_text: None,
                embedding_text: None,
            },
        ),
        Err(GraphError::RelationshipRequired)
    );
    assert_eq!(
        service.delete_edge(scope_command(), "bad code".to_owned()),
        Err(GraphError::InvalidGraphCode)
    );
    assert_eq!(store.calls(), Vec::new());
}

#[test]
fn store_port_receives_resolved_scope_and_typed_lpg_requests() {
    let scope = resolved_scope();
    let resolver = FakeScopeResolver::new(scope.clone());
    let store = FakeStore::with_nodes(&["NODE-A", "NODE-B"]);
    let service = GraphService::new(store.clone(), resolver);
    let search = search_query();
    let traversal = traversal_query();

    let created_node = service
        .create_node(scope_command(), node_write(" NODE-A "))
        .expect("create node should pass");
    let updated_node = service
        .update_node(scope_command(), node_write("NODE-A"))
        .expect("update node should pass");
    service
        .delete_node(scope_command(), "NODE-A".to_owned())
        .expect("delete node should pass");
    let created_edge = service
        .create_edge(scope_command(), edge_write("EDGE-A", "NODE-A", "NODE-B"))
        .expect("create edge should pass");
    let updated_edge = service
        .update_edge(scope_command(), edge_write("EDGE-A", "NODE-A", "NODE-B"))
        .expect("update edge should pass");
    service
        .delete_edge(scope_command(), "EDGE-A".to_owned())
        .expect("delete edge should pass");
    service
        .search(scope_command(), search.clone())
        .expect("search should pass");
    service
        .traverse(scope_command(), traversal.clone())
        .expect("traverse should pass");

    assert_eq!(created_node.created_at, "2026-05-29T00:00:00Z");
    assert_eq!(updated_node.updated_at, "2026-05-29T00:00:00Z");
    assert_eq!(created_edge.created_at, "2026-05-29T00:00:00Z");
    assert_eq!(updated_edge.updated_at, "2026-05-29T00:00:00Z");

    assert_eq!(
        store.calls(),
        vec![
            StoreCall::CreateNode {
                scope: scope.clone(),
                command: node_write("NODE-A")
                    .validate()
                    .expect("node should validate"),
            },
            StoreCall::UpdateNode {
                scope: scope.clone(),
                command: node_write("NODE-A")
                    .validate()
                    .expect("node should validate"),
            },
            StoreCall::DeleteNode {
                scope: scope.clone(),
                node_code: "NODE-A".to_owned(),
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
            StoreCall::GetNode {
                scope: scope.clone(),
                node_code: "NODE-A".to_owned(),
            },
            StoreCall::GetNode {
                scope: scope.clone(),
                node_code: "NODE-B".to_owned(),
            },
            StoreCall::UpdateEdge {
                scope: scope.clone(),
                command: edge_write("EDGE-A", "NODE-A", "NODE-B")
                    .validate()
                    .expect("edge should validate"),
            },
            StoreCall::DeleteEdge {
                scope: scope.clone(),
                edge_code: "EDGE-A".to_owned(),
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
