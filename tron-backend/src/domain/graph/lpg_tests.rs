use std::collections::BTreeMap;

use crate::domain::graph::{
    GraphEdgeWrite, GraphError, GraphFilterOperator, GraphNodeWrite, GraphPropertyFilter,
    GraphPropertyValue, GraphQueryLimit, GraphSearchQuery, GraphSearchTraversal,
    GraphTraversalClause, GraphTraversalDirection, GraphTraverseQuery,
};

#[test]
fn node_and_edge_write_commands_validate_codes_labels_relationships_and_properties() {
    let node = GraphNodeWrite {
        node_code: " NODE-A ".to_owned(),
        labels: vec![
            " Memory ".to_owned(),
            "Agent".to_owned(),
            "Memory".to_owned(),
        ],
        properties: BTreeMap::from([
            (
                "displayName".to_owned(),
                GraphPropertyValue::Text("memory node".to_owned()),
            ),
            ("priority".to_owned(), GraphPropertyValue::Integer(10)),
            ("active".to_owned(), GraphPropertyValue::Boolean(true)),
        ]),
        full_text: Some("  release note text ".to_owned()),
        embedding_text: Some(" semantic release note ".to_owned()),
        embedding_model_code: Some(" TEXT_EMBED_3_SMALL ".to_owned()),
    }
    .validate()
    .expect("valid node write should normalize");

    assert_eq!(node.node_code, "NODE-A");
    assert_eq!(node.labels, vec!["Memory", "Agent"]);
    assert_eq!(
        node.properties.get("displayName"),
        Some(&GraphPropertyValue::Text("memory node".to_owned()))
    );
    assert_eq!(node.full_text.as_deref(), Some("release note text"));
    assert_eq!(
        node.embedding_text.as_deref(),
        Some("semantic release note")
    );
    assert_eq!(
        node.embedding_model_code.as_deref(),
        Some("TEXT_EMBED_3_SMALL")
    );

    let edge = GraphEdgeWrite {
        edge_code: " EDGE-A ".to_owned(),
        source_node_code: " NODE-A ".to_owned(),
        target_node_code: " NODE-B ".to_owned(),
        relationship: " REFERENCES ".to_owned(),
        properties: BTreeMap::from([("confidence".to_owned(), GraphPropertyValue::Decimal(0.93))]),
        full_text: Some("  relationship context ".to_owned()),
        embedding_text: Some(" semantic relationship ".to_owned()),
        embedding_model_code: Some(" TEXT_EMBED_3_SMALL ".to_owned()),
    }
    .validate()
    .expect("valid edge write should normalize");

    assert_eq!(edge.edge_code, "EDGE-A");
    assert_eq!(edge.source_node_code, "NODE-A");
    assert_eq!(edge.target_node_code, "NODE-B");
    assert_eq!(edge.relationship, "REFERENCES");
    assert_eq!(edge.full_text.as_deref(), Some("relationship context"));
    assert_eq!(
        edge.embedding_text.as_deref(),
        Some("semantic relationship")
    );
    assert_eq!(
        edge.embedding_model_code.as_deref(),
        Some("TEXT_EMBED_3_SMALL")
    );

    assert_eq!(
        GraphNodeWrite {
            node_code: "NODE-A".to_owned(),
            labels: vec![" ".to_owned()],
            properties: BTreeMap::new(),
            full_text: None,
            embedding_text: None,
            embedding_model_code: None,
        }
        .validate(),
        Err(GraphError::LabelRequired)
    );
    assert_eq!(
        GraphEdgeWrite {
            edge_code: "EDGE-A".to_owned(),
            source_node_code: "NODE-A".to_owned(),
            target_node_code: "NODE-B".to_owned(),
            relationship: " ".to_owned(),
            properties: BTreeMap::new(),
            full_text: None,
            embedding_text: None,
            embedding_model_code: None,
        }
        .validate(),
        Err(GraphError::RelationshipRequired)
    );
    assert_eq!(
        GraphNodeWrite {
            node_code: "bad code".to_owned(),
            labels: vec!["Memory".to_owned()],
            properties: BTreeMap::new(),
            full_text: None,
            embedding_text: None,
            embedding_model_code: None,
        }
        .validate(),
        Err(GraphError::InvalidGraphCode)
    );
    assert_eq!(
        GraphNodeWrite {
            node_code: "NODE-A".to_owned(),
            labels: vec!["Memory".to_owned()],
            properties: BTreeMap::from([(
                "workspaceCode".to_owned(),
                GraphPropertyValue::Text("WS-ROOT".to_owned()),
            )]),
            full_text: None,
            embedding_text: None,
            embedding_model_code: None,
        }
        .validate(),
        Err(GraphError::ReservedPropertyName)
    );
}

#[test]
fn search_and_traversal_contracts_are_structured_and_typed() {
    let query = GraphSearchQuery {
        filters: vec![GraphPropertyFilter {
            property_name: "kind".to_owned(),
            operator: GraphFilterOperator::Equals,
            value: GraphPropertyValue::Text("decision".to_owned()),
        }],
        full_text: Some("  release checklist ".to_owned()),
        vector_text: Some("semantic description of release risk".to_owned()),
        embedding_model_code: Some(" TEXT_EMBED_3_SMALL ".to_owned()),
        traversal: Some(GraphSearchTraversal {
            start_node_codes: vec![" NODE-A ".to_owned()],
            clauses: vec![GraphTraversalClause {
                direction: GraphTraversalDirection::Outgoing,
                relationships: vec!["DEPENDS_ON".to_owned()],
                min_depth: 1,
                max_depth: 2,
            }],
        }),
        labels: vec!["Memory".to_owned(), "Decision".to_owned()],
        limit: GraphQueryLimit::new(25).expect("positive limit should be valid"),
        min_score: Some(0.72),
    }
    .validate()
    .expect("hybrid search query should validate");

    assert_eq!(query.full_text.as_deref(), Some("release checklist"));
    assert_eq!(
        query.vector_text.as_deref(),
        Some("semantic description of release risk")
    );
    assert_eq!(
        query.embedding_model_code.as_deref(),
        Some("TEXT_EMBED_3_SMALL")
    );
    assert_eq!(
        query
            .traversal
            .as_ref()
            .expect("hybrid search should carry traversal")
            .start_node_codes,
        vec!["NODE-A"]
    );
    assert_eq!(query.labels, vec!["Memory", "Decision"]);
    assert_eq!(query.limit.value(), 25);
    assert_eq!(query.min_score, Some(0.72));

    assert_eq!(
        GraphSearchQuery::empty().validate(),
        Err(GraphError::EmptySearchQuery)
    );

    let traversal = GraphTraverseQuery {
        start_node_codes: vec![" NODE-A ".to_owned()],
        clauses: vec![GraphTraversalClause {
            direction: GraphTraversalDirection::Outgoing,
            relationships: vec!["DEPENDS_ON".to_owned(), "REFERENCES".to_owned()],
            min_depth: 1,
            max_depth: 3,
        }],
        filters: vec![GraphPropertyFilter {
            property_name: "active".to_owned(),
            operator: GraphFilterOperator::Equals,
            value: GraphPropertyValue::Boolean(true),
        }],
        limit: GraphQueryLimit::new(50).expect("positive limit should be valid"),
    }
    .validate()
    .expect("traversal should validate");

    assert_eq!(traversal.start_node_codes, vec!["NODE-A"]);
    assert_eq!(traversal.clauses[0].min_depth, 1);
    assert_eq!(traversal.clauses[0].max_depth, 3);
    assert_eq!(
        traversal.clauses[0].relationships,
        vec!["DEPENDS_ON", "REFERENCES"]
    );

    assert_eq!(
        GraphTraverseQuery {
            start_node_codes: vec![],
            clauses: vec![GraphTraversalClause {
                direction: GraphTraversalDirection::Any,
                relationships: vec![],
                min_depth: 0,
                max_depth: 0,
            }],
            filters: vec![],
            limit: GraphQueryLimit::new(10).expect("positive limit should be valid"),
        }
        .validate(),
        Err(GraphError::InvalidTraversalQuery)
    );
}
