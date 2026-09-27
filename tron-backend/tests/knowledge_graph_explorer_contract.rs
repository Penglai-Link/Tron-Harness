mod common;

use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
use tron_backend::{
    adapters::outbound::persistence::{GrafeoAdapterConfig, GrafeoGraphStore, GrafeoStoreCore},
    application::graph::{GraphService, ScopeResolver, WorkspaceDetailProvider},
    config::AppConfig,
    domain::{
        graph::{GraphEdgeWrite, GraphNodeWrite, GraphPropertyValue, GraphScopeCommand},
        workspace::{
            WorkspaceDetail, WorkspaceError, WorkspaceLineageItem, WorkspaceStatus,
            WorkspaceTreeNode,
        },
    },
};

use common::{app_with_config, authenticated_request, request, response_json};

#[tokio::test]
async fn list_namespaces_returns_all_visible_graph_namespaces() {
    let fixture = KnowledgeGraphFixture::new("namespace-list").await;

    let response = authenticated_request(
        &fixture.router,
        Method::GET,
        "/api/v1/knowledge-graph/list-namespaces?workspace-code=TRON_CORE",
        None,
    )
    .await;

    let body = success_json(response).await;
    assert_eq!(body["code"], json!("SUCCESS"));
    assert_eq!(body["data"], json!(["other_namespace", "rag_skill"]));
}

#[tokio::test]
async fn snapshot_returns_space_and_current_user_without_other_user_data() {
    let fixture = KnowledgeGraphFixture::new("snapshot-scope").await;

    let response = authenticated_request(
        &fixture.router,
        Method::GET,
        "/api/v1/knowledge-graph?namespace=rag_skill&workspace-code=TRON_CORE",
        None,
    )
    .await;

    let body = success_json(response).await;
    assert_eq!(body["code"], json!("SUCCESS"));
    assert_eq!(body["data"]["scope"]["currentWorkspaceCode"], "TRON_CORE");
    assert_eq!(body["data"]["scope"]["rootWorkspaceCode"], "ENTERPRISE");
    assert_eq!(
        body["data"]["scope"]["visibleWorkspaceCodes"],
        json!(["ENTERPRISE", "PLATFORM", "TRON_CORE"])
    );
    assert_eq!(
        body["data"]["scope"]["visibleUserCodes"],
        json!(["__NONE__", "USR-AUTH-0001"])
    );

    assert_node_codes(
        &body,
        &[
            "CURRENT_SPACE_NODE",
            "PARENT_SPACE_NODE",
            "USER_PRIVATE_NODE",
            "USER_OVERRIDE_NODE",
            "WORKSPACE_OVERRIDE_NODE",
        ],
    );
    assert_no_node_code(&body, "OTHER_USER_NODE");
    assert_no_node_code(&body, "SIBLING_NODE");
    assert_no_node_code(&body, "CHILD_NODE");
    assert_no_node_code(&body, "OTHER_NAMESPACE_NODE");
    assert_edge_codes(&body, &["CURRENT_TO_PARENT", "USER_PRIVATE_LINK"]);

    let user_node = node_by_code(&body, "USER_PRIVATE_NODE");
    assert_eq!(user_node["scopeType"], "USER");
    assert_eq!(user_node["originUserCode"], "USR-AUTH-0001");
    assert_eq!(user_node["editable"], true);
    assert_eq!(user_node["inherited"], false);

    let parent_node = node_by_code(&body, "PARENT_SPACE_NODE");
    assert_eq!(parent_node["scopeType"], "SPACE");
    assert_eq!(parent_node["originWorkspaceCode"], "PLATFORM");
    assert_eq!(parent_node["originWorkspaceName"], "Platform");
    assert_eq!(parent_node["editable"], false);
    assert_eq!(parent_node["inherited"], true);
}

#[tokio::test]
async fn snapshot_omits_embedding_vector_payload_fields() {
    let fixture = KnowledgeGraphFixture::new("snapshot-no-embedding-payload").await;

    let response = authenticated_request(
        &fixture.router,
        Method::GET,
        "/api/v1/knowledge-graph?namespace=rag_skill&workspace-code=TRON_CORE",
        None,
    )
    .await;

    let body = success_json(response).await;
    let node = node_by_code(&body, "CURRENT_SPACE_NODE");

    assert!(node.get("embeddingText").is_none());
    assert!(node["properties"].get("embeddingJson").is_none());
    assert_eq!(node["properties"]["source"], "space");
}

#[tokio::test]
async fn snapshot_returns_all_visible_nodes_without_default_limit() {
    let fixture = KnowledgeGraphFixture::new_with_extra_nodes("snapshot-no-node-limit", 125).await;

    let response = authenticated_request(
        &fixture.router,
        Method::GET,
        "/api/v1/knowledge-graph?namespace=rag_skill&workspace-code=TRON_CORE",
        None,
    )
    .await;

    let body = success_json(response).await;
    assert_eq!(body["data"]["window"]["nodeTotal"], 130);
    assert_eq!(body["data"]["window"]["returnedNodeCount"], 130);
    assert_eq!(body["data"]["window"]["hasMore"], false);
    assert!(node_by_code(&body, "BULK_VISIBLE_NODE_124").is_object());
}

#[tokio::test]
async fn snapshot_uses_workspace_lineage_without_sibling_or_child_visibility() {
    let fixture = KnowledgeGraphFixture::new("workspace-lineage").await;

    let response = authenticated_request(
        &fixture.router,
        Method::GET,
        "/api/v1/knowledge-graph?namespace=rag_skill&workspace-code=TRON_CORE",
        None,
    )
    .await;
    let body = success_json(response).await;

    assert_node_codes(
        &body,
        &[
            "CURRENT_SPACE_NODE",
            "PARENT_SPACE_NODE",
            "USER_PRIVATE_NODE",
            "USER_OVERRIDE_NODE",
            "WORKSPACE_OVERRIDE_NODE",
        ],
    );
    assert_no_node_code(&body, "SIBLING_NODE");
    assert_no_node_code(&body, "CHILD_NODE");
    assert_no_edge_code(&body, "SIBLING_EDGE");
    assert_no_edge_code(&body, "CHILD_EDGE");
}

#[tokio::test]
async fn snapshot_applies_user_and_nearest_workspace_override_rules() {
    let fixture = KnowledgeGraphFixture::new("override-rules").await;

    let response = authenticated_request(
        &fixture.router,
        Method::GET,
        "/api/v1/knowledge-graph?namespace=rag_skill&workspace-code=TRON_CORE",
        None,
    )
    .await;
    let body = success_json(response).await;

    assert_eq!(body["data"]["window"]["nodeTotal"], 5);

    let user_override = node_by_code(&body, "USER_OVERRIDE_NODE");
    assert_eq!(user_override["scopeType"], "USER");
    assert_eq!(user_override["properties"]["source"], "user");
    assert_eq!(user_override["originWorkspaceCode"], "TRON_CORE");
    assert_eq!(user_override["editable"], true);

    let workspace_override = node_by_code(&body, "WORKSPACE_OVERRIDE_NODE");
    assert_eq!(workspace_override["scopeType"], "SPACE");
    assert_eq!(workspace_override["properties"]["source"], "current-space");
    assert_eq!(workspace_override["originWorkspaceCode"], "TRON_CORE");
    assert_eq!(workspace_override["editable"], true);
}

#[tokio::test]
async fn query_filters_by_node_code_labels_and_relationship() {
    let fixture = KnowledgeGraphFixture::new("query-filters").await;

    let response = authenticated_request(
        &fixture.router,
        Method::POST,
        "/api/v1/knowledge-graph/query",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "namespace": "rag_skill",
            "scopeModes": ["SPACE", "USER"],
            "inheritanceMode": "WORKSPACE_LINEAGE",
            "query": "USER_OVERRIDE",
            "labels": ["RagSkill"],
            "relationship": "USES",
            "limit": 100
        })),
    )
    .await;

    let body = success_json(response).await;
    assert_node_codes(&body, &["USER_OVERRIDE_NODE"]);
    assert_edge_codes(&body, &[]);
}

#[tokio::test]
async fn query_filters_by_node_title() {
    let fixture = KnowledgeGraphFixture::new("query-title").await;

    let response = authenticated_request(
        &fixture.router,
        Method::POST,
        "/api/v1/knowledge-graph/query",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "namespace": "rag_skill",
            "scopeModes": ["SPACE", "USER"],
            "inheritanceMode": "WORKSPACE_LINEAGE",
            "query": "Current Shared Title",
            "labels": [],
            "relationships": [],
            "limit": 100
        })),
    )
    .await;

    let body = success_json(response).await;
    assert_node_codes(&body, &["CURRENT_SPACE_NODE"]);
    assert_edge_codes(&body, &[]);
}

#[tokio::test]
async fn query_label_filters_match_any_selected_label() {
    let fixture = KnowledgeGraphFixture::new("query-label-filter-any").await;

    let response = authenticated_request(
        &fixture.router,
        Method::POST,
        "/api/v1/knowledge-graph/query",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "namespace": "rag_skill",
            "scopeModes": ["SPACE", "USER"],
            "inheritanceMode": "WORKSPACE_LINEAGE",
            "query": "",
            "labels": ["RagSkill", "RagTag"],
            "relationships": [],
            "limit": 100
        })),
    )
    .await;

    let body = success_json(response).await;
    assert_node_codes(
        &body,
        &[
            "CURRENT_SPACE_NODE",
            "PARENT_SPACE_NODE",
            "USER_OVERRIDE_NODE",
            "USER_PRIVATE_NODE",
            "WORKSPACE_OVERRIDE_NODE",
        ],
    );
    assert_edge_codes(&body, &["CURRENT_TO_PARENT", "USER_PRIVATE_LINK"]);
}

#[tokio::test]
async fn query_filters_by_multiple_relationships() {
    let fixture = KnowledgeGraphFixture::new("query-multiple-relationships").await;

    let response = authenticated_request(
        &fixture.router,
        Method::POST,
        "/api/v1/knowledge-graph/query",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "namespace": "rag_skill",
            "scopeModes": ["SPACE", "USER"],
            "inheritanceMode": "WORKSPACE_LINEAGE",
            "query": "",
            "labels": [],
            "relationships": ["IGNORED_RELATIONSHIP", "USES"],
            "limit": 100
        })),
    )
    .await;

    let body = success_json(response).await;
    assert_node_codes(
        &body,
        &[
            "CURRENT_SPACE_NODE",
            "PARENT_SPACE_NODE",
            "USER_PRIVATE_NODE",
            "USER_OVERRIDE_NODE",
            "WORKSPACE_OVERRIDE_NODE",
        ],
    );
    assert_edge_codes(&body, &["CURRENT_TO_PARENT", "USER_PRIVATE_LINK"]);
}

#[tokio::test]
async fn expand_returns_authorized_neighborhood_only() {
    let fixture = KnowledgeGraphFixture::new("expand").await;

    let response = authenticated_request(
        &fixture.router,
        Method::POST,
        "/api/v1/knowledge-graph/expand",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "namespace": "rag_skill",
            "scopeModes": ["SPACE", "USER"],
            "inheritanceMode": "WORKSPACE_LINEAGE",
            "nodeCode": "CURRENT_SPACE_NODE",
            "depth": 1,
            "relationships": [],
            "limit": 100
        })),
    )
    .await;

    let body = success_json(response).await;
    assert_node_codes(&body, &["CURRENT_SPACE_NODE", "PARENT_SPACE_NODE"]);
    assert_edge_codes(&body, &["CURRENT_TO_PARENT"]);
    assert_no_node_code(&body, "OTHER_USER_NODE");
    assert_no_node_code(&body, "SIBLING_NODE");
}

#[tokio::test]
async fn http_enforces_uniform_response_auth_and_workspace_access() {
    let fixture = KnowledgeGraphFixture::new("http-boundary").await;

    let unauthenticated = request(
        &common::app_without_profile(),
        Method::GET,
        "/api/v1/knowledge-graph?namespace=rag_skill&workspace-code=TRON_CORE",
        None,
        None,
    )
    .await;
    assert_eq!(unauthenticated.status(), StatusCode::CONFLICT);
    let unauthenticated_body = response_json(unauthenticated).await;
    assert_eq!(unauthenticated_body["code"], "PERSONAI_PROFILE_REQUIRED");
    assert!(unauthenticated_body.get("data").is_some());

    // Profile-based desktop identity can select another existing workspace;
    // its graph scope must still exclude the current workspace's private data.
    let other_workspace = authenticated_request(
        &fixture.router,
        Method::GET,
        "/api/v1/knowledge-graph?namespace=rag_skill&workspace-code=COMPLIANCE",
        None,
    )
    .await;
    assert_eq!(other_workspace.status(), StatusCode::OK);
    let other_body = response_json(other_workspace).await;
    assert!(!other_body.to_string().contains("CURRENT_SPACE_NODE"));

    let malformed = authenticated_request(
        &fixture.router,
        Method::POST,
        "/api/v1/knowledge-graph/query",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "namespace": "",
            "limit": 0
        })),
    )
    .await;
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    let malformed_body = response_json(malformed).await;
    assert_eq!(malformed_body["code"], "KNOWLEDGE_GRAPH_REQUEST_INVALID");
    assert!(malformed_body.get("data").is_some());
}

struct KnowledgeGraphFixture {
    router: axum::Router,
}

impl KnowledgeGraphFixture {
    async fn new(name: &str) -> Self {
        Self::new_with_extra_nodes(name, 0).await
    }

    async fn new_with_extra_nodes(name: &str, extra_visible_nodes: usize) -> Self {
        let database_url = unique_path(name, "sqlite3").display().to_string();
        let graph_storage_path = unique_path(name, "grafeo").display().to_string();
        seed_graph(&graph_storage_path, extra_visible_nodes).await;
        let router = app_with_config(AppConfig {
            database_url,
            graph_storage_path,
            ..AppConfig::default()
        });
        Self { router }
    }
}

async fn seed_graph(graph_storage_path: &str, extra_visible_nodes: usize) {
    let graph = graph_service(graph_storage_path);
    for (scope, node) in [
        (
            space_scope("PLATFORM"),
            node_write(
                "PARENT_SPACE_NODE",
                &["RagSkill"],
                "parent shared review",
                "parent",
            ),
        ),
        (
            space_scope("PLATFORM"),
            node_write(
                "WORKSPACE_OVERRIDE_NODE",
                &["RagSkill"],
                "parent override",
                "parent-space",
            ),
        ),
        (
            space_scope("TRON_CORE"),
            node_write(
                "WORKSPACE_OVERRIDE_NODE",
                &["RagSkill"],
                "current override",
                "current-space",
            ),
        ),
        (
            space_scope("TRON_CORE"),
            node_write_with_embedding_payload(
                "CURRENT_SPACE_NODE",
                &["RagSkill"],
                "current shared",
                "space",
            ),
        ),
        (
            space_scope("TRON_CORE"),
            node_write("USER_OVERRIDE_NODE", &["RagSkill"], "space review", "space"),
        ),
        (
            user_scope("TRON_CORE", "USR-AUTH-0001"),
            node_write("USER_OVERRIDE_NODE", &["RagSkill"], "user review", "user"),
        ),
        (
            user_scope("TRON_CORE", "USR-AUTH-0001"),
            node_write("USER_PRIVATE_NODE", &["RagSkill"], "private review", "user"),
        ),
        (
            user_scope("TRON_CORE", "OTHER-USER"),
            node_write(
                "OTHER_USER_NODE",
                &["RagSkill"],
                "other user hidden",
                "other",
            ),
        ),
        (
            space_scope("TRON_SIBLING"),
            node_write("SIBLING_NODE", &["RagSkill"], "sibling hidden", "sibling"),
        ),
        (
            space_scope("TRON_CHILD"),
            node_write("CHILD_NODE", &["RagSkill"], "child hidden", "child"),
        ),
        (
            namespace_scope("other_namespace", "TRON_CORE", None),
            node_write(
                "OTHER_NAMESPACE_NODE",
                &["RagSkill"],
                "namespace hidden",
                "namespace",
            ),
        ),
    ] {
        graph
            .create_node(scope, node)
            .await
            .expect("node should seed");
    }

    for (scope, edge) in [
        (
            space_scope("TRON_CORE"),
            edge_write(
                "CURRENT_TO_PARENT",
                "CURRENT_SPACE_NODE",
                "PARENT_SPACE_NODE",
                "USES",
            ),
        ),
        (
            user_scope("TRON_CORE", "USR-AUTH-0001"),
            edge_write(
                "USER_PRIVATE_LINK",
                "USER_PRIVATE_NODE",
                "USER_OVERRIDE_NODE",
                "USES",
            ),
        ),
        (
            user_scope("TRON_CORE", "OTHER-USER"),
            edge_write(
                "OTHER_USER_EDGE",
                "OTHER_USER_NODE",
                "OTHER_USER_NODE",
                "USES",
            ),
        ),
        (
            space_scope("TRON_SIBLING"),
            edge_write("SIBLING_EDGE", "SIBLING_NODE", "SIBLING_NODE", "USES"),
        ),
        (
            space_scope("TRON_CHILD"),
            edge_write("CHILD_EDGE", "CHILD_NODE", "CHILD_NODE", "USES"),
        ),
    ] {
        graph
            .create_edge(scope, edge)
            .await
            .expect("edge should seed");
    }

    for index in 0..extra_visible_nodes {
        graph
            .create_node(
                space_scope("TRON_CORE"),
                node_write(
                    &format!("BULK_VISIBLE_NODE_{index:03}"),
                    &["RagSkill"],
                    &format!("bulk visible node {index}"),
                    "bulk",
                ),
            )
            .await
            .expect("bulk node should seed");
    }
}

fn graph_service(storage_path: &str) -> GraphService<ScopeResolver<TestWorkspaceProvider>> {
    let core = GrafeoStoreCore::connect(GrafeoAdapterConfig {
        storage_path: storage_path.to_owned(),
    })
    .expect("graph store should connect");
    GraphService::new(
        GrafeoGraphStore::new(core),
        ScopeResolver::with_workspace_provider(TestWorkspaceProvider),
    )
}

#[derive(Clone)]
struct TestWorkspaceProvider;

impl WorkspaceDetailProvider for TestWorkspaceProvider {
    fn get_workspace(&self, workspace_code: &str) -> Result<WorkspaceDetail, WorkspaceError> {
        workspace_detail(workspace_code).ok_or(WorkspaceError::NotFound)
    }
}

fn workspace_detail(workspace_code: &str) -> Option<WorkspaceDetail> {
    let (name, parent_workspace_code, depth, path) = match workspace_code {
        "ENTERPRISE" => (
            "Enterprise",
            None,
            0,
            vec![lineage_item("ENTERPRISE", "Enterprise", 0)],
        ),
        "PLATFORM" => (
            "Platform",
            Some("ENTERPRISE".to_owned()),
            1,
            vec![
                lineage_item("ENTERPRISE", "Enterprise", 0),
                lineage_item("PLATFORM", "Platform", 1),
            ],
        ),
        "TRON_CORE" => (
            "Tron Core",
            Some("PLATFORM".to_owned()),
            2,
            vec![
                lineage_item("ENTERPRISE", "Enterprise", 0),
                lineage_item("PLATFORM", "Platform", 1),
                lineage_item("TRON_CORE", "Tron Core", 2),
            ],
        ),
        "TRON_SIBLING" => (
            "Tron Sibling",
            Some("PLATFORM".to_owned()),
            2,
            vec![
                lineage_item("ENTERPRISE", "Enterprise", 0),
                lineage_item("PLATFORM", "Platform", 1),
                lineage_item("TRON_SIBLING", "Tron Sibling", 2),
            ],
        ),
        "TRON_CHILD" => (
            "Tron Child",
            Some("TRON_CORE".to_owned()),
            3,
            vec![
                lineage_item("ENTERPRISE", "Enterprise", 0),
                lineage_item("PLATFORM", "Platform", 1),
                lineage_item("TRON_CORE", "Tron Core", 2),
                lineage_item("TRON_CHILD", "Tron Child", 3),
            ],
        ),
        _ => return None,
    };

    Some(WorkspaceDetail {
        id: format!("id-{workspace_code}"),
        workspace_code: workspace_code.to_owned(),
        name: name.to_owned(),
        description: None,
        status: WorkspaceStatus::Active,
        parent_workspace_code,
        depth,
        created_at: "2026-07-03T00:00:00Z".to_owned(),
        updated_at: "2026-07-03T00:00:00Z".to_owned(),
        archived_at: None,
        path,
        children: Vec::<WorkspaceTreeNode>::new(),
    })
}

fn lineage_item(workspace_code: &str, name: &str, depth: usize) -> WorkspaceLineageItem {
    WorkspaceLineageItem {
        workspace_code: workspace_code.to_owned(),
        name: name.to_owned(),
        depth,
    }
}

fn space_scope(workspace_code: &str) -> GraphScopeCommand {
    namespace_scope("rag_skill", workspace_code, None)
}

fn user_scope(workspace_code: &str, user_code: &str) -> GraphScopeCommand {
    namespace_scope("rag_skill", workspace_code, Some(user_code))
}

fn namespace_scope(
    namespace: &str,
    workspace_code: &str,
    user_code: Option<&str>,
) -> GraphScopeCommand {
    GraphScopeCommand {
        namespace: namespace.to_owned(),
        workspace_code: workspace_code.to_owned(),
        app_code: None,
        user_code: user_code.map(str::to_owned),
    }
}

fn node_write(node_code: &str, labels: &[&str], full_text: &str, source: &str) -> GraphNodeWrite {
    GraphNodeWrite {
        node_code: node_code.to_owned(),
        labels: labels.iter().map(|label| (*label).to_owned()).collect(),
        properties: BTreeMap::from([(
            "source".to_owned(),
            GraphPropertyValue::Text(source.to_owned()),
        )]),
        full_text: Some(full_text.to_owned()),
        embedding_text: None,
        embedding_model_code: None,
    }
}

fn node_write_with_embedding_payload(
    node_code: &str,
    labels: &[&str],
    full_text: &str,
    source: &str,
) -> GraphNodeWrite {
    let mut node = node_write(node_code, labels, full_text, source);
    node.properties.insert(
        "title".to_owned(),
        GraphPropertyValue::Text("Current Shared Title".to_owned()),
    );
    node.properties.insert(
        "embeddingJson".to_owned(),
        GraphPropertyValue::Text("[0.1,0.2,0.3]".to_owned()),
    );
    node
}

fn edge_write(
    edge_code: &str,
    source_node_code: &str,
    target_node_code: &str,
    relationship: &str,
) -> GraphEdgeWrite {
    GraphEdgeWrite {
        edge_code: edge_code.to_owned(),
        source_node_code: source_node_code.to_owned(),
        target_node_code: target_node_code.to_owned(),
        relationship: relationship.to_owned(),
        properties: BTreeMap::new(),
        full_text: None,
        embedding_text: None,
        embedding_model_code: None,
    }
}

fn assert_node_codes(body: &Value, expected: &[&str]) {
    let mut actual = body["data"]["nodes"]
        .as_array()
        .expect("nodes should be an array")
        .iter()
        .map(|node| node["nodeCode"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    actual.sort();
    assert_eq!(actual, sorted(expected));
}

async fn success_json(response: axum::response::Response) -> Value {
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(status, StatusCode::OK, "unexpected response body: {body}");
    body
}

fn assert_edge_codes(body: &Value, expected: &[&str]) {
    let mut actual = body["data"]["edges"]
        .as_array()
        .expect("edges should be an array")
        .iter()
        .map(|edge| edge["edgeCode"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    actual.sort();
    assert_eq!(actual, sorted(expected));
}

fn sorted(values: &[&str]) -> Vec<String> {
    let mut values = values
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    values.sort();
    values
}

fn node_by_code<'a>(body: &'a Value, node_code: &str) -> &'a Value {
    body["data"]["nodes"]
        .as_array()
        .expect("nodes should be an array")
        .iter()
        .find(|node| node["nodeCode"] == node_code)
        .expect("node should exist")
}

fn assert_no_node_code(body: &Value, node_code: &str) {
    assert!(
        body["data"]["nodes"]
            .as_array()
            .expect("nodes should be an array")
            .iter()
            .all(|node| node["nodeCode"] != node_code),
        "node {node_code} should not be returned",
    );
}

fn assert_no_edge_code(body: &Value, edge_code: &str) {
    assert!(
        body["data"]["edges"]
            .as_array()
            .expect("edges should be an array")
            .iter()
            .all(|edge| edge["edgeCode"] != edge_code),
        "edge {edge_code} should not be returned",
    );
}

fn unique_path(name: &str, extension: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    let process_id = std::process::id();
    std::env::temp_dir().join(format!(
        "tron-knowledge-graph-{name}-{process_id}-{nanos}.{extension}"
    ))
}
