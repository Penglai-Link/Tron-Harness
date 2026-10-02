mod common;

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    extract::State,
    http::{Method, StatusCode},
    routing::post,
};
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};

use common::{app, authenticated_bearer_token, request_with_headers, response_json};

const POLICY: &str = "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent";

struct Server {
    url: String,
    requests: Arc<Mutex<Vec<Value>>>,
    task: JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn model_server(calls: Vec<(&str, Value)>) -> Server {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let replies = Arc::new(Mutex::new(VecDeque::from([
        json!({"choices":[{"message":{"role":"assistant","tool_calls":calls.into_iter().map(|(name, arguments)| json!({
            "id":format!("call_{name}"),"type":"function","function":{"name":name,"arguments":arguments.to_string()}
        })).collect::<Vec<_>>()}}]}),
        json!({"choices":[{"message":{"role":"assistant","content":"allowlist checked"}}]}),
    ])));
    async fn completion(
        State((requests, replies)): State<(Arc<Mutex<Vec<Value>>>, Arc<Mutex<VecDeque<Value>>>)>,
        Json(payload): Json<Value>,
    ) -> Json<Value> {
        requests.lock().unwrap().push(payload);
        Json(
            replies
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected extra model request"),
        )
    }
    let router = Router::new()
        .route("/v1/chat/completions", post(completion))
        .with_state((requests.clone(), replies));
    serve(router, requests, "/v1").await
}

async fn mcp_server() -> Server {
    let requests = Arc::new(Mutex::new(Vec::new()));
    async fn transport(
        State(requests): State<Arc<Mutex<Vec<Value>>>>,
        Json(payload): Json<Value>,
    ) -> Json<Value> {
        requests.lock().unwrap().push(payload.clone());
        let result = match payload["method"].as_str().unwrap_or_default() {
            "initialize" => {
                json!({"protocolVersion":"2025-06-18","serverInfo":{"name":"allowlist-fixture","version":"1"},"capabilities":{"tools":{}}})
            }
            "tools/list" => {
                json!({"tools":[{"name":"search/docs","description":"Real MCP definition","inputSchema":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}}]})
            }
            "tools/call" => {
                assert_eq!(
                    payload["params"]["name"], "search/docs",
                    "the original remote executor must keep its remote name"
                );
                json!({"content":[{"type":"text","text":"native MCP result"}]})
            }
            _ => json!({}),
        };
        Json(json!({"jsonrpc":"2.0","id":payload["id"],"result":result}))
    }
    let router = Router::new()
        .route("/mcp", post(transport))
        .with_state(requests.clone());
    serve(router, requests, "/mcp").await
}

async fn serve(router: Router, requests: Arc<Mutex<Vec<Value>>>, suffix: &str) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    Server {
        url: format!("http://{address}{suffix}"),
        requests,
        task: tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        }),
    }
}

async fn api(
    app: &Router,
    authorization: &str,
    method: Method,
    path: &str,
    payload: Value,
) -> Value {
    let response =
        request_with_headers(app, method, path, Some(payload), None, Some(authorization)).await;
    let status = response.status();
    let body = response_json(response).await;
    assert!(status.is_success(), "{status}: {body}");
    body
}

async fn fixture(selected: &[&str], exposure: &str) -> (Router, String) {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    api(
        &app,
        &authorization,
        Method::PATCH,
        &format!("{POLICY}/policy"),
        json!({"toolAssignmentMode":"selected_tools","toolExposureMode":exposure}),
    )
    .await;
    api(
        &app,
        &authorization,
        Method::PUT,
        &format!("{POLICY}/tool-assignments"),
        json!({"toolCodes":selected}),
    )
    .await;
    api(
        &app,
        &authorization,
        Method::PATCH,
        &format!("{POLICY}/mcp-policy"),
        json!({"mcpAccessMode":"disabled"}),
    )
    .await;
    (app, authorization)
}

fn forged_tool(name: &str) -> Value {
    json!({"toolName":name,"domainCode":"forged","description":"FORGED DESCRIPTION","inputSchema":{"type":"object","properties":{"forged":{"type":"boolean"}},"required":["forged"]},"skillName":null,"requiredVariableKeys":[]})
}

async fn run(app: &Router, authorization: &str, model: &Server, requested: Vec<Value>) -> Value {
    let body = api(app, authorization, Method::POST, "/api/v1/agents/runs", json!({
        "workspaceCode":"TRON_CORE","agentCode":"main-agent","prompt":"Exercise the saved allowlist.","tools":requested,
        "model":{"modelCode":"ALLOWLIST_TEST","protocol":"CHAT_COMPLETIONS","baseUrl":model.url,"apiKey":"fixture","modelName":"fixture","contextWindowTokens":65536}
    })).await;
    assert_eq!(body["data"]["finalOutput"], "allowlist checked", "{body}");
    assert_eq!(body["data"]["status"], "COMPLETED");
    assert!(
        body["data"]["native"]["runId"]
            .as_str()
            .is_some_and(|id| !id.is_empty()),
        "regression must execute through the native host: {body}"
    );
    body
}

fn exposed(model: &Server) -> Vec<Value> {
    model.requests.lock().unwrap()[0]["tools"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}
fn tool_output(model: &Server, name: &str) -> String {
    let requests = model.requests.lock().unwrap();
    let messages = requests[1]["messages"].as_array().unwrap();
    messages
        .iter()
        .find(|message| {
            message["role"] == "tool" && message["tool_call_id"] == format!("call_{name}")
        })
        .expect("native tool observation")["content"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn saved_empty_allowlist_blocks_forged_local_tools_at_native_execution() {
    let (app, authorization) = fixture(&[], "direct_list").await;
    let temp = tempfile::tempdir().unwrap();
    let marker = temp.path().join("must-not-exist.txt");
    let model = model_server(vec![
        (
            "fs_write_text",
            json!({"path":marker,"content":"unauthorized"}),
        ),
        ("bash_exec", json!({"command":"echo forbidden"})),
    ])
    .await;
    run(
        &app,
        &authorization,
        &model,
        vec![forged_tool("fs_write_text"), forged_tool("bash_exec")],
    )
    .await;
    assert!(exposed(&model).is_empty());
    assert!(
        !marker.exists(),
        "a forged DTO must not admit a local executor"
    );
    assert!(tool_output(&model, "fs_write_text").contains("is not allowed"));
    assert!(tool_output(&model, "bash_exec").contains("is not allowed"));
}

#[tokio::test]
async fn saved_selected_local_tools_use_original_schemas_and_bash_alias() {
    let (app, authorization) = fixture(&["bash.exec", "fs.read_text"], "direct_list").await;
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.txt");
    let marker = temp.path().join("denied.txt");
    std::fs::write(&source, "allowed read").unwrap();
    let model = model_server(vec![
        (
            "bash_exec",
            json!({"command":"echo native-alias-authorized"}),
        ),
        ("fs_read_text", json!({"path":source})),
        ("fs_write_text", json!({"path":marker,"content":"denied"})),
        (
            "system_exec",
            json!({"command":"echo canonical-name-must-not-expand-authority"}),
        ),
    ])
    .await;
    run(
        &app,
        &authorization,
        &model,
        vec![
            forged_tool("bash_exec"),
            forged_tool("fs_read_text"),
            forged_tool("fs_write_text"),
            forged_tool("system_exec"),
        ],
    )
    .await;
    let tools = exposed(&model);
    assert_eq!(tools.len(), 2, "{tools:?}");
    let bash = tools
        .iter()
        .find(|tool| tool["function"]["name"] == "bash_exec")
        .unwrap();
    assert!(bash["function"]["parameters"]["properties"]["command"].is_object());
    assert!(bash["function"]["parameters"]["properties"]["forged"].is_null());
    assert_ne!(bash["function"]["description"], "FORGED DESCRIPTION");
    assert!(tool_output(&model, "bash_exec").contains("native-alias-authorized"));
    assert!(tool_output(&model, "fs_read_text").contains("allowed read"));
    assert!(!marker.exists());
    assert!(tool_output(&model, "fs_write_text").contains("is not allowed"));
    assert!(tool_output(&model, "system_exec").contains("is not allowed"));
}

#[tokio::test]
async fn nonempty_request_narrows_saved_local_authority_while_empty_uses_saved_policy() {
    let (app, authorization) = fixture(&["bash.exec", "fs.read_text"], "direct_list").await;
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.txt");
    std::fs::write(&source, "read only").unwrap();
    let narrowed = model_server(vec![
        ("fs_read_text", json!({"path":source})),
        ("bash_exec", json!({"command":"echo must-be-denied"})),
    ])
    .await;
    run(
        &app,
        &authorization,
        &narrowed,
        vec![forged_tool("fs_read_text")],
    )
    .await;
    assert_eq!(exposed(&narrowed).len(), 1);
    assert!(tool_output(&narrowed, "fs_read_text").contains("read only"));
    assert!(tool_output(&narrowed, "bash_exec").contains("is not allowed"));
    let saved = model_server(vec![(
        "bash_exec",
        json!({"command":"echo empty-request-keeps-saved-policy"}),
    )])
    .await;
    run(&app, &authorization, &saved, vec![]).await;
    assert_eq!(exposed(&saved).len(), 2);
    assert!(tool_output(&saved, "bash_exec").contains("empty-request-keeps-saved-policy"));
}

#[tokio::test]
async fn domain_discovery_cannot_authorize_requested_tools_outside_saved_selection() {
    let (app, authorization) = fixture(&["bash.exec"], "domain_discovery").await;
    let temp = tempfile::tempdir().unwrap();
    let marker = temp.path().join("denied.txt");
    let model = model_server(vec![
        ("fs_write_text", json!({"path":marker,"content":"denied"})),
        (
            "bash_exec",
            json!({"command":"echo request-omitted-this-tool"}),
        ),
    ])
    .await;
    run(
        &app,
        &authorization,
        &model,
        vec![forged_tool("fs_write_text")],
    )
    .await;
    let tools = exposed(&model);
    assert_eq!(
        tools.len(),
        1,
        "only the saved discovery control is exposed"
    );
    assert_eq!(tools[0]["function"]["name"], "discovery_expand_tool_domain");
    assert!(!marker.exists());
    assert!(tool_output(&model, "fs_write_text").contains("is not allowed"));
    assert!(tool_output(&model, "bash_exec").contains("is not allowed"));
}

#[tokio::test]
async fn dynamic_mcp_respects_disabled_empty_selected_and_request_intersection() {
    let (app, authorization) = fixture(&[], "direct_list").await;
    let selected_server = mcp_server().await;
    let other_server = mcp_server().await;
    for (code, server) in [
        ("selected_tools", &selected_server),
        ("other_tools", &other_server),
    ] {
        api(
            &app,
            &authorization,
            Method::POST,
            "/api/v1/mcp/servers",
            json!({"serverCode":code,"name":code,"url":server.url,"authKind":"none","headers":{}}),
        )
        .await;
    }
    for mode in ["disabled", "selected_servers"] {
        api(
            &app,
            &authorization,
            Method::PATCH,
            &format!("{POLICY}/mcp-policy"),
            json!({"mcpAccessMode":mode}),
        )
        .await;
        let denied = model_server(vec![(
            "mcp_selected_tools_search_docs",
            json!({"query":"denied"}),
        )])
        .await;
        run(
            &app,
            &authorization,
            &denied,
            vec![forged_tool("mcp_selected_tools_search_docs")],
        )
        .await;
        assert!(exposed(&denied).is_empty(), "{mode}");
        assert!(tool_output(&denied, "mcp_selected_tools_search_docs").contains("is not allowed"));
        assert!(
            selected_server.requests.lock().unwrap().is_empty(),
            "{mode} must not even discover a server"
        );
        assert!(other_server.requests.lock().unwrap().is_empty());
    }
    api(
        &app,
        &authorization,
        Method::PUT,
        &format!("{POLICY}/mcp-server-assignments"),
        json!({"serverCodes":["selected_tools"]}),
    )
    .await;
    let selected = model_server(vec![
        ("mcp_selected_tools_search_docs", json!({"query":"allowed"})),
        ("mcp_other_tools_search_docs", json!({"query":"unselected"})),
    ])
    .await;
    run(
        &app,
        &authorization,
        &selected,
        vec![
            forged_tool("mcp_selected_tools_search_docs"),
            forged_tool("mcp_other_tools_search_docs"),
        ],
    )
    .await;
    let tools = exposed(&selected);
    assert_eq!(tools.len(), 1);
    assert_eq!(
        tools[0]["function"]["name"],
        "mcp_selected_tools_search_docs"
    );
    assert_eq!(tools[0]["function"]["description"], "Real MCP definition");
    assert!(tools[0]["function"]["parameters"]["properties"]["query"].is_object());
    assert!(tools[0]["function"]["parameters"]["properties"]["forged"].is_null());
    assert!(tool_output(&selected, "mcp_selected_tools_search_docs").contains("native MCP result"));
    assert!(tool_output(&selected, "mcp_other_tools_search_docs").contains("is not allowed"));
    assert!(
        other_server.requests.lock().unwrap().is_empty(),
        "an unselected server must not connect"
    );
    assert!(
        selected_server
            .requests
            .lock()
            .unwrap()
            .iter()
            .any(|request| request["method"] == "tools/call")
    );

    // All-enabled server policy still grants only the request's name intersection.
    api(
        &app,
        &authorization,
        Method::PATCH,
        &format!("{POLICY}/mcp-policy"),
        json!({"mcpAccessMode":"all_enabled_servers"}),
    )
    .await;
    let narrowed = model_server(vec![
        ("mcp_selected_tools_search_docs", json!({"query":"allowed"})),
        (
            "mcp_other_tools_search_docs",
            json!({"query":"request-omitted"}),
        ),
    ])
    .await;
    run(
        &app,
        &authorization,
        &narrowed,
        vec![forged_tool("mcp_selected_tools_search_docs")],
    )
    .await;
    assert_eq!(exposed(&narrowed).len(), 1);
    assert!(tool_output(&narrowed, "mcp_selected_tools_search_docs").contains("native MCP result"));
    assert!(tool_output(&narrowed, "mcp_other_tools_search_docs").contains("is not allowed"));
    assert!(
        other_server
            .requests
            .lock()
            .unwrap()
            .iter()
            .any(|request| request["method"] == "tools/list")
    );
    assert!(
        other_server
            .requests
            .lock()
            .unwrap()
            .iter()
            .all(|request| request["method"] != "tools/call")
    );

    // A request cannot revive a server disabled after the previous run snapshot.
    let response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/mcp/servers/selected_tools/disable",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let calls_before = selected_server.requests.lock().unwrap().len();
    let disabled = model_server(vec![(
        "mcp_selected_tools_search_docs",
        json!({"query":"disabled"}),
    )])
    .await;
    run(
        &app,
        &authorization,
        &disabled,
        vec![forged_tool("mcp_selected_tools_search_docs")],
    )
    .await;
    assert!(exposed(&disabled).is_empty());
    assert!(tool_output(&disabled, "mcp_selected_tools_search_docs").contains("is not allowed"));
    assert_eq!(selected_server.requests.lock().unwrap().len(), calls_before);
}
