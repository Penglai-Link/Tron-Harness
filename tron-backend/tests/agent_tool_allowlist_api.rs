mod common;

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    extract::State,
    http::{Method, StatusCode},
    response::IntoResponse,
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
    model_server_rounds(vec![calls]).await
}

/// Multi-turn mock: each round is one assistant tool-call batch; a final
/// text reply always follows. Gateway envelope calls keep the inner tool's
/// name in their call id so observations stay individually addressable.
async fn model_server_rounds(rounds: Vec<Vec<(&str, Value)>>) -> Server {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let mut scripted = Vec::new();
    for calls in rounds {
        let mut used_ids = std::collections::HashSet::new();
        scripted.push(json!({"choices":[{"message":{"role":"assistant","tool_calls":calls.into_iter().map(|(name, arguments)| {
            let base = if name == "tool_execute" {
                arguments["tool"].as_str().map(|inner| format!("call_{inner}")).unwrap_or_else(|| format!("call_{name}"))
            } else {
                format!("call_{name}")
            };
            // Call ids must stay unique within one batch.
            let mut id = base.clone();
            let mut suffix = 1;
            while !used_ids.insert(id.clone()) {
                suffix += 1;
                id = format!("{base}-{suffix}");
            }
            json!({
            "id":id,"type":"function","function":{"name":name,"arguments":arguments.to_string()}
        })}).collect::<Vec<_>>()}}]}));
    }
    scripted
        .push(json!({"choices":[{"message":{"role":"assistant","content":"allowlist checked"}}]}));
    let replies = Arc::new(Mutex::new(VecDeque::from(scripted)));
    async fn completion(
        State((requests, replies)): State<(Arc<Mutex<Vec<Value>>>, Arc<Mutex<VecDeque<Value>>>)>,
        Json(payload): Json<Value>,
    ) -> axum::response::Response {
        let streaming = payload["stream"].as_bool().unwrap_or(false);
        requests.lock().unwrap().push(payload);
        let reply = replies
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected extra model request");
        if !streaming {
            return Json(reply).into_response();
        }
        let mut delta = reply["choices"][0]["message"].clone();
        if let Some(calls) = delta["tool_calls"].as_array_mut() {
            for (index, call) in calls.iter_mut().enumerate() {
                call["index"] = json!(index);
            }
        }
        let finish = if delta["tool_calls"].is_array() {
            "tool_calls"
        } else {
            "stop"
        };
        let chunk = json!({"id":"fixture","object":"chat.completion.chunk","model":"fixture",
            "choices":[{"index":0,"delta":delta,"finish_reason":null}]});
        let done = json!({"id":"fixture","object":"chat.completion.chunk","model":"fixture",
            "choices":[{"index":0,"delta":{},"finish_reason":finish}]});
        (
            [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
            format!("data: {chunk}\n\ndata: {done}\n\ndata: [DONE]\n\n"),
        )
            .into_response()
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

async fn fixture(selected: &[&str]) -> (Router, String) {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    api(
        &app,
        &authorization,
        Method::PATCH,
        &format!("{POLICY}/policy"),
        json!({"toolAssignmentMode":"selected_tools"}),
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
// Direct-list cases below exercise saved local/MCP assignments. The platform's
// independent collaboration/skill facilities are covered in agent_collaboration.
// Keep every other name, so an unexpected tool still fails the exact assertions.
fn exposed_assigned_tools(model: &Server) -> Vec<Value> {
    const PLATFORM_RUNTIME_TOOLS: &[&str] = &[
        "tool_search",
        "tool_execute",
        "agent_get",
        "agent_spawn",
        "child_answer",
        "child_cancel",
        "child_list",
        "child_read",
        "child_send",
        "child_wait",
        "thread_answer",
        "thread_close",
        "thread_create",
        "thread_list",
        "thread_read",
        "thread_send",
        "thread_wait",
        "skill_load",
        "skill_read_resource",
    ];
    exposed(model)
        .into_iter()
        .filter(|tool| {
            !tool["function"]["name"]
                .as_str()
                .is_some_and(|name| PLATFORM_RUNTIME_TOOLS.contains(&name))
        })
        .collect()
}
fn tool_outputs(model: &Server, name: &str) -> Vec<String> {
    let requests = model.requests.lock().unwrap();
    requests
        .iter()
        .flat_map(|request| {
            request["messages"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
        })
        .filter(|message| {
            if message["role"] != "tool" {
                return false;
            }
            let id = message["tool_call_id"].as_str().unwrap_or_default();
            let prefix = format!("call_{name}");
            id == prefix || (id.starts_with(&prefix) && id[prefix.len()..].starts_with('-'))
        })
        .map(|message| message["content"].as_str().unwrap().to_string())
        .collect()
}

fn tool_output(model: &Server, name: &str) -> String {
    tool_outputs(model, name)
        .into_iter()
        .next()
        .expect("native tool observation")
}

/// Parsed payloads of every `tool_search` observation, in call order.
fn search_outputs(model: &Server) -> Vec<Value> {
    tool_outputs(model, "tool_search")
        .iter()
        .flat_map(|output| {
            let value: Value = serde_json::from_str(output).unwrap();
            value["results"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["result"].clone())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn envelope(tool: &str, arguments: Value) -> (&'static str, Value) {
    (
        "tool_execute",
        json!({"tool":tool,"arguments":{"items":[arguments]}}),
    )
}

/// The stable gateway pair is the only model-visible tool surface.
fn assert_only_gateway_pair_exposed(model: &Server) {
    let mut names: Vec<String> = exposed(model)
        .iter()
        .filter_map(|tool| tool["function"]["name"].as_str())
        .map(str::to_string)
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec!["tool_execute".to_string(), "tool_search".to_string()],
        "{names:?}"
    );
    assert!(exposed_assigned_tools(model).is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn saved_empty_allowlist_blocks_forged_local_tools_at_native_execution() {
    let (app, authorization) = fixture(&[]).await;
    let temp = tempfile::tempdir().unwrap();
    let marker = temp.path().join("must-not-exist.txt");
    let model = model_server(vec![
        (
            "fs_write_text",
            json!({"path":marker,"content":"unauthorized"}),
        ),
        ("command_exec", json!({"command":"echo forbidden"})),
    ])
    .await;
    run(
        &app,
        &authorization,
        &model,
        vec![forged_tool("fs_write_text"), forged_tool("command_exec")],
    )
    .await;
    assert_only_gateway_pair_exposed(&model);
    assert!(
        !marker.exists(),
        "a forged DTO must not admit a local executor"
    );
    assert!(tool_output(&model, "fs_write_text").contains("is not allowed"));
    assert!(tool_output(&model, "command_exec").contains("is not allowed"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn saved_selected_local_tools_execute_through_the_gateway_pair() {
    let (app, authorization) = fixture(&["bash.exec", "fs.read_text"]).await;
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.txt");
    let marker = temp.path().join("denied.txt");
    std::fs::write(&source, "allowed read").unwrap();
    std::fs::write(temp.path().join("second.txt"), "second batched read").unwrap();
    let model = model_server(vec![
        envelope("command_exec", json!({"command":"echo native-alias-authorized"})),
        ("tool_execute",json!({"tool":"fs_read_text","arguments":{"items":[{"path":source},{"path":temp.path().join("second.txt")}]}})),
        // Direct inner-name calls bypass the gateway surface and must be
        // rejected even for saved tools.
        ("fs_write_text", json!({"path":marker,"content":"denied"})),
        (
            "command_exec",
            json!({"command":"echo canonical-name-must-not-expand-authority"}),
        ),
    ])
    .await;
    run(
        &app,
        &authorization,
        &model,
        vec![
            forged_tool("command_exec"),
            forged_tool("fs_read_text"),
            forged_tool("fs_write_text"),
            forged_tool("command_exec"),
        ],
    )
    .await;
    // Only the stable pair is model-visible; saved inner tools never appear
    // as request-level schemas, so forged request DTOs cannot leak in.
    assert_only_gateway_pair_exposed(&model);
    assert!(tool_output(&model, "command_exec").contains("native-alias-authorized"));
    let batch: Value = serde_json::from_str(&tool_output(&model, "fs_read_text")).unwrap();
    assert_eq!(batch["results"].as_array().unwrap().len(), 2);
    assert!(
        batch["results"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["isError"] == false)
    );
    assert!(tool_output(&model, "fs_read_text").contains("second batched read"));
    assert!(tool_output(&model, "fs_read_text").contains("allowed read"));
    assert!(!marker.exists());
    assert!(tool_output(&model, "fs_write_text").contains("is not allowed"));
    assert!(tool_output(&model, "command_exec").contains("is not allowed"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn nonempty_request_narrows_saved_local_authority_while_empty_uses_saved_policy() {
    let (app, authorization) = fixture(&["bash.exec", "fs.read_text"]).await;
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.txt");
    std::fs::write(&source, "read only").unwrap();
    // A nonempty request narrows authority to its intersection with the
    // saved selection: fs.read_text executes, bash.exec is denied.
    let narrowed = model_server(vec![
        envelope("fs_read_text", json!({"path":source})),
        envelope("command_exec", json!({"command":"echo must-be-denied"})),
    ])
    .await;
    run(
        &app,
        &authorization,
        &narrowed,
        vec![forged_tool("fs_read_text")],
    )
    .await;
    assert_only_gateway_pair_exposed(&narrowed);
    assert!(tool_output(&narrowed, "fs_read_text").contains("read only"));
    assert!(tool_output(&narrowed, "command_exec").contains("is not allowed"));
    // An empty request keeps the saved policy: both saved tools may execute.
    let saved = model_server(vec![(
        "tool_execute",
        json!({"tool":"command_exec","arguments":{"items":[{"command":"echo empty-request-keeps-saved-policy"}]}}),
    )])
    .await;
    run(&app, &authorization, &saved, vec![]).await;
    assert_only_gateway_pair_exposed(&saved);
    assert!(tool_output(&saved, "command_exec").contains("empty-request-keeps-saved-policy"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn gateway_search_and_envelopes_cannot_authorize_tools_outside_saved_selection() {
    let (app, authorization) = fixture(&["bash.exec"]).await;
    let temp = tempfile::tempdir().unwrap();
    let marker = temp.path().join("denied.txt");
    // Discovery round: search for saved and unsaved tools alike.
    let model = model_server_rounds(vec![
        vec![
            ("tool_search", json!({"items":[{"tool":"command_exec"}]})),
            ("tool_search", json!({"items":[{"query":"write text"}]})),
        ],
        vec![
            envelope("command_exec", json!({"command":"echo gateway-authorized"})),
            envelope("fs_write_text", json!({"path":marker,"content":"denied"})),
        ],
    ])
    .await;
    // The request carries forged DTOs; only its intersection with the saved
    // selection (bash.exec) stays authorized.
    run(
        &app,
        &authorization,
        &model,
        vec![forged_tool("fs_write_text"), forged_tool("command_exec")],
    )
    .await;
    assert_only_gateway_pair_exposed(&model);
    // Search returns only saved-selection descriptors, with the real catalog
    // schema — never the forged request DTO.
    let searches = search_outputs(&model);
    let bash_results = &searches[0];
    let bash_tools: Vec<&str> = bash_results["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|tool| tool["call_name"].as_str())
        .collect();
    assert!(bash_tools.contains(&"command_exec"), "{bash_results}");
    let bash_descriptor = bash_results["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["call_name"] == "command_exec")
        .unwrap();
    assert_ne!(bash_descriptor["description"], "FORGED DESCRIPTION");
    assert!(
        bash_descriptor["input_schema"]["properties"]["items"]["items"]["properties"]["command"]
            .is_object()
    );
    assert!(
        bash_descriptor["input_schema"]["properties"]["items"]["items"]["properties"]["forged"]
            .is_null()
    );
    let write_results = &searches[1];
    assert!(
        write_results["tools"].as_array().unwrap().is_empty(),
        "unsaved tools must not be discoverable: {write_results}"
    );
    // Envelope execution still enforces the saved selection.
    let bash_output = tool_output(&model, "command_exec");
    assert!(bash_output.contains("gateway-authorized"), "{bash_output}");
    assert!(!marker.exists());
    assert!(tool_output(&model, "fs_write_text").contains("is not allowed"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dynamic_mcp_respects_disabled_empty_selected_and_request_intersection() {
    let (app, authorization) = fixture(&[]).await;
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
        assert_only_gateway_pair_exposed(&denied);
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
        (
            eliteforge_agent_core::TOOL_SEARCH_CALL_NAME,
            json!({"items":[{"domainCode":"mcp.selected_tools","limit":20}]}),
        ),
        envelope("mcp_selected_tools_search_docs", json!({"query":"allowed"})),
        envelope("mcp_other_tools_search_docs", json!({"query":"unselected"})),
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
    assert_only_gateway_pair_exposed(&selected);
    let prompt = selected.requests.lock().unwrap()[0]["messages"].to_string();
    assert!(
        prompt.contains("mcp.selected_tools"),
        "MCP group must be advertised in the initial prompt: {prompt}"
    );
    assert!(!prompt.contains("FORGED DESCRIPTION"));
    let discovered = search_outputs(&selected);
    assert!(
        discovered
            .iter()
            .any(|output| output["tools"].as_array().is_some_and(|tools| {
                tools
                    .iter()
                    .any(|tool| tool["call_name"] == "mcp_selected_tools_search_docs")
            })),
        "the advertised MCP group must be searchable: {discovered:?}"
    );
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
        envelope("mcp_selected_tools_search_docs", json!({"query":"allowed"})),
        envelope(
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
    assert_only_gateway_pair_exposed(&narrowed);
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
    assert!(exposed_assigned_tools(&disabled).is_empty());
    assert!(tool_output(&disabled, "mcp_selected_tools_search_docs").contains("is not allowed"));
    assert_eq!(selected_server.requests.lock().unwrap().len(), calls_before);
}

#[tokio::test]
async fn disabling_tool_extension_removes_gateway_tools_and_guidance() {
    let (app, authorization) = fixture(&[]).await;
    for enabled in [false, true] {
        api(
            &app,
            &authorization,
            Method::PUT,
            "/api/v1/capabilities/extensions/tool/enabled",
            json!({"enabled":enabled}),
        )
        .await;
        let model = model_server_rounds(vec![]).await;
        run(&app, &authorization, &model, vec![]).await;
        let prompt = model.requests.lock().unwrap()[0]["messages"].to_string();
        if enabled {
            assert_only_gateway_pair_exposed(&model);
            assert!(prompt.contains(eliteforge_agent_core::TOOL_SEARCH_CALL_NAME));
        } else {
            assert!(exposed(&model).is_empty());
            assert!(
                !prompt.contains(eliteforge_agent_core::TOOL_SEARCH_CALL_NAME),
                "disabled gateway guidance leaked: {prompt}"
            );
            assert!(
                !prompt.contains(eliteforge_agent_core::TOOL_EXECUTE_CALL_NAME),
                "disabled execution guidance leaked: {prompt}"
            );
        }
    }
}
