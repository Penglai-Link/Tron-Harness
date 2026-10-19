use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use tron_backend::{
    adapters::outbound::persistence::{GrafeoAdapterConfig, GrafeoRagStore, GrafeoStoreCore},
    application::rag::{
        RagTextUnitWrite,
        graph::{GraphAdjacencyQuery, GraphStructuralQuery, RagGraphStore, build_graph_generation},
    },
    domain::rag::graph::*,
};
fn partition(corpus: &str) -> GraphPartition {
    GraphPartition {
        corpus_id: corpus.into(),
        scope_type: "USER".into(),
        scope_code: "owner".into(),
    }
}
fn generation(id: &str) -> RagGraphGeneration {
    let source = format!("source-{id}");
    let chunk = format!("chunk-{id}");
    let nodes = ["A", "B"].map(|id| RagGraphNode {
        id: id.into(),
        kind: RagGraphNodeKind::Entity,
        generation_id: String::new(),
        name: id.into(),
        entity_type: Some("Concept".into()),
        properties: json!({}),
    });
    RagGraphGeneration {
        partition: partition("corpus-a"),
        id: id.into(),
        source_code: source.clone(),
        source_version: "v1".into(),
        task_id: id.into(),
        nodes: nodes.to_vec(),
        edges: vec![RagGraphEdge {
            id: format!("support-{id}"),
            logical_relation_id: "A-causes-B".into(),
            from: "A".into(),
            to: "B".into(),
            relationship: "CAUSES".into(),
            structural: false,
            support: RagGraphSupport {
                generation_id: id.into(),
                source_code: source,
                source_version: "v1".into(),
                task_id: id.into(),
                chunk_code: chunk,
                quote: "A causes B".into(),
                location: json!({"page": 1}),
            },
        }],
    }
}
fn query(generations: &[&str], direction: GraphDirection) -> GraphAdjacencyQuery {
    GraphAdjacencyQuery {
        partition: partition("corpus-a"),
        node_id: match direction {
            GraphDirection::Outgoing => "A",
            GraphDirection::Incoming => "B",
        }
        .into(),
        direction,
        generations: generations.iter().map(|s| (*s).into()).collect(),
        relationship_types: BTreeSet::new(),
        include_structural: false,
        after_edge_id: None,
        limit: 1,
    }
}
fn core(path: &str) -> GrafeoStoreCore {
    GrafeoStoreCore::connect(GrafeoAdapterConfig {
        storage_path: path.into(),
    })
    .unwrap()
}
#[test]
fn actual_edges_page_in_both_directions_and_filter_unauthorized_generations() {
    let store = GrafeoRagStore::new(core(":memory:"));
    let a = generation("one");
    let b = generation("two");
    a.validate(&BTreeMap::from([("chunk-one".into(), "A causes B".into())]))
        .unwrap();
    store.stage_generation(&a).unwrap();
    store.stage_generation(&a).unwrap();
    store.stage_generation(&b).unwrap();
    let mut diagnostic = query(&["one"], GraphDirection::Outgoing);
    diagnostic.limit = 100;
    let rows = store.adjacent(&diagnostic).unwrap();
    assert_eq!(rows.edges.len(), 1, "unexpected supports: {rows:#?}");
    let visible = store
        .adjacent(&query(&["one"], GraphDirection::Outgoing))
        .unwrap();
    assert_eq!(visible.edges.len(), 1);
    assert!(!visible.has_more, "{visible:#?}");
    assert_eq!(visible.edges[0].support.source_code, "source-one");
    let incoming = store
        .adjacent(&query(&["one"], GraphDirection::Incoming))
        .unwrap();
    assert_eq!(incoming.edges, visible.edges);
    let mut all = query(&["one", "two"], GraphDirection::Outgoing);
    let first = store.adjacent(&all).unwrap();
    assert!(first.has_more);
    all.after_edge_id = Some(first.edges[0].id.clone());
    let second = store.adjacent(&all).unwrap();
    assert!(!second.has_more);
    assert_ne!(first.edges[0].id, second.edges[0].id);
    all.partition = partition("other-corpus");
    assert!(store.adjacent(&all).unwrap().edges.is_empty());
    store.delete_generation(&a.partition, &a.id).unwrap();
    assert!(
        store
            .adjacent(&query(&["one"], GraphDirection::Outgoing))
            .unwrap()
            .edges
            .is_empty()
    );
    assert_eq!(
        store
            .adjacent(&query(&["two"], GraphDirection::Outgoing))
            .unwrap()
            .edges
            .len(),
        1
    );
    assert_eq!(
        store
            .nodes(&b.partition, &["A".into()], &BTreeSet::from(["two".into()]))
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn committed_graph_survives_reopening_and_shared_entities_have_no_source_descriptions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("graph.grafeo");
    let a = generation("one");
    {
        let store = GrafeoRagStore::new(core(path.to_str().unwrap()));
        store.stage_generation(&a).unwrap();
    }
    let store = GrafeoRagStore::new(core(path.to_str().unwrap()));
    assert_eq!(
        store
            .adjacent(&query(&["one"], GraphDirection::Outgoing))
            .unwrap()
            .edges
            .len(),
        1
    );
    let mut invalid = a.clone();
    invalid.nodes[0].properties = json!({"description":"private source"});
    assert!(invalid.validate(&BTreeMap::new()).is_err());
    store.delete_generation(&a.partition, &a.id).unwrap();
    let remaining = store
        .nodes(&a.partition, &["A".into()], &BTreeSet::from(["one".into()]))
        .unwrap();
    assert!(remaining.is_empty(), "remaining nodes: {remaining:#?}");
}

#[test]
fn entity_provenance_pages_only_authorized_structural_supports() {
    let store = GrafeoRagStore::new(core(":memory:"));
    for id in ["one", "two"] {
        let mut facts = generation(id);
        let chunk = format!("chunk-{id}");
        facts.nodes.push(RagGraphNode {
            id: chunk.clone(),
            kind: RagGraphNodeKind::Chunk,
            generation_id: id.into(),
            name: chunk.clone(),
            entity_type: None,
            properties: json!({}),
        });
        for relationship in ["MENTIONS", "DESCRIBES"] {
            let mut edge = facts.edges[0].clone();
            edge.id = format!("{relationship}-{id}");
            edge.logical_relation_id = edge.id.clone();
            edge.from = chunk.clone();
            edge.to = "A".into();
            edge.relationship = relationship.into();
            edge.structural = true;
            facts.edges.push(edge);
        }
        store.stage_generation(&facts).unwrap();
    }
    let ids = vec!["A".into(), "B".into()];
    let allowed = BTreeSet::from(["one".into()]);
    let first = store
        .entity_evidence(&partition("corpus-a"), &ids, &allowed, None, 1)
        .unwrap();
    assert_eq!(first.edges.len(), 1);
    assert!(first.has_more);
    let second = store
        .entity_evidence(
            &partition("corpus-a"),
            &ids,
            &allowed,
            Some(&first.edges[0].id),
            1,
        )
        .unwrap();
    assert_eq!(second.edges.len(), 1);
    assert!(!second.has_more);
    assert_ne!(first.edges[0].id, second.edges[0].id);
    for edge in first.edges.iter().chain(&second.edges) {
        assert_eq!(edge.from, "chunk-one");
        assert_eq!(edge.to, "A");
        assert_eq!(edge.support.generation_id, "one");
        assert!(edge.structural);
        assert!(["MENTIONS", "DESCRIBES"].contains(&edge.relationship.as_str()));
    }
    assert!(
        store
            .entity_evidence(&partition("corpus-b"), &ids, &allowed, None, 100)
            .unwrap()
            .edges
            .is_empty()
    );
    assert!(
        store
            .entity_evidence(&partition("corpus-a"), &ids, &BTreeSet::new(), None, 100)
            .unwrap()
            .edges
            .is_empty()
    );
}
#[test]
fn business_graph_scope_cannot_enter_reserved_rag_namespace() {
    use tron_backend::domain::graph::{GraphScope, GraphScopeCommand};
    assert!(
        GraphScope::from_command(GraphScopeCommand {
            namespace: "rag-internal".into(),
            workspace_code: "PLATFORM".into(),
            app_code: None,
            user_code: None
        })
        .is_err()
    );
}

#[test]
fn bfs_keeps_directions_separate_and_resumes_at_budget_without_losing_edges() {
    use tron_backend::application::rag::graph::{GraphExpansionState, GraphTraversalLimits};
    let store = GrafeoRagStore::new(core(":memory:"));
    let mut g = generation("chain");
    let names = ["M", "I1", "I2", "O1", "O2", "O3", "O4", "X"];
    g.nodes = names
        .iter()
        .map(|id| RagGraphNode {
            id: (*id).into(),
            kind: RagGraphNodeKind::Entity,
            generation_id: String::new(),
            name: (*id).into(),
            entity_type: Some("Concept".into()),
            properties: json!({}),
        })
        .collect();
    let support = g.edges[0].support.clone();
    g.edges = [
        ("I2", "I1"),
        ("I1", "M"),
        ("M", "O1"),
        ("O1", "O2"),
        ("O2", "O3"),
        ("O3", "O4"),
        ("I1", "X"),
        ("O3", "M"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (from, to))| RagGraphEdge {
        id: format!("edge-{index}"),
        logical_relation_id: format!("logical-{index}"),
        from: from.into(),
        to: to.into(),
        relationship: "LINKS".into(),
        structural: false,
        support: support.clone(),
    })
    .collect();
    store.stage_generation(&g).unwrap();
    let mut state = GraphExpansionState::new(
        g.partition.clone(),
        BTreeSet::from([g.id.clone()]),
        &["M".into()],
        BTreeSet::new(),
        3,
        3,
    )
    .unwrap();
    let mut nodes = BTreeSet::new();
    let mut edges = BTreeSet::new();
    let mut iterations = 0;
    loop {
        let window = state
            .expand(
                &store,
                GraphTraversalLimits {
                    node_limit: 3,
                    edge_limit: 2,
                },
                |_| Ok(true),
            )
            .unwrap();
        nodes.extend(window.node_ids);
        edges.extend(window.edges.into_iter().map(|e| e.id));
        iterations += 1;
        assert!(iterations < 12, "continuation did not advance");
        if !window.has_more {
            break;
        }
        // Opaque cursor state must survive serialization; identity binding is done by the query service.
        state = serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
    }
    assert_eq!(
        nodes,
        BTreeSet::from_iter(["M", "I1", "I2", "O1", "O2", "O3"].map(str::to_owned))
    );
    assert_eq!(edges.len(), 6);
    let mut blocked = GraphExpansionState::new(
        g.partition.clone(),
        BTreeSet::from([g.id]),
        &["M".into()],
        BTreeSet::new(),
        0,
        3,
    )
    .unwrap();
    let window = blocked
        .expand(&store, Default::default(), |support| {
            Ok(support.source_code != "source-chain")
        })
        .unwrap();
    assert!(window.edges.is_empty());
    assert!(!window.has_more);
}

#[test]
fn grafeo_adjacency_predicate_contract() {
    use grafeo::{GrafeoDB, Value};
    use std::collections::HashMap;
    let db = GrafeoDB::new_in_memory();
    let session = db.session();
    session
        .execute("INSERT (:Test {id: 'A'}), (:Test {id: 'B'})")
        .unwrap();
    session.execute("MATCH (a:Test), (b:Test) WHERE a.id = 'A' AND b.id = 'B' INSERT (a)-[:REL {id: 'one', generation: 'one'}]->(b)").unwrap();
    session.execute("MATCH (a:Test), (b:Test) WHERE a.id = 'A' AND b.id = 'B' INSERT (a)-[:REL {id: 'two', generation: 'two'}]->(b)").unwrap();
    let rows=session.execute_with_params("MATCH (n:Test)-[e:REL]->(other:Test) WHERE n.id = $id AND e.generation IN $generations RETURN e.id ORDER BY e.id LIMIT 3",
        HashMap::from([("id".into(),Value::from("A")),("generations".into(),Value::List(vec![Value::from("one")].into()))])).unwrap();
    assert_eq!(
        rows.rows().len(),
        1,
        "filtered adjacency rows: {:?}",
        rows.rows()
    );
}

#[test]
fn partial_generation_write_rolls_back_when_immutable_support_collides() {
    let store = GrafeoRagStore::new(core(":memory:"));
    let one = generation("one");
    store.stage_generation(&one).unwrap();
    let mut invalid = generation("two");
    invalid.nodes.push(RagGraphNode {
        id: "TEMPORARY".into(),
        kind: RagGraphNodeKind::Chunk,
        generation_id: "two".into(),
        name: "temporary".into(),
        entity_type: None,
        properties: json!({}),
    });
    invalid.edges[0].id = "support-one".into();
    assert!(store.stage_generation(&invalid).is_err());
    assert!(
        store
            .nodes(
                &one.partition,
                &["TEMPORARY".into()],
                &BTreeSet::from(["two".into()])
            )
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        store
            .adjacent(&query(&["one"], GraphDirection::Outgoing))
            .unwrap()
            .edges
            .len(),
        1
    );
}

fn source_unit(code: &str, text: &str, image: bool) -> RagTextUnitWrite {
    let original = json!({"sourceBlockId":"figure","blockKind":"image","page":2,"imageResourceCodes":["asset"],"imageAnnotations":[{"kind":"source_caption","text":"Figure 1 Original scale"}]});
    let location = if image {
        original.clone()
    } else {
        json!({"sourceBlockId":code,"blockKind":"paragraph","page":2})
    };
    RagTextUnitWrite {
        text_unit_code: code.into(),
        unit_type: if code == "pixels" { "IMAGE" } else { "LEAF" }.into(),
        text: text.into(),
        search_text: text.into(),
        heading_path: String::new(),
        ordinal: 0,
        token_count: 10,
        content_hash: code.into(),
        status: "ACTIVE".into(),
        fulltext_status: "INDEXED".into(),
        embedding_model_code: None,
        embedding_version: None,
        embedding_text_hash: None,
        embedding_dimension: None,
        embedding: None,
        embedding_status: "NOT_ENABLED".into(),
        metadata: json!({"location":location,"sourceLocations":[location]}),
    }
}
#[test]
fn source_structure_batch_pages_are_generation_scoped_and_survive_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("structural.grafeo");
    let units = vec![
        source_unit("body", "See Figure 1.", false),
        source_unit("caption", "Figure 1 Original scale", true),
        source_unit("pixels", "Figure 1 Original scale", true),
        source_unit("notice", "Unrelated notice", false),
    ];
    {
        let store = GrafeoRagStore::new(core(path.to_str().unwrap()));
        for id in ["one", "two"] {
            let evidence = serde_json::Value::Object(
                units
                    .iter()
                    .filter(|unit| unit.unit_type != "IMAGE")
                    .map(|unit| (unit.text_unit_code.clone(), json!({"nodes":[],"edges":[]})))
                    .collect(),
            );
            let facts = build_graph_generation(generation(id), &units, &evidence)
                .unwrap()
                .0;
            store.stage_generation(&facts).unwrap();
        }
    }
    let store = GrafeoRagStore::new(core(path.to_str().unwrap()));
    let mut query = GraphStructuralQuery {
        partition: partition("corpus-a"),
        node_ids: BTreeSet::from([
            stable_id("CHUNK", &["one", "caption"]),
            stable_id("CHUNK", &["one", "pixels"]),
        ]),
        direction: GraphDirection::Outgoing,
        generations: BTreeSet::from(["one".into()]),
        relationships: BTreeSet::from(["COVERS".into()]),
        after_edge_id: None,
        limit: 1,
    };
    let first = store.structural_adjacent(&query).unwrap();
    assert_eq!(first.edges.len(), 1);
    assert!(first.has_more);
    assert_eq!(first.edges[0].support.generation_id, "one");
    query.after_edge_id = Some(first.edges[0].id.clone());
    let second = store.structural_adjacent(&query).unwrap();
    assert_eq!(second.edges.len(), 1);
    assert!(!second.has_more);
    assert_ne!(second.edges[0].id, first.edges[0].id);
    query.node_ids = BTreeSet::from([stable_id("IMAGE", &["one", "figure"])]);
    query.direction = GraphDirection::Incoming;
    query.after_edge_id = None;
    query.limit = 100;
    let incoming = store.structural_adjacent(&query).unwrap();
    assert_eq!(incoming.edges.len(), 2);
    assert!(
        incoming
            .edges
            .iter()
            .all(|edge| ["caption", "pixels"].contains(&edge.support.chunk_code.as_str()))
    );
    query.node_ids = BTreeSet::from([stable_id("CHUNK", &["one", "body"])]);
    query.direction = GraphDirection::Outgoing;
    let body_coverage = store.structural_adjacent(&query).unwrap();
    assert_eq!(body_coverage.edges.len(), 1);
    let body_block = body_coverage.edges[0].to.clone();
    query.relationships = BTreeSet::from(["REFERENCES_FIGURE".into()]);
    assert!(store.structural_adjacent(&query).unwrap().edges.is_empty());
    // References belong to original parser blocks. Packed/overlapping chunks
    // only cover those blocks and must not become association connectors.
    query.node_ids = BTreeSet::from([body_block]);
    let references = store.structural_adjacent(&query).unwrap();
    assert_eq!(references.edges.len(), 1);
    assert_eq!(references.edges[0].support.quote, "Figure 1");
    assert_eq!(references.edges[0].support.chunk_code, "body");
    query.partition = partition("corpus-b");
    assert!(store.structural_adjacent(&query).unwrap().edges.is_empty());
    query.relationships = BTreeSet::from(["MENTIONS".into()]);
    assert!(store.structural_adjacent(&query).is_err());
    query.relationships = BTreeSet::from(["COVERS".into()]);
    query.node_ids = (0..501).map(|i| format!("node-{i}")).collect();
    assert!(store.structural_adjacent(&query).is_err());
}
