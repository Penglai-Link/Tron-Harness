use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use grafeo::{GrafeoDB, Value};
use tron_backend::{
    adapters::outbound::persistence::{
        GrafeoAdapterConfig, GrafeoGraphStore, GrafeoOntologyStore, GrafeoStoreCore,
        encode_graph_name, encode_schema_name,
    },
    application::graph::{GraphService, OntologyService, ScopeResolver, WorkspaceDetailProvider},
    domain::{
        graph::{
            GraphEdgeWrite, GraphError, GraphNodeWrite, GraphPropertyValue, GraphQueryLimit,
            GraphScopeCommand, GraphSearchHit, GraphSearchQuery, GraphTraversalClause,
            GraphTraversalDirection, GraphTraverseQuery, OntologyClass, OntologyConstraint,
            OntologyDomain, OntologyDomainResource, OntologyError, OntologyProperty,
            OntologyQueryLimit, OntologyRangeResource, OntologyRangeTarget, OntologyResource,
            OntologyResourceType, OntologySearchQuery, OntologyShaclShape, OntologyShapeProperty,
        },
        workspace::{WorkspaceDetail, WorkspaceError, WorkspaceLineageItem, WorkspaceStatus},
    },
};

#[tokio::test]
async fn embedded_grafeo_adapter_bootstraps_namespaces_and_persists_required_graph_fields() {
    let storage_path = unique_graph_storage_path();
    {
        let (graph_service, _) = build_services(&storage_path);

        let sentinel_node = graph_service
            .create_node(
                root_scope(),
                node_write("ROOT_SENTINEL", "sentinel root", 1),
            )
            .await
            .expect("sentinel node should persist");
        let updated_sentinel = graph_service
            .update_node(
                root_scope(),
                node_write("ROOT_SENTINEL", "sentinel root updated", 2),
            )
            .await
            .expect("sentinel node should update");
        assert!(!sentinel_node.created_at.is_empty());
        assert_eq!(
            updated_sentinel.full_text.as_deref(),
            Some("sentinel root updated")
        );
        assert_eq!(
            updated_sentinel.properties.get("priority"),
            Some(&GraphPropertyValue::Integer(2))
        );

        graph_service
            .create_node(root_scoped_scope(), node_write("ROOT_NODE", "root task", 3))
            .await
            .expect("root scoped node should persist");
        graph_service
            .create_node(child_scope(), node_write("CHILD_NODE", "child task", 4))
            .await
            .expect("child scoped node should persist");
        graph_service
            .create_edge(
                child_scope(),
                edge_write("ROOT_TO_CHILD", "ROOT_NODE", "CHILD_NODE", "LINKS", 8.0),
            )
            .await
            .expect("edge should persist");
        let updated_edge = graph_service
            .update_edge(
                child_scope(),
                edge_write(
                    "ROOT_TO_CHILD",
                    "ROOT_NODE",
                    "CHILD_NODE",
                    "RELATES_TO",
                    9.5,
                ),
            )
            .await
            .expect("edge should update");
        assert_eq!(updated_edge.relationship, "RELATES_TO");
        assert_eq!(
            updated_edge.properties.get("weight"),
            Some(&GraphPropertyValue::Decimal(9.5))
        );

        graph_service
            .create_node(
                audit_root_scope(),
                node_write("AUDIT_NODE", "audit record", 5),
            )
            .await
            .expect("audit namespace node should persist");

        graph_service
            .create_node(root_scoped_scope(), node_write("DELETE_ME", "delete me", 6))
            .await
            .expect("temporary node should persist");
        graph_service
            .delete_node(root_scoped_scope(), "DELETE_ME".to_owned())
            .expect("temporary node should delete");
        assert_eq!(
            graph_service.get_node(root_scoped_scope(), "DELETE_ME".to_owned()),
            Err(GraphError::MissingNode {
                node_code: "DELETE_ME".to_owned(),
            })
        );
        assert_eq!(
            graph_service
                .get_edge(child_scope(), "ROOT_TO_CHILD".to_owned())
                .expect("edge should load")
                .relationship,
            "RELATES_TO"
        );
    }

    let sentinel_row = single_row(query_string_rows(
        &storage_path,
        "knowledge-graph",
        "WS-ROOT",
        "MATCH (n:TronGraphNode) WHERE n.nodeCode = 'ROOT_SENTINEL' \
         RETURN n.namespace, n.workspaceCode, n.appCode, n.userCode, n.createdAt, n.updatedAt",
    ));
    assert_eq!(sentinel_row[0], "knowledge-graph");
    assert_eq!(sentinel_row[1], "WS-ROOT");
    assert_eq!(sentinel_row[2], "__NONE__");
    assert_eq!(sentinel_row[3], "__NONE__");
    assert!(!sentinel_row[4].is_empty());
    assert!(!sentinel_row[5].is_empty());

    let root_row = single_row(query_string_rows(
        &storage_path,
        "knowledge-graph",
        "WS-ROOT",
        "MATCH (n:TronGraphNode) WHERE n.nodeCode = 'ROOT_NODE' \
         RETURN n.workspaceCode, n.appCode, n.userCode",
    ));
    assert_eq!(
        root_row,
        vec![
            "WS-ROOT".to_owned(),
            "APP-A".to_owned(),
            "USER-A".to_owned()
        ]
    );

    let child_row = single_row(query_string_rows(
        &storage_path,
        "knowledge-graph",
        "WS-ROOT",
        "MATCH (n:TronGraphNode) WHERE n.nodeCode = 'CHILD_NODE' \
         RETURN n.workspaceCode, n.appCode, n.userCode",
    ));
    assert_eq!(
        child_row,
        vec![
            "WS-CHILD".to_owned(),
            "APP-A".to_owned(),
            "USER-A".to_owned()
        ]
    );

    let edge_row = single_row(query_string_rows(
        &storage_path,
        "knowledge-graph",
        "WS-ROOT",
        "MATCH ()-[e:TRON_EDGE]->() WHERE e.edgeCode = 'ROOT_TO_CHILD' \
         RETURN e.namespace, e.workspaceCode, e.appCode, e.userCode, e.relationship, e.createdAt, e.updatedAt",
    ));
    assert_eq!(edge_row[0], "knowledge-graph");
    assert_eq!(edge_row[1], "WS-CHILD");
    assert_eq!(edge_row[2], "APP-A");
    assert_eq!(edge_row[3], "USER-A");
    assert_eq!(edge_row[4], "RELATES_TO");
    assert!(!edge_row[5].is_empty());
    assert!(!edge_row[6].is_empty());

    let graph_codes = first_column(query_string_rows(
        &storage_path,
        "knowledge-graph",
        "WS-ROOT",
        "MATCH (n:TronGraphNode) RETURN n.nodeCode ORDER BY n.nodeCode",
    ));
    assert_eq!(
        graph_codes,
        vec!["CHILD_NODE", "ROOT_NODE", "ROOT_SENTINEL"]
    );

    let audit_codes = first_column(query_string_rows(
        &storage_path,
        "audit-graph",
        "WS-ROOT",
        "MATCH (n:TronGraphNode) RETURN n.nodeCode ORDER BY n.nodeCode",
    ));
    assert_eq!(audit_codes, vec!["AUDIT_NODE"]);
}

#[tokio::test]
async fn embedded_grafeo_adapter_enforces_named_graph_and_lineage_visibility_for_graph_search_and_traversal()
 {
    let storage_path = unique_graph_storage_path();
    let (graph_service, _) = build_services(&storage_path);

    graph_service
        .create_node(
            root_scoped_scope(),
            node_write("ROOT_NODE", "root visible", 1),
        )
        .await
        .expect("root node should persist");
    graph_service
        .create_node(child_scope(), node_write("CHILD_NODE", "child visible", 2))
        .await
        .expect("child node should persist");
    graph_service
        .create_node(
            sibling_scope(),
            node_write("SIBLING_NODE", "sibling hidden", 3),
        )
        .await
        .expect("sibling node should persist");
    graph_service
        .create_node(
            other_root_child_scope(),
            node_write("OTHER_NODE", "other root hidden", 4),
        )
        .await
        .expect("other-root node should persist");

    graph_service
        .create_edge(
            child_scope(),
            edge_write("ROOT_TO_CHILD", "ROOT_NODE", "CHILD_NODE", "LINKS", 1.0),
        )
        .await
        .expect("child edge should persist");
    graph_service
        .create_edge(
            sibling_scope(),
            edge_write("ROOT_TO_SIBLING", "ROOT_NODE", "SIBLING_NODE", "LINKS", 2.0),
        )
        .await
        .expect("sibling edge should persist");

    let child_hits = graph_service
        .search(child_scope(), label_search_query())
        .await
        .expect("child search should work");
    assert_eq!(
        graph_hit_codes(&child_hits),
        vec!["CHILD_NODE", "ROOT_NODE"]
    );

    let root_hits = graph_service
        .search(root_scoped_scope(), label_search_query())
        .await
        .expect("root search should work");
    assert_eq!(graph_hit_codes(&root_hits), vec!["ROOT_NODE"]);

    let sibling_hits = graph_service
        .search(sibling_scope(), label_search_query())
        .await
        .expect("sibling search should work");
    assert_eq!(
        graph_hit_codes(&sibling_hits),
        vec!["ROOT_NODE", "SIBLING_NODE"]
    );

    let other_hits = graph_service
        .search(other_root_child_scope(), label_search_query())
        .await
        .expect("other-root search should work");
    assert_eq!(graph_hit_codes(&other_hits), vec!["OTHER_NODE"]);

    let child_paths = graph_service
        .traverse(child_scope(), outgoing_links_from("ROOT_NODE"))
        .expect("child traversal should work");
    assert_eq!(child_paths.len(), 1);
    assert_eq!(
        child_paths[0]
            .nodes
            .last()
            .map(|node| node.node_code.as_str()),
        Some("CHILD_NODE")
    );

    assert_eq!(
        graph_service.get_node(child_scope(), "SIBLING_NODE".to_owned()),
        Err(GraphError::MissingNode {
            node_code: "SIBLING_NODE".to_owned(),
        })
    );
    assert_eq!(
        graph_service.get_node(child_scope(), "OTHER_NODE".to_owned()),
        Err(GraphError::MissingNode {
            node_code: "OTHER_NODE".to_owned(),
        })
    );
}

#[tokio::test]
async fn embedded_grafeo_adapter_persists_all_typed_ontology_resources_with_scope_rules() {
    let storage_path = unique_graph_storage_path();
    let (_, ontology_service) = build_services(&storage_path);

    let updated_class = ontology_service
        .create_class(root_scoped_scope(), task_class(None))
        .and_then(|_| {
            ontology_service
                .update_class(root_scoped_scope(), task_class(Some("Updated task class")))
        })
        .expect("root class should persist and update");
    assert_eq!(
        updated_class.description.as_deref(),
        Some("Updated task class")
    );

    let domain = ontology_service
        .create_domain(child_scope(), task_domain_resource())
        .expect("domain resource should persist");
    let range = ontology_service
        .create_range(child_scope(), task_range_resource())
        .expect("range resource should persist");
    let property = ontology_service
        .create_property(child_scope(), task_property())
        .expect("property resource should persist");
    let constraint = ontology_service
        .create_constraint(child_scope(), task_constraint())
        .expect("constraint resource should persist");
    let shape = ontology_service
        .create_shape(child_scope(), task_shape())
        .expect("shape resource should persist");

    ontology_service
        .create_class(
            sibling_scope(),
            named_class("SIBLING_ONLY", "Sibling Only", None),
        )
        .expect("sibling class should persist");
    ontology_service
        .create_class(
            other_root_child_scope(),
            named_class("OTHER_ONLY", "Other Only", None),
        )
        .expect("other-root class should persist");

    assert_eq!(
        ontology_service
            .get_domain(child_scope(), domain.domain_code.clone())
            .expect("domain should load"),
        domain
    );
    assert_eq!(
        ontology_service
            .get_range(child_scope(), range.range_code.clone())
            .expect("range should load"),
        range
    );
    assert_eq!(
        ontology_service
            .get_property(child_scope(), property.property_code.clone())
            .expect("property should load"),
        property
    );
    assert_eq!(
        ontology_service
            .get_constraint(child_scope(), constraint.constraint_code.clone())
            .expect("constraint should load"),
        constraint
    );
    assert_eq!(
        ontology_service
            .get_shape(child_scope(), shape.shape_code.clone())
            .expect("shape should load"),
        shape
    );

    let child_resources = ontology_service
        .search(child_scope(), ontology_all_resources_query())
        .expect("child ontology search should work");
    assert_eq!(
        ontology_codes(&child_resources),
        vec![
            "TASK",
            "TASK_CONSTRAINT",
            "TASK_DOMAIN",
            "TASK_PROPERTY",
            "TASK_RANGE",
            "TASK_SHAPE",
        ]
    );

    let root_resources = ontology_service
        .search(root_scoped_scope(), ontology_all_resources_query())
        .expect("root ontology search should work");
    assert_eq!(ontology_codes(&root_resources), vec!["TASK"]);

    let target_resources = ontology_service
        .search(child_scope(), ontology_target_class_query())
        .expect("target-class ontology search should work");
    assert_eq!(
        ontology_codes(&target_resources),
        vec!["TASK_DOMAIN", "TASK_RANGE", "TASK_SHAPE"]
    );

    assert_eq!(
        ontology_service.get_class(child_scope(), "OTHER_ONLY".to_owned()),
        Err(OntologyError::MissingOntologyResource {
            resource_type: "class",
            resource_code: "OTHER_ONLY".to_owned(),
        })
    );

    ontology_service
        .delete_class(sibling_scope(), "SIBLING_ONLY".to_owned())
        .expect("sibling class should delete");
    assert_eq!(
        ontology_service.get_class(sibling_scope(), "SIBLING_ONLY".to_owned()),
        Err(OntologyError::MissingOntologyResource {
            resource_type: "class",
            resource_code: "SIBLING_ONLY".to_owned(),
        })
    );
}

#[tokio::test]
async fn embedded_grafeo_adapter_maps_typed_store_errors_and_capability_gaps() {
    let storage_path = unique_graph_storage_path();
    let (graph_service, ontology_service) = build_services(&storage_path);

    graph_service
        .create_node(child_scope(), node_write("DUP_NODE", "duplicate", 1))
        .await
        .expect("duplicate test node should persist");
    assert_eq!(
        graph_service
            .create_node(child_scope(), node_write("DUP_NODE", "duplicate", 1))
            .await,
        Err(GraphError::DuplicateNode {
            node_code: "DUP_NODE".to_owned(),
        })
    );
    assert_eq!(
        graph_service.get_edge(child_scope(), "MISSING_EDGE".to_owned()),
        Err(GraphError::MissingEdge {
            edge_code: "MISSING_EDGE".to_owned(),
        })
    );
    ontology_service
        .create_class(
            child_scope(),
            named_class("DUP_CLASS", "Duplicate Class", None),
        )
        .expect("duplicate test class should persist");
    assert_eq!(
        ontology_service.create_class(
            child_scope(),
            named_class("DUP_CLASS", "Duplicate Class", None)
        ),
        Err(OntologyError::DuplicateOntologyResource {
            resource_type: "class",
            resource_code: "DUP_CLASS".to_owned(),
        })
    );
    assert_eq!(
        ontology_service.get_property(child_scope(), "MISSING_PROPERTY".to_owned()),
        Err(OntologyError::MissingOntologyResource {
            resource_type: "property",
            resource_code: "MISSING_PROPERTY".to_owned(),
        })
    );
}

#[derive(Debug, Clone, Copy)]
struct FakeWorkspaceProvider;

impl WorkspaceDetailProvider for FakeWorkspaceProvider {
    fn get_workspace(&self, workspace_code: &str) -> Result<WorkspaceDetail, WorkspaceError> {
        workspace_detail(workspace_code).ok_or(WorkspaceError::NotFound)
    }
}

fn build_services(
    storage_path: &Path,
) -> (
    GraphService<ScopeResolver<FakeWorkspaceProvider>>,
    OntologyService<ScopeResolver<FakeWorkspaceProvider>>,
) {
    let core = GrafeoStoreCore::connect(GrafeoAdapterConfig {
        storage_path: storage_path.display().to_string(),
    })
    .expect("Grafeo adapter should initialize");
    let resolver = ScopeResolver::with_workspace_provider(FakeWorkspaceProvider);
    (
        GraphService::new(GrafeoGraphStore::new(core.clone()), resolver.clone()),
        OntologyService::new(GrafeoOntologyStore::new(core), resolver),
    )
}

fn workspace_detail(workspace_code: &str) -> Option<WorkspaceDetail> {
    let (parent_workspace_code, depth, path) = match workspace_code {
        "WS-ROOT" => (None, 0, vec![lineage_item("WS-ROOT", "Root", 0)]),
        "WS-CHILD" => (
            Some("WS-ROOT".to_owned()),
            1,
            vec![
                lineage_item("WS-ROOT", "Root", 0),
                lineage_item("WS-CHILD", "Child", 1),
            ],
        ),
        "WS-SIBLING" => (
            Some("WS-ROOT".to_owned()),
            1,
            vec![
                lineage_item("WS-ROOT", "Root", 0),
                lineage_item("WS-SIBLING", "Sibling", 1),
            ],
        ),
        "WS-OTHER-ROOT" => (
            None,
            0,
            vec![lineage_item("WS-OTHER-ROOT", "Other Root", 0)],
        ),
        "WS-OTHER-CHILD" => (
            Some("WS-OTHER-ROOT".to_owned()),
            1,
            vec![
                lineage_item("WS-OTHER-ROOT", "Other Root", 0),
                lineage_item("WS-OTHER-CHILD", "Other Child", 1),
            ],
        ),
        _ => return None,
    };

    Some(WorkspaceDetail {
        id: format!("id-{workspace_code}"),
        workspace_code: workspace_code.to_owned(),
        name: workspace_code.to_owned(),
        description: None,
        status: WorkspaceStatus::Active,
        parent_workspace_code,
        depth,
        created_at: "2026-05-30T00:00:00Z".to_owned(),
        updated_at: "2026-05-30T00:00:00Z".to_owned(),
        archived_at: None,
        path,
        children: Vec::new(),
    })
}

fn lineage_item(workspace_code: &str, name: &str, depth: usize) -> WorkspaceLineageItem {
    WorkspaceLineageItem {
        workspace_code: workspace_code.to_owned(),
        name: name.to_owned(),
        depth,
    }
}

fn root_scope() -> GraphScopeCommand {
    GraphScopeCommand {
        namespace: "knowledge-graph".to_owned(),
        workspace_code: "WS-ROOT".to_owned(),
        app_code: None,
        user_code: None,
    }
}

fn root_scoped_scope() -> GraphScopeCommand {
    scoped_graph_command("knowledge-graph", "WS-ROOT")
}

fn child_scope() -> GraphScopeCommand {
    scoped_graph_command("knowledge-graph", "WS-CHILD")
}

fn sibling_scope() -> GraphScopeCommand {
    scoped_graph_command("knowledge-graph", "WS-SIBLING")
}

fn other_root_child_scope() -> GraphScopeCommand {
    scoped_graph_command("knowledge-graph", "WS-OTHER-CHILD")
}

fn audit_root_scope() -> GraphScopeCommand {
    GraphScopeCommand {
        namespace: "audit-graph".to_owned(),
        workspace_code: "WS-ROOT".to_owned(),
        app_code: None,
        user_code: None,
    }
}

fn scoped_graph_command(namespace: &str, workspace_code: &str) -> GraphScopeCommand {
    GraphScopeCommand {
        namespace: namespace.to_owned(),
        workspace_code: workspace_code.to_owned(),
        app_code: Some("APP-A".to_owned()),
        user_code: Some("USER-A".to_owned()),
    }
}

fn node_write(node_code: &str, full_text: &str, priority: i64) -> GraphNodeWrite {
    GraphNodeWrite {
        node_code: node_code.to_owned(),
        labels: vec!["Task".to_owned()],
        properties: BTreeMap::from([(
            "priority".to_owned(),
            GraphPropertyValue::Integer(priority),
        )]),
        full_text: Some(full_text.to_owned()),
        embedding_text: None,
        embedding_model_code: None,
    }
}

fn edge_write(
    edge_code: &str,
    source_node_code: &str,
    target_node_code: &str,
    relationship: &str,
    weight: f64,
) -> GraphEdgeWrite {
    GraphEdgeWrite {
        edge_code: edge_code.to_owned(),
        source_node_code: source_node_code.to_owned(),
        target_node_code: target_node_code.to_owned(),
        relationship: relationship.to_owned(),
        properties: BTreeMap::from([("weight".to_owned(), GraphPropertyValue::Decimal(weight))]),
        full_text: None,
        embedding_text: None,
        embedding_model_code: None,
    }
}

fn label_search_query() -> GraphSearchQuery {
    GraphSearchQuery {
        filters: Vec::new(),
        full_text: None,
        vector_text: None,
        embedding_model_code: None,
        traversal: None,
        labels: vec!["Task".to_owned()],
        limit: GraphQueryLimit::new(10).expect("limit"),
        min_score: None,
    }
}

fn outgoing_links_from(start_node_code: &str) -> GraphTraverseQuery {
    GraphTraverseQuery {
        start_node_codes: vec![start_node_code.to_owned()],
        clauses: vec![GraphTraversalClause {
            direction: GraphTraversalDirection::Outgoing,
            relationships: vec!["LINKS".to_owned()],
            min_depth: 1,
            max_depth: 1,
        }],
        filters: Vec::new(),
        limit: GraphQueryLimit::new(10).expect("limit"),
    }
}

fn task_class(description: Option<&str>) -> OntologyClass {
    named_class("TASK", "Task", description)
}

fn named_class(class_code: &str, label: &str, description: Option<&str>) -> OntologyClass {
    OntologyClass {
        class_code: class_code.to_owned(),
        ontology_domain: OntologyDomain::Development,
        label: label.to_owned(),
        description: description.map(str::to_owned),
        parent_class_codes: Vec::new(),
    }
}

fn task_domain_resource() -> OntologyDomainResource {
    OntologyDomainResource {
        domain_code: "TASK_DOMAIN".to_owned(),
        ontology_domain: OntologyDomain::Development,
        label: "Task Domain".to_owned(),
        description: Some("Domain for task resources".to_owned()),
        class_code: "TASK".to_owned(),
    }
}

fn task_range_resource() -> OntologyRangeResource {
    OntologyRangeResource {
        range_code: "TASK_RANGE".to_owned(),
        ontology_domain: OntologyDomain::Development,
        label: "Task Range".to_owned(),
        description: Some("Range for related tasks".to_owned()),
        target: OntologyRangeTarget::Class {
            class_code: "TASK".to_owned(),
        },
    }
}

fn task_property() -> OntologyProperty {
    OntologyProperty {
        property_code: "TASK_PROPERTY".to_owned(),
        ontology_domain: OntologyDomain::Development,
        label: "Task Property".to_owned(),
        description: Some("Links task resources".to_owned()),
        domain_codes: vec!["TASK_DOMAIN".to_owned()],
        range_codes: vec!["TASK_RANGE".to_owned()],
    }
}

fn task_constraint() -> OntologyConstraint {
    OntologyConstraint {
        constraint_code: "TASK_CONSTRAINT".to_owned(),
        ontology_domain: OntologyDomain::Development,
        label: "Task Constraint".to_owned(),
        description: Some("Requires linked task resources".to_owned()),
        required: true,
        min_count: Some(1),
        max_count: Some(1),
        allowed_range_codes: vec!["TASK_RANGE".to_owned()],
        pattern: None,
    }
}

fn task_shape() -> OntologyShaclShape {
    OntologyShaclShape {
        shape_code: "TASK_SHAPE".to_owned(),
        ontology_domain: OntologyDomain::Development,
        label: "Task Shape".to_owned(),
        description: Some("Shape for task resources".to_owned()),
        target_class_code: "TASK".to_owned(),
        property_constraints: vec![OntologyShapeProperty {
            property_code: "TASK_PROPERTY".to_owned(),
            constraint_codes: vec!["TASK_CONSTRAINT".to_owned()],
        }],
    }
}

fn ontology_all_resources_query() -> OntologySearchQuery {
    OntologySearchQuery {
        resource_types: vec![
            OntologyResourceType::Class,
            OntologyResourceType::Constraint,
            OntologyResourceType::Domain,
            OntologyResourceType::Property,
            OntologyResourceType::Range,
            OntologyResourceType::Shape,
        ],
        ontology_domains: vec![OntologyDomain::Development],
        codes: Vec::new(),
        label: None,
        target_class_code: None,
        limit: OntologyQueryLimit::new(20).expect("limit"),
    }
}

fn ontology_target_class_query() -> OntologySearchQuery {
    OntologySearchQuery {
        resource_types: vec![
            OntologyResourceType::Domain,
            OntologyResourceType::Range,
            OntologyResourceType::Shape,
        ],
        ontology_domains: vec![OntologyDomain::Development],
        codes: Vec::new(),
        label: None,
        target_class_code: Some("TASK".to_owned()),
        limit: OntologyQueryLimit::new(10).expect("limit"),
    }
}

fn graph_hit_codes(hits: &[GraphSearchHit]) -> Vec<&str> {
    let mut codes = hits
        .iter()
        .map(|hit| hit.node.node_code.as_str())
        .collect::<Vec<_>>();
    codes.sort_unstable();
    codes
}

fn ontology_codes(resources: &[OntologyResource]) -> Vec<&str> {
    let mut codes = resources
        .iter()
        .map(|resource| match resource {
            OntologyResource::Class(class) => class.class_code.as_str(),
            OntologyResource::Property(property) => property.property_code.as_str(),
            OntologyResource::Domain(domain) => domain.domain_code.as_str(),
            OntologyResource::Range(range) => range.range_code.as_str(),
            OntologyResource::Constraint(constraint) => constraint.constraint_code.as_str(),
            OntologyResource::Shape(shape) => shape.shape_code.as_str(),
        })
        .collect::<Vec<_>>();
    codes.sort_unstable();
    codes
}

fn query_string_rows(
    storage_path: &Path,
    namespace: &str,
    named_graph: &str,
    query: &str,
) -> Vec<Vec<String>> {
    let db = GrafeoDB::open(storage_path).expect("persistent Grafeo database should reopen");
    let session = db.session();
    session
        .execute(&format!(
            "SESSION SET SCHEMA {}",
            encode_schema_name(namespace)
        ))
        .expect("schema should bind");
    session
        .execute(&format!("USE GRAPH {}", encode_graph_name(named_graph)))
        .expect("named graph should bind");

    let result = session
        .execute(query)
        .expect("graph contract query should succeed");
    result
        .rows()
        .iter()
        .map(|row| row.iter().map(required_string).collect::<Vec<_>>())
        .collect::<Vec<_>>()
}

fn required_string(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| panic!("expected string value, got {value:?}"))
}

fn single_row(mut rows: Vec<Vec<String>>) -> Vec<String> {
    assert_eq!(rows.len(), 1, "expected exactly one row, got {rows:?}");
    rows.remove(0)
}

fn first_column(rows: Vec<Vec<String>>) -> Vec<String> {
    rows.into_iter()
        .map(|row| row[0].clone())
        .collect::<Vec<_>>()
}

fn unique_graph_storage_path() -> PathBuf {
    static UNIQUE_GRAPH_STORAGE_COUNTER: AtomicU64 = AtomicU64::new(0);

    let counter = UNIQUE_GRAPH_STORAGE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "tron-grafeo-contract-{}-{suffix}-{counter}.grafeo",
        std::process::id()
    ))
}
