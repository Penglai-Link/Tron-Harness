use std::{
    collections::{BTreeMap, VecDeque},
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use grafeo::{GrafeoDB, Value};
use serde_json::Value as JsonValue;
use tron_backend::{
    adapters::outbound::persistence::{
        GrafeoAdapterConfig, GrafeoGraphStore, GrafeoStoreCore, encode_graph_name,
        encode_schema_name,
    },
    application::graph::{
        GraphChunkedEmbeddingCallResult, GraphEmbeddingCallResult, GraphEmbeddingRuntime,
        GraphSemanticProjection, GraphSemanticSlice, GraphService, ScopeResolver,
        WorkspaceDetailProvider,
    },
    domain::{
        graph::{
            GraphEdgeWrite, GraphError, GraphNodeWrite, GraphPropertyFilter, GraphPropertyValue,
            GraphQueryLimit, GraphScopeCommand, GraphSearchQuery, GraphSearchTraversal,
            GraphTraversalClause, GraphTraversalDirection,
        },
        workspace::{WorkspaceDetail, WorkspaceError, WorkspaceLineageItem, WorkspaceStatus},
    },
};

#[tokio::test]
async fn vector_indexed_writes_persist_adapter_owned_semantic_payloads() {
    let storage_path = unique_graph_storage_path();
    let runtime = FakeEmbeddingRuntime::default();
    runtime.push_single_result(Ok(single_embedding("TEXT_EMBED_3_SMALL", &[0.9, 0.1])));
    runtime.push_chunked_result(Ok(chunked_embedding(
        "TEXT_EMBED_3_SMALL",
        &[
            (0, "chunk-alpha", &[0.8, 0.2]),
            (1, "chunk-beta", &[0.3, 0.7]),
        ],
    )));
    runtime.push_single_result(Ok(single_embedding("TEXT_EMBED_3_SMALL", &[0.4, 0.6])));
    {
        let graph_service = build_graph_service(&storage_path, runtime);

        graph_service
            .create_node(root_scope(), plain_node("ROOT_NODE", "root anchor", 1))
            .await
            .expect("root node should persist");
        graph_service
            .create_node(
                child_scope(),
                semantic_node("SEM_NODE", "release semantic body", 5),
            )
            .await
            .expect("semantic node should persist");
        graph_service
            .create_node(
                child_scope(),
                long_semantic_node("LONG_NODE", &"x".repeat(513), 8),
            )
            .await
            .expect("long semantic node should persist");
        graph_service
            .create_edge(
                child_scope(),
                semantic_edge("ROOT_TO_SEM", "ROOT_NODE", "SEM_NODE", "LINKS", 1.0),
            )
            .await
            .expect("semantic edge should persist");
    }

    let node_payload = semantic_payload_json(
        &storage_path,
        "knowledge-graph",
        "WS-ROOT",
        "MATCH (n:TronGraphNode) WHERE n.nodeCode = 'SEM_NODE' RETURN n.semanticPayload",
    );
    assert_eq!(
        node_payload.get("model_code").and_then(JsonValue::as_str),
        Some("TEXT_EMBED_3_SMALL")
    );
    assert_eq!(
        node_payload
            .get("projection")
            .and_then(JsonValue::as_array)
            .map(Vec::len),
        Some(2)
    );

    let long_payload = semantic_payload_json(
        &storage_path,
        "knowledge-graph",
        "WS-ROOT",
        "MATCH (n:TronGraphNode) WHERE n.nodeCode = 'LONG_NODE' RETURN n.semanticPayload",
    );
    assert!(
        long_payload
            .get("projection")
            .is_some_and(JsonValue::is_null)
    );
    assert_eq!(
        long_payload
            .get("slices")
            .and_then(JsonValue::as_array)
            .map(Vec::len),
        Some(2)
    );

    let edge_payload = semantic_payload_json(
        &storage_path,
        "knowledge-graph",
        "WS-ROOT",
        "MATCH ()-[e:TRON_EDGE]->() WHERE e.edgeCode = 'ROOT_TO_SEM' RETURN e.semanticPayload",
    );
    assert_eq!(
        edge_payload.get("model_code").and_then(JsonValue::as_str),
        Some("TEXT_EMBED_3_SMALL")
    );
    assert_eq!(
        edge_payload
            .get("projection")
            .and_then(JsonValue::as_array)
            .map(Vec::len),
        Some(2)
    );
}

#[tokio::test]
async fn hybrid_search_combines_traversal_filters_text_and_vector_with_scope_isolation() {
    let storage_path = unique_graph_storage_path();
    let runtime = FakeEmbeddingRuntime::default();
    runtime.push_single_result(Ok(single_embedding("TEXT_EMBED_3_SMALL", &[1.0, 0.0])));
    runtime.push_single_result(Ok(single_embedding("TEXT_EMBED_3_SMALL", &[0.0, 1.0])));
    runtime.push_single_result(Ok(single_embedding("TEXT_EMBED_3_SMALL", &[1.0, 0.0])));
    let graph_service = build_graph_service(&storage_path, runtime);

    graph_service
        .create_node(root_scope(), plain_node("ROOT_NODE", "launch root", 1))
        .await
        .expect("root node should persist");
    graph_service
        .create_node(
            child_scope(),
            semantic_node("TARGET_NODE", "launch child memory", 5),
        )
        .await
        .expect("target node should persist");
    graph_service
        .create_node(
            other_root_child_scope(),
            semantic_node("HIDDEN_NODE", "launch hidden memory", 5),
        )
        .await
        .expect("hidden node should persist");
    graph_service
        .create_edge(
            child_scope(),
            plain_edge("ROOT_TO_TARGET", "ROOT_NODE", "TARGET_NODE", "LINKS", 1.0),
        )
        .await
        .expect("target edge should persist");

    let hits = graph_service
        .search(
            child_scope(),
            GraphSearchQuery {
                filters: vec![GraphPropertyFilter {
                    property_name: "priority".to_owned(),
                    operator: tron_backend::domain::graph::GraphFilterOperator::Equals,
                    value: GraphPropertyValue::Integer(5),
                }],
                full_text: Some("launch".to_owned()),
                vector_text: Some("release semantic lookup".to_owned()),
                embedding_model_code: Some("TEXT_EMBED_3_SMALL".to_owned()),
                traversal: Some(GraphSearchTraversal {
                    start_node_codes: vec!["ROOT_NODE".to_owned()],
                    clauses: vec![GraphTraversalClause {
                        direction: GraphTraversalDirection::Outgoing,
                        relationships: vec!["LINKS".to_owned()],
                        min_depth: 1,
                        max_depth: 1,
                    }],
                }),
                labels: vec!["Task".to_owned()],
                limit: GraphQueryLimit::new(10).expect("limit should validate"),
                min_score: Some(0.7),
            },
        )
        .await
        .expect("hybrid search should pass");

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].node.node_code, "TARGET_NODE");
    assert!(hits[0].score.is_some_and(|score| score >= 0.99));
}

#[derive(Clone, Default)]
struct FakeEmbeddingRuntime {
    state: Arc<Mutex<FakeEmbeddingRuntimeState>>,
}

#[derive(Default)]
struct FakeEmbeddingRuntimeState {
    single_results: VecDeque<Result<GraphEmbeddingCallResult, GraphError>>,
    chunked_results: VecDeque<Result<GraphChunkedEmbeddingCallResult, GraphError>>,
}

impl FakeEmbeddingRuntime {
    fn push_single_result(&self, result: Result<GraphEmbeddingCallResult, GraphError>) {
        self.state
            .lock()
            .expect("embedding runtime state mutex poisoned")
            .single_results
            .push_back(result);
    }

    fn push_chunked_result(&self, result: Result<GraphChunkedEmbeddingCallResult, GraphError>) {
        self.state
            .lock()
            .expect("embedding runtime state mutex poisoned")
            .chunked_results
            .push_back(result);
    }
}

impl GraphEmbeddingRuntime for FakeEmbeddingRuntime {
    fn embedding_call<'a>(
        &'a self,
        _workspace_code: &'a str,
        _model_code: Option<&'a str>,
        _input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<GraphEmbeddingCallResult, GraphError>> + Send + 'a>>
    {
        Box::pin(async move {
            self.state
                .lock()
                .expect("embedding runtime state mutex poisoned")
                .single_results
                .pop_front()
                .expect("single embedding result should be queued")
        })
    }

    fn embedding_call_with_chunking<'a>(
        &'a self,
        _workspace_code: &'a str,
        _model_code: Option<&'a str>,
        _input: &'a str,
        _max_chars_per_chunk: usize,
    ) -> Pin<
        Box<dyn Future<Output = Result<GraphChunkedEmbeddingCallResult, GraphError>> + Send + 'a>,
    > {
        Box::pin(async move {
            self.state
                .lock()
                .expect("embedding runtime state mutex poisoned")
                .chunked_results
                .pop_front()
                .expect("chunked embedding result should be queued")
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct FakeWorkspaceProvider;

impl WorkspaceDetailProvider for FakeWorkspaceProvider {
    fn get_workspace(&self, workspace_code: &str) -> Result<WorkspaceDetail, WorkspaceError> {
        workspace_detail(workspace_code).ok_or(WorkspaceError::NotFound)
    }
}

fn build_graph_service(
    storage_path: &Path,
    runtime: FakeEmbeddingRuntime,
) -> GraphService<ScopeResolver<FakeWorkspaceProvider>> {
    let core = GrafeoStoreCore::connect(GrafeoAdapterConfig {
        storage_path: storage_path.display().to_string(),
    })
    .expect("Grafeo adapter should initialize");
    GraphService::with_embedding_runtime(
        GrafeoGraphStore::new(core),
        ScopeResolver::with_workspace_provider(FakeWorkspaceProvider),
        Arc::new(runtime),
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
        app_code: Some("APP-A".to_owned()),
        user_code: Some("USER-A".to_owned()),
    }
}

fn child_scope() -> GraphScopeCommand {
    GraphScopeCommand {
        namespace: "knowledge-graph".to_owned(),
        workspace_code: "WS-CHILD".to_owned(),
        app_code: Some("APP-A".to_owned()),
        user_code: Some("USER-A".to_owned()),
    }
}

fn other_root_child_scope() -> GraphScopeCommand {
    GraphScopeCommand {
        namespace: "knowledge-graph".to_owned(),
        workspace_code: "WS-OTHER-CHILD".to_owned(),
        app_code: Some("APP-A".to_owned()),
        user_code: Some("USER-A".to_owned()),
    }
}

fn plain_node(node_code: &str, full_text: &str, priority: i64) -> GraphNodeWrite {
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

fn semantic_node(node_code: &str, full_text: &str, priority: i64) -> GraphNodeWrite {
    GraphNodeWrite {
        embedding_text: Some("release semantic body".to_owned()),
        embedding_model_code: Some("TEXT_EMBED_3_SMALL".to_owned()),
        ..plain_node(node_code, full_text, priority)
    }
}

fn long_semantic_node(node_code: &str, embedding_text: &str, priority: i64) -> GraphNodeWrite {
    GraphNodeWrite {
        embedding_text: Some(embedding_text.to_owned()),
        embedding_model_code: Some("TEXT_EMBED_3_SMALL".to_owned()),
        ..plain_node(node_code, "long semantic carrier", priority)
    }
}

fn plain_edge(
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

fn semantic_edge(
    edge_code: &str,
    source_node_code: &str,
    target_node_code: &str,
    relationship: &str,
    weight: f64,
) -> GraphEdgeWrite {
    GraphEdgeWrite {
        full_text: Some("semantic link body".to_owned()),
        embedding_text: Some("semantic link body".to_owned()),
        embedding_model_code: Some("TEXT_EMBED_3_SMALL".to_owned()),
        ..plain_edge(
            edge_code,
            source_node_code,
            target_node_code,
            relationship,
            weight,
        )
    }
}

fn single_embedding(model_code: &str, projection: &[f32]) -> GraphEmbeddingCallResult {
    GraphEmbeddingCallResult {
        model_code: model_code.to_owned(),
        projection: GraphSemanticProjection::from_values(projection.to_vec()),
    }
}

fn chunked_embedding(
    model_code: &str,
    slices: &[(usize, &str, &[f32])],
) -> GraphChunkedEmbeddingCallResult {
    GraphChunkedEmbeddingCallResult {
        model_code: model_code.to_owned(),
        slices: slices
            .iter()
            .map(|(chunk_index, content, projection)| GraphSemanticSlice {
                chunk_index: *chunk_index,
                content: (*content).to_owned(),
                projection: GraphSemanticProjection::from_values(projection.to_vec()),
            })
            .collect(),
    }
}

fn semantic_payload_json(
    storage_path: &Path,
    namespace: &str,
    named_graph: &str,
    query: &str,
) -> JsonValue {
    let row = single_row(query_string_rows(
        storage_path,
        namespace,
        named_graph,
        query,
    ));
    serde_json::from_str(&row[0]).expect("semantic payload should be valid JSON")
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
        .expect("graph hybrid contract query should succeed");
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

fn unique_graph_storage_path() -> PathBuf {
    static UNIQUE_GRAPH_STORAGE_COUNTER: AtomicU64 = AtomicU64::new(0);

    let counter = UNIQUE_GRAPH_STORAGE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "tron-grafeo-hybrid-contract-{}-{suffix}-{counter}.grafeo",
        std::process::id()
    ))
}
