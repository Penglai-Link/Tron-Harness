mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use fixture::*;
use serde_json::{Value, json};
use std::sync::{Arc, atomic::Ordering};
use tower::ServiceExt;
use tron_backend::{
    application::{llm::LlmRuntimeService, rag::*},
    domain::agent::{AgentRunContext, ToolError, ToolExecutionControl, ToolRequest},
};
const SEARCH: &str = "/api/v1/rag/graph/search";
const EXPAND: &str = "/api/v1/rag/graph/expand";
const BROWSE: &str = "/api/v1/rag/graph/browse";

#[tokio::test]
async fn graph_browse_covers_sources_and_continues_disconnected_windows_without_model_calls() {
    use std::collections::BTreeSet;
    let f = Fixture::new();
    let mut corpora = vec![];
    for name in ["First", "Second", "Third"] {
        let kb = f.service.create(USER, name, "").unwrap();
        let doc = f
            .service
            .upload(USER, &kb.id, "manual.md", b"Alpha calls Beta.")
            .unwrap();
        f.service.index(USER, &kb.id, &doc.id).await.unwrap();
        corpora.push(kb.id);
    }
    let mut state = f.state.clone();
    state.knowledge_bases = f.service.clone();
    let app = tron_backend::app::build_router_with_state(state).unwrap();
    let calls = f.client.calls.load(Ordering::SeqCst);
    f.client.failing.store(true, Ordering::SeqCst);
    let response = common::authenticated_request(
        &app,
        Method::POST,
        BROWSE,
        Some(json!({
            "corpusIds": corpora, "seedCount": 3, "incomingDepth": 0, "outgoingDepth": 0,
            "nodeLimit": 3, "edgeLimit": 20
        })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let first = common::response_json(response).await;
    let first = &first["data"];
    let snapshot = first["snapshotId"].clone();
    let rows = first["nodes"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows.iter()
            .map(|node| node["corpusId"].as_str().unwrap())
            .collect::<BTreeSet<_>>()
            .len(),
        3
    );
    assert!(
        first["truncatedBy"]
            .as_array()
            .unwrap()
            .contains(&json!("BrowseSeedBudget"))
    );
    assert!(
        first["nextCursor"].is_string(),
        "unseen disconnected nodes need a continuation"
    );
    let mut seen = rows
        .iter()
        .map(|node| node["id"].as_str().unwrap().to_owned())
        .collect::<BTreeSet<_>>();
    let mut cursor = first["nextCursor"].clone();
    let mut pages = 1;
    while !cursor.is_null() {
        assert!(pages < 5, "browse continuation did not make progress");
        let response = common::authenticated_request(
            &app,
            Method::POST,
            EXPAND,
            Some(json!({"cursor": cursor})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let next = common::response_json(response).await;
        let next = &next["data"];
        assert_eq!(next["snapshotId"], snapshot);
        no_storage_fields(next);
        let before = seen.len();
        seen.extend(
            next["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|node| node["id"].as_str().unwrap().to_owned()),
        );
        assert!(seen.len() > before);
        cursor = next["nextCursor"].clone();
        pages += 1;
    }
    assert_eq!(seen.len(), 6);
    assert_eq!(f.client.calls.load(Ordering::SeqCst), calls);
    f.state.agent_framework.close().await;
}

fn no_storage_fields(value: &Value) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                assert!(
                    !matches!(
                        key.as_str(),
                        "embedding" | "vector" | "properties" | "apiKey" | "traversal" | "taskId"
                    ),
                    "private field {key}"
                );
                no_storage_fields(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                no_storage_fields(value)
            }
        }
        _ => {}
    }
}
#[tokio::test]
async fn graph_management_returns_source_grounded_views_and_expands_without_embedding() {
    let f = Fixture::new();
    let kb = f.service.create(USER, "Service manuals", "").unwrap();
    let doc = f
        .service
        .upload(
            USER,
            &kb.id,
            "operations.md",
            b"# Services\nAlpha calls Beta.",
        )
        .unwrap();
    f.service.index(USER, &kb.id, &doc.id).await.unwrap();
    assert!(
        f.service
            .library_agent_grants(USER, &kb.id)
            .unwrap()
            .is_empty(),
        "management uses ownership without an Agent grant"
    );
    let mut state = f.state.clone();
    state.knowledge_bases = f.service.clone();
    let app = tron_backend::app::build_router_with_state(state).unwrap();
    let response = common::authenticated_request(
        &app,
        Method::POST,
        SEARCH,
        Some(json!({"query":"Alpha","corpusIds":[kb.id]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let result = common::response_json(response).await;
    assert_eq!(result["code"], "0");
    let data = &result["data"];
    no_storage_fields(data);
    assert_eq!(data["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(data["edges"].as_array().unwrap().len(), 1);
    assert_eq!(data["references"][0]["fileName"], "operations.md");
    assert_eq!(data["references"][0]["quote"], "Alpha calls Beta.");
    assert_eq!(data["nodes"][0]["kind"], "Entity");
    assert_eq!(data["stages"][0]["status"], "Completed");
    assert!(data.get("nextCursor").unwrap().is_null());
    assert!(data.get("expiresAt").unwrap().is_null());
    let ids = data["references"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    for node in data["nodes"].as_array().unwrap() {
        for reference in node["referenceIds"].as_array().unwrap() {
            assert!(ids.contains(reference.as_str().unwrap()));
        }
    }
    let alpha = data["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["name"] == "Alpha")
        .unwrap()["id"]
        .clone();
    let calls = f.client.calls.load(Ordering::SeqCst);
    f.client.failing.store(true, Ordering::SeqCst);
    let response = common::authenticated_request(
        &app,
        Method::POST,
        BROWSE,
        Some(json!({"corpusIds":[kb.id]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let overview = common::response_json(response).await;
    assert_eq!(overview["data"]["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(overview["data"]["edges"].as_array().unwrap().len(), 1);
    assert_eq!(
        overview["data"]["references"][0]["quote"],
        "Alpha calls Beta."
    );
    assert_eq!(overview["data"]["stages"][0]["channel"], "graph.browse");
    assert_eq!(f.client.calls.load(Ordering::SeqCst), calls);
    no_storage_fields(&overview);
    let response = common::authenticated_request(
        &app,
        Method::POST,
        EXPAND,
        Some(json!({"nodeIds":[alpha],"corpusIds":[kb.id],"incomingDepth":0,"outgoingDepth":1})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(f.client.calls.load(Ordering::SeqCst), calls);
    let expanded = common::response_json(response).await;
    assert_eq!(expanded["data"]["edges"].as_array().unwrap().len(), 1);
    no_storage_fields(&expanded);
    let response = common::authenticated_request(
        &app,
        Method::POST,
        SEARCH,
        Some(json!({"query":"Alpha","corpusIds":[kb.id]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let error = common::response_json(response).await;
    assert_eq!(error["code"], "101033008");
    assert_eq!(error["data"], json!({}));
    assert!(!error.to_string().contains("synthetic embedding outage"));
    f.state.agent_framework.close().await;
}

#[tokio::test]
async fn graph_api_rejects_scope_forgery_invalid_budgets_and_expired_cursors_with_protocol_errors()
{
    let f = Fixture::new();
    let kb = f.service.create(USER, "Owned", "").unwrap();
    let foreign = f.service.create("FOREIGN", "Private", "").unwrap();
    let mut state = f.state.clone();
    state.knowledge_bases = f.service.clone();
    let app = tron_backend::app::build_router_with_state(state).unwrap();
    for (path, input, status, code) in [
        (
            BROWSE,
            json!({"corpusIds":[foreign.id]}),
            StatusCode::FORBIDDEN,
            "101033005",
        ),
        (
            BROWSE,
            json!({"userCode":"FOREIGN"}),
            StatusCode::BAD_REQUEST,
            "101033004",
        ),
        (
            BROWSE,
            json!({"nodeLimit":1}),
            StatusCode::BAD_REQUEST,
            "101033004",
        ),
        (
            BROWSE,
            json!({"incomingDepth":7}),
            StatusCode::BAD_REQUEST,
            "101033004",
        ),
        (
            SEARCH,
            json!({"query":"Alpha","corpusIds":[foreign.id]}),
            StatusCode::FORBIDDEN,
            "101033005",
        ),
        (
            SEARCH,
            json!({"query":"Alpha","userCode":"FOREIGN"}),
            StatusCode::BAD_REQUEST,
            "101033004",
        ),
        (
            SEARCH,
            json!({"query":"Alpha","workspaceCode":"PRIVATE"}),
            StatusCode::BAD_REQUEST,
            "101033004",
        ),
        (
            SEARCH,
            json!({"query":"Alpha","incomingDepth":7}),
            StatusCode::BAD_REQUEST,
            "101033004",
        ),
        (
            SEARCH,
            json!({"query":"Alpha","nodeLimit":1}),
            StatusCode::BAD_REQUEST,
            "101033004",
        ),
        (
            SEARCH,
            json!({"query":"Alpha","filters":[]}),
            StatusCode::BAD_REQUEST,
            "101033004",
        ),
        (
            EXPAND,
            json!({"nodeIds":["missing"],"corpusIds":[kb.id]}),
            StatusCode::NOT_FOUND,
            "101033006",
        ),
        (
            EXPAND,
            json!({"cursor":"0".repeat(64)}),
            StatusCode::CONFLICT,
            "101033007",
        ),
        (
            EXPAND,
            json!({"cursor":"0".repeat(64),"outgoingDepth":6}),
            StatusCode::BAD_REQUEST,
            "101033004",
        ),
    ] {
        let response = common::authenticated_request(&app, Method::POST, path, Some(input)).await;
        assert_eq!(response.status(), status);
        let body = common::response_json(response).await;
        assert_eq!(body["code"], code);
        assert_eq!(body["data"], json!({}));
    }
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(SEARCH)
                .header("Content-Type", "application/json")
                .header("Accept-Language", "en-US")
                .body(Body::from("{invalid"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = common::response_json(response).await;
    assert_eq!(body["message"], "Invalid graph query parameters");
    let response = common::authenticated_request(
        &app,
        Method::POST,
        SEARCH,
        Some(json!({"query":"x".repeat(70_000)})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(common::response_json(response).await["code"], "101033004");
    let mut settings = f.state.rag.settings_for_corpus(&kb.id).unwrap();
    settings.graph = false;
    f.state.rag.configure_corpus(&kb.id, settings).unwrap();
    let response = common::authenticated_request(
        &app,
        Method::POST,
        SEARCH,
        Some(json!({"query":"Alpha","corpusIds":[kb.id]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(common::response_json(response).await["code"], "101033010");
    let response = common::authenticated_request(
        &app,
        Method::POST,
        BROWSE,
        Some(json!({"corpusIds":[kb.id]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(common::response_json(response).await["code"], "101033010");
    f.state.agent_framework.close().await;
}
struct Control;
impl ToolExecutionControl for Control {
    fn check_active(&self) -> Result<(), ToolError> {
        Ok(())
    }
    fn remaining(&self) -> std::time::Duration {
        std::time::Duration::from_secs(10)
    }
    fn cancelled(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(std::future::pending())
    }
}
#[tokio::test]
async fn graph_tools_use_run_identity_and_require_agent_grants() {
    let f = Fixture::new();
    let kb = f.service.create(USER, "Manual", "").unwrap();
    let doc = f
        .service
        .upload(USER, &kb.id, "manual.md", b"Alpha calls Beta.")
        .unwrap();
    f.service.index(USER, &kb.id, &doc.id).await.unwrap();
    let llm = LlmRuntimeService::new(Arc::new(EmbeddingResolver), Arc::new(f.client.clone()));
    let rag = f
        .state
        .rag
        .with_runtime(DefaultRagRuntime::new(llm, FixtureGraphRouter));
    let tools = tron_backend::application::agent::rag_graph_tools(rag, f.service.clone());
    let search = tools
        .iter()
        .find(|t| t.name() == "rag_graph_search")
        .unwrap();
    let expand = tools
        .iter()
        .find(|t| t.name() == "rag_graph_expand")
        .unwrap();
    let context:AgentRunContext=serde_json::from_value(json!({
        "agentCode":f.agent,"conversationCode":null,"runCode":"graph-tools","workspaceCode":WORKSPACE,"userCode":USER,"locale":"en-US","agentKind":"REACT",
        "model":{"modelCode":"fixture-chat","protocol":"CHAT_COMPLETIONS","baseUrl":"http://127.0.0.1:1/v1","apiKey":"synthetic","modelName":"fixture"},
        "variables":[],"skills":[],"tools":[],"executionScope":{},"limits":{"maxSteps":2,"maxToolCalls":2,"maxParallelToolCalls":1},"messages":[],"observations":[],"startedAt":"fixture"
    })).unwrap();
    let call = ToolRequest {
        tool_name: "rag_graph_search".into(),
        invocation_id: None,
        input: json!({"query":"Alpha","corpusIds":[kb.id]}),
    };
    assert!(
        search
            .exec_async(call.clone(), &context, &Control)
            .unwrap()
            .await
            .is_err()
    );
    f.service
        .set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    let output = search
        .exec_async(call.clone(), &context, &Control)
        .unwrap()
        .await
        .unwrap();
    let data: Value = serde_json::from_str(&output.observation.output).unwrap();
    assert_eq!(data["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(data["references"][0]["fileName"], "manual.md");
    no_storage_fields(&data);
    let mut forged = call.clone();
    forged.input["userCode"] = json!("FOREIGN");
    assert!(
        search
            .exec_async(forged, &context, &Control)
            .unwrap()
            .await
            .is_err()
    );
    let request = ToolRequest {
        tool_name: "rag_graph_expand".into(),
        invocation_id: None,
        input: json!({"nodeIds":[data["nodes"][0]["id"]],"corpusIds":[kb.id],"options":{"incomingDepth":0,"outgoingDepth":1}}),
    };
    let calls = f.client.calls.load(Ordering::SeqCst);
    expand
        .exec_async(request.clone(), &context, &Control)
        .unwrap()
        .await
        .unwrap();
    assert_eq!(f.client.calls.load(Ordering::SeqCst), calls);
    // Verify actual AppState registration, not just separately constructed tools.
    let definition =
        tron_backend::application::agent::BusinessToolCatalog::tool("rag.graph.expand", "en-US")
            .unwrap();
    assert_eq!(definition.tool_name, "rag_graph_expand");
    assert!(!definition.description.starts_with("agent.toolCatalog"));
    let chinese =
        tron_backend::application::agent::BusinessToolCatalog::tool("rag.graph.search", "zh-CN")
            .unwrap();
    assert!(chinese.description.contains("A2UI"));
    let mut registered_context = context.clone();
    registered_context.tools =
        vec![tron_backend::application::agent::tool_record_to_effective_tool(definition)];
    let mut registered_request = request.clone();
    registered_request.input["workspaceCode"] = json!(WORKSPACE);
    let registered = f
        .state
        .agent_tool_registry
        .execute_allowed(registered_request, &registered_context)
        .unwrap();
    let registered: Value = serde_json::from_str(&registered.observation.output).unwrap();
    assert!(!registered["nodes"].as_array().unwrap().is_empty());
    f.service.set_grants(USER, &kb.id, vec![]).unwrap();
    assert!(
        expand
            .exec_async(request, &context, &Control)
            .unwrap()
            .await
            .is_err()
    );
    f.state.agent_framework.close().await;
}
