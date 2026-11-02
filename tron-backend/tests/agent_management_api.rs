mod common;

use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    body::Body,
    http::{Method, Request, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};
use tower::ServiceExt;
use tron_backend::config::{AppConfig, SystemAgentConfig};

use common::{authenticated_bearer_token, request_with_headers, response_json};

// Agent tests have no ingestion jobs. Stop unrelated background workers before
// they can block a single-thread test runtime on the shared SQLite transaction.
fn app() -> Router {
    app_with_config(AppConfig::default())
}
fn app_with_config(config: AppConfig) -> Router {
    common::app_with_state_setup(common::prepared_config(config), |state| {
        state.knowledge_bases.stop_ingestion_workers();
        // API fixtures use the production-style multi-thread Tokio runtime: real
        // AppState background services can block on SQLite while an HTTP run
        // needs another task to finish its transaction.
    })
}

// Associate native provider results with their exact assistant call, rather than
// treating tool output as a synthetic user message containing a tool-name prefix.
fn provider_tool_result(messages: &[Value], name: &str) -> String {
    // Tool execution goes through the `tool_execute` gateway: the assistant
    // call is the envelope and the inner tool is addressed by its arguments.
    let calls: Vec<_> = messages
        .iter()
        .filter(|message| message["role"] == json!("assistant"))
        .filter_map(|message| message["tool_calls"].as_array())
        .flatten()
        .filter(|call| {
            // `tool_search` is addressed directly; wrapped tools run through
            // the `tool_execute` envelope with the inner name in arguments.
            if call["function"]["name"] == json!(name)
                && (name == "tool_search" || name == "tool_execute")
            {
                return true;
            }
            call["function"]["name"] == json!("tool_execute")
                && call["function"]["arguments"]
                    .as_str()
                    .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
                    .and_then(|arguments| arguments["tool"].as_str().map(str::to_string))
                    .as_deref()
                    == Some(name)
        })
        .collect();
    assert_eq!(
        calls.len(),
        1,
        "expected one provider call for {name}: {messages:?}"
    );
    let id = calls[0]["id"]
        .as_str()
        .expect("provider call must have an ID");
    assert_eq!(id, format!("call_{name}"));
    let results: Vec<_> = messages
        .iter()
        .filter(|message| message["role"] == json!("tool") && message["tool_call_id"] == json!(id))
        .collect();
    assert_eq!(
        results.len(),
        1,
        "expected one native result for {name}/{id}: {messages:?}"
    );
    common::single_gateway_result(
        results[0]["content"]
            .as_str()
            .expect("tool output must be text"),
    )
}

struct RecordingModelServer {
    base_url: String,
    requests: Arc<Mutex<Vec<Value>>>,
    _task: JoinHandle<()>,
}

struct ScriptedModelServer {
    base_url: String,
    requests: Arc<Mutex<Vec<Value>>>,
    _task: JoinHandle<()>,
}

struct BlockingMcpServer {
    base_url: String,
    requests: Arc<Mutex<Vec<Value>>>,
    release_call: Arc<tokio::sync::Notify>,
    tool_call_started: tokio::sync::oneshot::Receiver<()>,
    _task: JoinHandle<()>,
}

static DB_SEQUENCE: AtomicU64 = AtomicU64::new(0);

async fn recording_model_server() -> RecordingModelServer {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("agent mock server should bind");
    let address = listener
        .local_addr()
        .expect("agent mock server should expose address");
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let app = Router::new()
        .route("/v1/chat/completions", post(record_chat_completion))
        .with_state(requests.clone());
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("agent mock server should stay available");
    });
    RecordingModelServer {
        base_url: format!("http://{address}/v1"),
        requests,
        _task: task,
    }
}

async fn scripted_model_server(responses: Vec<Value>) -> ScriptedModelServer {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("agent scripted mock server should bind");
    let address = listener
        .local_addr()
        .expect("agent scripted mock server should expose address");
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
    let app = Router::new()
        .route("/v1/chat/completions", post(scripted_chat_completion))
        .with_state((requests.clone(), responses));
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("agent scripted mock server should stay available");
    });
    ScriptedModelServer {
        base_url: format!("http://{address}/v1"),
        requests,
        _task: task,
    }
}

async fn blocking_mcp_server(tool_name: &str, tool_output: &str) -> BlockingMcpServer {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("blocking MCP server should bind");
    let address = listener
        .local_addr()
        .expect("blocking MCP server should expose address");
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let release_call = Arc::new(tokio::sync::Notify::new());
    let (tool_call_started_sender, tool_call_started) = tokio::sync::oneshot::channel();
    let state = Arc::new(BlockingMcpServerState {
        requests: requests.clone(),
        tool_name: tool_name.to_string(),
        tool_output: tool_output.to_string(),
        release_call: release_call.clone(),
        tool_call_started: Mutex::new(Some(tool_call_started_sender)),
    });
    let app = Router::new()
        .route("/mcp", post(blocking_mcp_transport))
        .with_state(state);
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("blocking MCP server should stay available");
    });
    BlockingMcpServer {
        base_url: format!("http://{address}/mcp"),
        requests,
        release_call,
        tool_call_started,
        _task: task,
    }
}

struct BlockingMcpServerState {
    requests: Arc<Mutex<Vec<Value>>>,
    tool_name: String,
    tool_output: String,
    release_call: Arc<tokio::sync::Notify>,
    tool_call_started: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
}

async fn blocking_mcp_transport(
    axum::extract::State(state): axum::extract::State<Arc<BlockingMcpServerState>>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    state
        .requests
        .lock()
        .expect("blocking MCP request mutex poisoned")
        .push(payload.clone());
    let method = payload
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let id = payload.get("id").cloned().unwrap_or(Value::Null);
    match method {
        "initialize" => Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2025-06-18",
                "serverInfo": {
                    "name": "test-mcp",
                    "version": "1.0.0"
                },
                "capabilities": {
                    "tools": {}
                }
            }
        })),
        "tools/list" => Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [{
                    "name": state.tool_name,
                    "description": "Search MCP knowledge",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string" }
                        }
                    }
                }]
            }
        })),
        "tools/call" => {
            if let Some(sender) = state
                .tool_call_started
                .lock()
                .expect("blocking MCP started mutex poisoned")
                .take()
            {
                let _ = sender.send(());
            }
            state.release_call.notified().await;
            Json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{
                        "type": "text",
                        "text": state.tool_output
                    }]
                }
            }))
        }
        _ => Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": "unsupported method"
            }
        })),
    }
}

// The Framework requests streaming completions. Keep fixture transport faithful
// while reusing the same scripted message payloads for non-streaming callers.
fn model_completion(payload: &Value, response: Value) -> Response {
    if payload["stream"] != true || !response["choices"].is_array() {
        return Json(response).into_response();
    }
    let message = &response["choices"][0]["message"];
    let mut delta = message.clone();
    if let Some(calls) = delta["tool_calls"].as_array_mut() {
        for (index, call) in calls.iter_mut().enumerate() {
            call["index"] = json!(index);
        }
    }
    let finish = response["choices"][0]["finish_reason"].as_str().unwrap_or(
        if message["tool_calls"].is_array() {
            "tool_calls"
        } else {
            "stop"
        },
    );
    let chunk = json!({"choices":[{"index":0,"delta":delta,"finish_reason":null}]});
    let done = json!({"choices":[{"index":0,"delta":{},"finish_reason":finish}],"usage":response["usage"]});
    (
        [(header::CONTENT_TYPE, "text/event-stream")],
        format!("data: {chunk}\n\ndata: {done}\n\ndata: [DONE]\n\n"),
    )
        .into_response()
}

async fn record_chat_completion(
    axum::extract::State(requests): axum::extract::State<Arc<Mutex<Vec<Value>>>>,
    Json(payload): Json<Value>,
) -> Response {
    requests
        .lock()
        .expect("recording model request mutex poisoned")
        .push(payload.clone());
    let prompt = payload["messages"]
        .as_array()
        .and_then(|messages| messages.last())
        .and_then(|message| message["content"].as_str())
        .unwrap_or_default();
    model_completion(
        &payload,
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": format!("mock: {prompt}")
                }
            }]
        }),
    )
}

async fn scripted_chat_completion(
    axum::extract::State((requests, responses)): axum::extract::State<(
        Arc<Mutex<Vec<Value>>>,
        Arc<Mutex<VecDeque<Value>>>,
    )>,
    Json(payload): Json<Value>,
) -> Response {
    requests
        .lock()
        .expect("recording model request mutex poisoned")
        .push(payload.clone());
    let response = responses
        .lock()
        .expect("scripted model response mutex poisoned")
        .pop_front()
        .unwrap_or_else(|| {
            json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "scripted fallback"
                    }
                }]
            })
        });
    model_completion(&payload, to_gateway_response(response))
}

/// The product exposes exactly the stable `tool_search`/`tool_execute` pair,
/// so scripted direct tool calls are rewritten into `tool_execute` envelopes.
/// The inner tool keeps its identity through the call id so observations stay
/// individually addressable. Already-gateway calls pass through unchanged.
fn to_gateway_response(mut response: Value) -> Value {
    let Some(calls) = response
        .pointer_mut("/choices/0/message/tool_calls")
        .and_then(|calls| calls.as_array_mut())
    else {
        return response;
    };
    for call in calls.iter_mut() {
        let is_gateway = call
            .pointer("/function/name")
            .and_then(|name| name.as_str())
            .is_some_and(|name| name == "tool_execute" || name == "tool_search");
        if is_gateway {
            let name = call["function"]["name"].as_str().unwrap();
            let raw = call["function"]["arguments"].as_str().unwrap_or("{}");
            let input: Value = serde_json::from_str(raw).unwrap();
            let input = if name == "tool_search" {
                common::gateway_arguments(name, input)
            } else {
                let envelope = eliteforge_agent_core::GatewayEnvelope::parse(&input).unwrap();
                json!({"tool":envelope.tool,"arguments":common::gateway_arguments(&envelope.tool,envelope.arguments)})
            };
            call["function"]["arguments"] = json!(input.to_string());
            continue;
        }
        let inner_name = call
            .pointer("/function/name")
            .and_then(|name| name.as_str().unwrap_or_default().to_string().into())
            .unwrap_or_else(|| "unknown".into());
        let inner_arguments = call
            .pointer("/function/arguments")
            .and_then(|raw| raw.as_str())
            .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
            .unwrap_or_else(|| json!({}));
        let call_id = call
            .get("id")
            .and_then(|id| id.as_str())
            .map(|id| id.to_string())
            .unwrap_or_else(|| format!("call_{inner_name}"));
        let envelope = json!({
            "id": call_id,
            "type": "function",
            "function": {
                "name": "tool_execute",
                "arguments": json!({"tool": inner_name, "arguments": common::gateway_arguments(&inner_name,inner_arguments)}).to_string(),
            },
        });
        *call = envelope;
    }
    response
}

fn temp_database_url(test_name: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    let sequence = DB_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir()
        .join(format!(
            "tron-agent-mgmt-{test_name}-{nanos}-{sequence}.sqlite3"
        ))
        .display()
        .to_string()
}

fn temp_command_path(command_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    let file_name = format!("tron-agent-mgmt-{command_name}-{nanos}");
    #[cfg(target_os = "windows")]
    let file_name = format!("{file_name}.cmd");
    std::env::temp_dir().join(file_name)
}

fn temp_workspace_dir(name: &str) -> PathBuf {
    let path = temp_command_path(name);
    std::fs::create_dir_all(&path).expect("temp workspace directory should be created");
    path
}

fn write_fake_command(path: &PathBuf) {
    write_executable_script(path, "#!/bin/sh\nexit 0\n");
}

fn write_executable_script(path: &PathBuf, script: &str) {
    #[cfg(target_os = "windows")]
    {
        let output = script.lines().map(str::trim).find_map(|line| {
            line.strip_prefix("echo '")
                .and_then(|value| value.strip_suffix('\''))
        });
        let batch_file = match output {
            Some(value) => format!("@echo off\r\necho {value}\r\nexit /b 0\r\n"),
            None => "@echo off\r\nexit /b 0\r\n".to_string(),
        };
        std::fs::write(path, batch_file).expect("fake command should be written");
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(path, script).expect("fake command should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("fake command metadata should exist")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("fake command should be executable");
    }
}

async fn request_with_locale(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    authorization_header: &str,
    locale: &str,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, authorization_header)
        .header(header::ACCEPT_LANGUAGE, locale);
    if let Some(workspace) = body
        .as_ref()
        .and_then(|body| body["workspaceCode"].as_str())
    {
        builder = builder.header("x-workspace-code", workspace);
    }
    let request_body = if let Some(json_body) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(json_body.to_string())
    } else {
        Body::empty()
    };

    app.clone()
        .oneshot(
            builder
                .body(request_body)
                .expect("request should be constructible"),
        )
        .await
        .expect("router should return a response")
}

fn test_config(codex_command: &PathBuf, opencode_command: &PathBuf) -> AppConfig {
    AppConfig {
        database_url: temp_database_url("agent-management"),
        system_agents: SystemAgentConfig {
            codex_command: codex_command.display().to_string(),
            opencode_command: opencode_command.display().to_string(),
            dsh_command: String::new(),
        },
        ..AppConfig::default()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn startup_scan_only_retains_available_system_agents() {
    let codex_command = temp_command_path("codex");
    let opencode_command = temp_command_path("opencode");
    write_fake_command(&codex_command);
    write_fake_command(&opencode_command);

    let app = app_with_config(test_config(&codex_command, &opencode_command));
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let platform_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/agents/registry/system",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(platform_response.status(), StatusCode::OK);
    let platform_body = response_json(platform_response).await;
    assert_eq!(platform_body["code"], json!("SUCCESS"));
    let platform_rows = platform_body["data"]
        .as_array()
        .expect("platform rows should be an array");
    assert_eq!(platform_rows.len(), 2);
    assert!(
        platform_rows
            .iter()
            .all(|row| row["status"] == json!("ENABLED"))
    );

    let workspace_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/agents/workspaces/TRON_CORE/system",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(workspace_response.status(), StatusCode::OK);
    let workspace_body = response_json(workspace_response).await;
    let workspace_rows = workspace_body["data"]
        .as_array()
        .expect("workspace rows should be an array");
    assert_eq!(workspace_rows.len(), 2);
    assert!(
        workspace_rows
            .iter()
            .all(|row| row["agentType"] == json!("SYSTEM"))
    );
    assert!(
        workspace_rows
            .iter()
            .all(|row| row["inheritedFrom"] == json!("Platform"))
    );
    assert!(
        workspace_rows
            .iter()
            .all(|row| row["agentCode"] != json!("main-agent"))
    );

    let disable_workspace = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/workspaces/TRON_CORE/system/codex/disable",
        Some(json!({})),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(disable_workspace.status(), StatusCode::OK);
    let disabled_workspace_row = response_json(disable_workspace).await;
    assert_eq!(disabled_workspace_row["data"]["status"], json!("DISABLED"));
    assert_eq!(
        disabled_workspace_row["data"]["platformEnabled"],
        json!(true)
    );
    assert_eq!(
        disabled_workspace_row["data"]["workspaceOverrideDisabled"],
        json!(true)
    );

    let platform_after_workspace_disable = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/agents/registry/system",
        None,
        None,
        Some(&authorization),
    )
    .await;
    let platform_after_workspace_disable_body =
        response_json(platform_after_workspace_disable).await;
    let codex_platform_row = platform_after_workspace_disable_body["data"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["agentCode"] == json!("codex")))
        .expect("codex platform row should exist");
    assert_eq!(codex_platform_row["status"], json!("ENABLED"));

    let enable_workspace = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/workspaces/TRON_CORE/system/codex/enable",
        Some(json!({})),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(enable_workspace.status(), StatusCode::OK);
    let enabled_workspace_row = response_json(enable_workspace).await;
    assert_eq!(enabled_workspace_row["data"]["status"], json!("ENABLED"));
    assert_eq!(
        enabled_workspace_row["data"]["platformEnabled"],
        json!(true)
    );
    assert_eq!(
        enabled_workspace_row["data"]["workspaceOverrideDisabled"],
        json!(false)
    );

    let disable_platform = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/registry/system/codex/disable",
        Some(json!({})),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(disable_platform.status(), StatusCode::OK);
    let disabled_platform_row = response_json(disable_platform).await;
    assert_eq!(disabled_platform_row["data"]["status"], json!("DISABLED"));

    let workspace_after_platform_disable = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/agents/workspaces/TRON_CORE/system",
        None,
        None,
        Some(&authorization),
    )
    .await;
    let workspace_after_platform_disable_body =
        response_json(workspace_after_platform_disable).await;
    let codex_workspace_row = workspace_after_platform_disable_body["data"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["agentCode"] == json!("codex")))
        .expect("codex workspace row should exist");
    assert_eq!(codex_workspace_row["status"], json!("DISABLED"));
    assert_eq!(codex_workspace_row["platformEnabled"], json!(false));
    assert_eq!(
        codex_workspace_row["workspaceOverrideDisabled"],
        json!(false)
    );

    std::fs::remove_file(&codex_command).expect("fake codex command should be removable");
    let rescan_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/registry/system/rescan",
        Some(json!({})),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(rescan_response.status(), StatusCode::OK);
    let rescan_body = response_json(rescan_response).await;
    assert_eq!(rescan_body["data"]["scannedCount"], json!(2));
    assert_eq!(rescan_body["data"]["unavailableCount"], json!(1));

    let platform_after_rescan = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/agents/registry/system",
        None,
        None,
        Some(&authorization),
    )
    .await;
    let platform_after_rescan_body = response_json(platform_after_rescan).await;
    let platform_rows_after_rescan = platform_after_rescan_body["data"]
        .as_array()
        .expect("platform rows should be an array");
    assert_eq!(platform_rows_after_rescan.len(), 1);
    assert!(
        platform_rows_after_rescan
            .iter()
            .all(|row| row["agentCode"] != json!("codex"))
    );

    let workspace_after_rescan = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/agents/workspaces/TRON_CORE/system",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(workspace_after_rescan.status(), StatusCode::OK);
    let workspace_rows_after_rescan = response_json(workspace_after_rescan).await;
    assert_eq!(
        workspace_rows_after_rescan["data"].as_array().map(Vec::len),
        Some(1)
    );

    let removed_workspace_mutation = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/workspaces/TRON_CORE/system/codex/disable",
        Some(json!({})),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(removed_workspace_mutation.status(), StatusCode::NOT_FOUND);
    let removed_workspace_body = response_json(removed_workspace_mutation).await;
    assert_eq!(
        removed_workspace_body["code"],
        json!("AGENT_SYSTEM_NOT_FOUND")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn startup_scan_loads_system_agent_models_and_rescan_refreshes_models() {
    let codex_command = temp_command_path("codex-models");
    let opencode_command = temp_command_path("opencode-models");
    write_executable_script(
        &codex_command,
        "#!/bin/sh\nif [ \"$1\" = \"models\" ]; then\n  echo '[{\"id\":\"gpt-5-codex\",\"displayName\":\"GPT-5 Codex\"}]'\n  exit 0\nfi\nexit 0\n",
    );
    write_executable_script(
        &opencode_command,
        "#!/bin/sh\nif [ \"$1\" = \"models\" ]; then\n  echo '{\"models\":[{\"id\":\"openai/gpt-4.1\",\"displayName\":\"OpenAI GPT-4.1\"}]}'\n  exit 0\nfi\nexit 0\n",
    );
    let app = app_with_config(test_config(&codex_command, &opencode_command));
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let startup_platform_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/agents/registry/system",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(startup_platform_response.status(), StatusCode::OK);
    let startup_platform_body = response_json(startup_platform_response).await;
    let startup_rows = startup_platform_body["data"]
        .as_array()
        .expect("platform rows should be an array");
    let startup_codex = startup_rows
        .iter()
        .find(|row| row["agentCode"] == json!("codex"))
        .expect("codex row should exist after startup scan");
    assert_eq!(
        startup_codex["models"],
        json!([{ "modelId": "gpt-5-codex", "displayName": "GPT-5 Codex", "supportedReasoningEfforts": [] }])
    );

    write_executable_script(
        &codex_command,
        "#!/bin/sh\nif [ \"$1\" = \"models\" ]; then\n  echo '[{\"id\":\"gpt-5-codex-next\",\"displayName\":\"GPT-5 Codex Next\"}]'\n  exit 0\nfi\nexit 0\n",
    );

    let rescan_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/registry/system/rescan",
        Some(json!({})),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(rescan_response.status(), StatusCode::OK);
    let rescan_body = response_json(rescan_response).await;
    let rescan_rows = rescan_body["data"]["records"]
        .as_array()
        .expect("rescan rows should be an array");
    let rescan_codex = rescan_rows
        .iter()
        .find(|row| row["agentCode"] == json!("codex"))
        .expect("codex row should exist after rescan");
    assert_eq!(
        rescan_codex["models"],
        json!([{ "modelId": "gpt-5-codex-next", "displayName": "GPT-5 Codex Next", "supportedReasoningEfforts": [] }])
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn platform_main_agent_guidance_is_editable_and_legacy_storage_still_drives_runs() {
    let codex_command = temp_command_path("codex-main");
    let opencode_command = temp_command_path("opencode-main");
    write_fake_command(&codex_command);
    write_fake_command(&opencode_command);

    let model_server = recording_model_server().await;
    let config = common::prepared_config(test_config(&codex_command, &opencode_command));
    let app = app_with_config(config.clone());
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let removed = request_with_headers(
        &app,
        Method::PATCH,
        "/api/v1/agents/registry/main-agent",
        Some(json!({"personality":"Legacy personality","instructions":"Legacy instructions"})),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(removed.status(), StatusCode::NOT_FOUND);
    let guidance = "Calm, terse, and surgical. Summarize the workspace problem before acting.";
    let denied = request_with_locale(
        &app,
        Method::PATCH,
        "/api/v1/agents/registry/internal/main-agent/definition",
        Some(json!({"rolePrompt":guidance})),
        &authorization,
        "en-US",
    )
    .await;
    assert_eq!(denied.status(), StatusCode::OK);
    let mut legacy = internal_agent_api(
        &app,
        &authorization,
        Method::GET,
        "/api/v1/agents/registry/internal/main-agent",
        None,
    )
    .await;
    legacy["rolePrompt"] = json!(guidance);
    let connection = rusqlite::Connection::open(&config.database_url).unwrap();
    connection.execute("INSERT INTO platform_internal_agent_configs(agent_code, config_json) VALUES('main-agent',?1) ON CONFLICT(agent_code) DO UPDATE SET config_json=excluded.config_json", [legacy.to_string()]).unwrap();
    let saved = internal_agent_api(
        &app,
        &authorization,
        Method::GET,
        "/api/v1/agents/registry/internal/main-agent",
        None,
    )
    .await;
    assert_eq!(saved["rolePrompt"], guidance);

    let main_run = request_with_locale(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({
            "prompt": "Investigate latency.",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "TEST_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": model_server.base_url,
                "apiKey": "sk-test-api",
                "modelName": "gpt-test"
            }
        })),
        &authorization,
        "en-US",
    )
    .await;
    assert_eq!(main_run.status(), StatusCode::OK);

    let task_run = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({
            "prompt": "Investigate latency for task.",
            "workspaceCode": "TRON_CORE",
            "taskCode": "TASK-1",
            "model": {
                "modelCode": "TEST_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": model_server.base_url,
                "apiKey": "sk-test-api",
                "modelName": "gpt-test"
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(task_run.status(), StatusCode::OK);

    let recorded_requests = model_server
        .requests
        .lock()
        .expect("recorded model request mutex poisoned")
        .clone();
    assert_eq!(recorded_requests.len(), 2);

    let main_messages = recorded_requests[0]["messages"]
        .as_array()
        .expect("main-agent messages should be an array");
    // Discovery and agent-directory context may add messages. Check the
    // guidance contract instead of fixing the total message count.
    assert_eq!(main_messages[0]["role"], json!("system"));
    let main_guidance = main_messages[0]["content"]
        .as_str()
        .expect("main-agent guidance should be text");
    assert!(main_guidance.contains("Calm, terse, and surgical."));
    assert!(main_guidance.contains("Summarize the workspace problem before acting."));
    let request_text = serde_json::to_string(main_messages).unwrap();
    for removed in [
        "Final assistant output contract:",
        "最终答复规范：",
        "SYS_AGENT_FINAL_OUTPUT",
        "agent.final-output",
    ] {
        assert!(
            !request_text.contains(removed),
            "retired prompt leaked: {removed}"
        );
    }
    assert!(
        main_messages.iter().any(
            |message| message["role"] == "user" && message["content"] == "Investigate latency."
        )
    );

    let task_messages = recorded_requests[1]["messages"]
        .as_array()
        .expect("task-agent messages should be an array");
    assert!(
        serde_json::to_string(task_messages)
            .unwrap()
            .contains("Calm, terse, and surgical.")
    );
    assert!(task_messages.iter().any(|message| message["role"] == "user"
        && message["content"] == "Investigate latency for task."));

    legacy["rolePrompt"] = json!("");
    connection.execute("UPDATE platform_internal_agent_configs SET config_json=?1 WHERE agent_code='main-agent'", [legacy.to_string()]).unwrap();
    let cleared = request_with_locale(
        &app, Method::POST, "/api/v1/agents/runs",
        Some(json!({"prompt":"Check blank instructions.","workspaceCode":"TRON_CORE",
            "model":{"modelCode":"TEST_MODEL","protocol":"CHAT_COMPLETIONS","baseUrl":model_server.base_url,
            "apiKey":"sk-test-api","modelName":"gpt-test"}})),
        &authorization, "en-US",
    ).await;
    assert_eq!(cleared.status(), StatusCode::OK);
    let requests = model_server.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 3);
    let messages = serde_json::to_string(&requests[2]["messages"]).unwrap();
    assert!(!messages.contains(guidance));
    assert!(!messages.contains("Legacy personality"));
    assert!(!messages.contains("Legacy instructions"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn internal_agent_domain_discovery_expands_and_executes_workspace_tools() {
    let codex_command = temp_command_path("codex-domain-discovery");
    let opencode_command = temp_command_path("opencode-domain-discovery");
    write_fake_command(&codex_command);
    write_fake_command(&opencode_command);

    let model_server = scripted_model_server(vec![
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_tool_search",
                        "type": "function",
                        "function": {
                            "name": "tool_search",
                            "arguments": "{\"query\":\"workspace\"}"
                        }
                    }]
                }
            }]
        }),
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_workspace_page",
                        "type": "function",
                        "function": {
                            "name": "workspace_page",
                            "arguments": "{\"workspaceCode\":\"TRON_CORE\"}"
                        }
                    }]
                }
            }]
        }),
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "expanded and executed"
                }
            }]
        }),
    ])
    .await;
    let app = app_with_config(test_config(&codex_command, &opencode_command));
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let patch_policy = request_with_headers(
        &app,
        Method::PATCH,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/policy",
        Some(json!({})),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(patch_policy.status(), StatusCode::OK);

    let run_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({
            "prompt": "Inspect the workspace catalog.",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "TEST_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": model_server.base_url,
                "apiKey": "sk-test-api",
                "modelName": "gpt-test"
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    let run_status = run_response.status();
    let run_body = response_json(run_response).await;
    assert_eq!(run_status, StatusCode::OK, "{run_body}");
    assert_eq!(
        run_body["data"]["finalOutput"],
        json!("expanded and executed")
    );

    let recorded_requests = model_server
        .requests
        .lock()
        .expect("recorded scripted model request mutex poisoned")
        .clone();
    assert_eq!(recorded_requests.len(), 3);

    // The gateway pair is the complete model-visible catalog for the whole
    // run: request 0 before any tool ran, request 1 after discovery, request
    // 2 after execution — all identical, cache-stable by construction.
    for (index, request) in recorded_requests.iter().enumerate() {
        let tools = request["tools"]
            .as_array()
            .unwrap_or_else(|| panic!("request {index} tools should be an array"));
        let names: Vec<_> = tools
            .iter()
            .map(|tool| tool["function"]["name"].clone())
            .collect();
        assert_eq!(
            names,
            vec![json!("tool_execute"), json!("tool_search")],
            "request {index} must expose exactly the gateway pair"
        );
    }
    let first_messages = recorded_requests[0]["messages"]
        .as_array()
        .expect("first request messages should be an array");
    assert!(
        !first_messages.iter().any(|message| {
            message["role"] == json!("system")
                && message["content"].as_str().is_some_and(|content| {
                    content.contains("never call hidden tools directly before expanding")
                        || content.contains("不要在展开领域前直接调用隐藏工具")
                })
        }),
        "the discovery-era instruction is retired with the gateway"
    );
    let second_messages = recorded_requests[1]["messages"]
        .as_array()
        .expect("second request messages should be an array");
    // Gateway discovery: the search result is a tool OUTPUT listing inventory
    // descriptors; the schema of every request stays the stable pair.
    let search_result = provider_tool_result(second_messages, "tool_search");
    let search: Value = serde_json::from_str(&search_result).expect("search result JSON");
    assert!(
        search["tools"]
            .as_array()
            .is_some_and(|tools| !tools.is_empty()),
        "search should return inventory descriptors: {search_result}"
    );
    assert!(
        search["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["call_name"] == json!("workspace_page")),
        "workspace_page should be discoverable: {search_result}"
    );

    let third_messages = recorded_requests[2]["messages"]
        .as_array()
        .expect("third request messages should be an array");
    let workspace_result: Value =
        serde_json::from_str(&provider_tool_result(third_messages, "workspace_page"))
            .expect("workspace page result JSON");
    assert!(
        workspace_result["records"]
            .as_array()
            .is_some_and(|records| !records.is_empty())
    );
}

/// The agent must know the scope of its abilities: the run instructions list
/// the authorized capability groups and never a domain the policy did not
/// grant, and `domainCode` search then lists exactly that group.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn run_instructions_list_only_authorized_tool_domains_and_search_stays_in_domain() {
    let codex_command = temp_command_path("codex-domain-inventory");
    let opencode_command = temp_command_path("opencode-domain-inventory");
    write_fake_command(&codex_command);
    write_fake_command(&opencode_command);

    let model_server = scripted_model_server(vec![
        // List one capability group by its domainCode; matching is case-insensitive.
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_tool_search",
                        "type": "function",
                        "function": {
                            "name": "tool_search",
                            "arguments": "{\"domainCode\":\"HARNESS_WORKSPACE\",\"limit\":20}"
                        }
                    }]
                }
            }]
        }),
        // The keyword path still discovers the same wrapped inventory.
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_tool_search",
                        "type": "function",
                        "function": {
                            "name": "tool_search",
                            "arguments": "{\"query\":\"workspace\"}"
                        }
                    }]
                }
            }]
        }),
        // Execute a tool discovered through the group listing.
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_workspace_page",
                        "type": "function",
                        "function": {
                            "name": "workspace_page",
                            "arguments": "{\"workspaceCode\":\"TRON_CORE\"}"
                        }
                    }]
                }
            }]
        }),
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "inventory verified"
                }
            }]
        }),
    ])
    .await;
    let app = app_with_config(test_config(&codex_command, &opencode_command));
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    // Narrow the saved policy to the workspace domain: `app` stays in the
    // catalog but is not granted to this run.
    let patch_policy = request_with_headers(
        &app,
        Method::PATCH,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/policy",
        Some(json!({ "toolAssignmentMode": "selected_tools" })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(patch_policy.status(), StatusCode::OK);
    let assign = request_with_headers(
        &app,
        Method::PUT,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/tool-assignments",
        Some(json!({ "toolCodes": ["workspace.page", "workspace.get"] })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(assign.status(), StatusCode::OK);
    assert_eq!(
        response_json(assign).await["data"]["selectedToolCodes"],
        // Assignment codes are normalized by sorting.
        json!(["workspace.get", "workspace.page"])
    );

    let model_url = model_server.base_url.clone();
    let run_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({
            "prompt": "List the tools you can reach.",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "TEST_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": model_url,
                "apiKey": "sk-test-api",
                "modelName": "gpt-test"
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    let run_status = run_response.status();
    let run_body = response_json(run_response).await;
    assert_eq!(run_status, StatusCode::OK, "{run_body}");
    assert_eq!(
        run_body["data"]["finalOutput"],
        json!("inventory verified"),
        "{run_body}"
    );

    let recorded = model_server
        .requests
        .lock()
        .expect("recorded model requests mutex poisoned")
        .clone();
    assert_eq!(recorded.len(), 4, "one request per model step");

    // The model-visible list is the stable pair on every single step.
    for (index, request) in recorded.iter().enumerate() {
        let names: Vec<Value> = request["tools"]
            .as_array()
            .unwrap_or_else(|| panic!("request {index} tools should be an array"))
            .iter()
            .map(|tool| tool["function"]["name"].clone())
            .collect();
        assert_eq!(
            names,
            vec![json!("tool_execute"), json!("tool_search")],
            "request {index} must expose exactly the gateway pair"
        );
    }

    // The inventory reaches the model before its first decision: the granted
    // group and its tools, without the ungranted `app` group.
    let system_messages: Vec<&str> = recorded[0]["messages"]
        .as_array()
        .expect("first request messages should be an array")
        .iter()
        .filter_map(|message| message["content"].as_str())
        .collect();
    let instructions = system_messages.join("\n");
    let inventory = system_messages
        .iter()
        .find(|message| message.contains("\"domainCode\":\"harness_workspace\""))
        .unwrap_or_else(|| {
            panic!("authorized domain missing from the run instructions: {instructions}")
        });
    assert!(
        !instructions.contains("{{visible_domains}}"),
        "placeholder leaked into the run instructions: {instructions}"
    );
    assert!(
        inventory.contains("workspace_page") && inventory.contains("workspace_get"),
        "domain tool codes missing from the run instructions: {inventory}"
    );
    assert!(
        !inventory.contains("\"domainCode\":\"harness_app\""),
        "unauthorized domain leaked into the run instructions: {inventory}"
    );
    assert!(
        instructions.contains("tool_search") && instructions.contains("tool_execute"),
        "gateway contract missing from the run instructions: {inventory}"
    );

    let latest_tool_output = |request: &Value| -> String {
        request["messages"]
            .as_array()
            .expect("request messages should be an array")
            .iter()
            .filter(|message| message["role"] == json!("tool"))
            .last()
            .and_then(|message| message["content"].as_str())
            .expect("the newest step should have a tool result")
            .to_string()
    };

    // Grouping: `domainCode` lists that group's wrapped tools only.
    let domain_search: Value = serde_json::from_str(&common::single_gateway_result(
        &latest_tool_output(&recorded[1]),
    ))
    .expect("domain search result JSON");
    let grouped: Vec<String> = domain_search["tools"]
        .as_array()
        .expect("grouped tools should be an array")
        .iter()
        .filter_map(|tool| tool["call_name"].as_str().map(str::to_string))
        .collect();
    assert!(
        grouped.contains(&"workspace_page".to_string())
            && grouped.contains(&"workspace_get".to_string()),
        "the granted group should list its own tools: {domain_search}"
    );
    assert!(
        !grouped.contains(&"app_page".to_string()),
        "another domain's tool leaked into the group listing: {domain_search}"
    );
    assert_eq!(domain_search["matched"], json!(2), "{domain_search}");
    assert_eq!(domain_search["truncated"], json!(false), "{domain_search}");
    assert!(
        domain_search["tools"]
            .as_array()
            .expect("grouped tools should be an array")
            .iter()
            .all(|tool| tool.get("input_schema").is_none()),
        "summary discovery must omit schemas: {domain_search}"
    );

    // Keyword discovery still reaches the same wrapped tool.
    let keyword_search: Value = serde_json::from_str(&common::single_gateway_result(
        &latest_tool_output(&recorded[2]),
    ))
    .expect("keyword search result JSON");
    assert!(
        keyword_search["tools"]
            .as_array()
            .expect("keyword tools should be an array")
            .iter()
            .any(|tool| tool["call_name"] == json!("workspace_page")),
        "the keyword path should still discover workspace_page: {keyword_search}"
    );

    // The discovered wrapped tool executes through the gateway envelope.
    let executed: Value = serde_json::from_str(&common::single_gateway_result(
        &latest_tool_output(&recorded[3]),
    ))
    .expect("workspace page result JSON");
    assert!(
        executed["records"]
            .as_array()
            .is_some_and(|records| !records.is_empty()),
        "{executed}"
    );
}

/// One scripted `tool_search` call that lists a single capability group.
fn domain_search_response(domain_code: &str) -> Value {
    json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "tool_calls": [{
                    "id": format!("call_tool_search_{domain_code}"),
                    "type": "function",
                    "function": {
                        "name": "tool_search",
                        "arguments": json!({ "domainCode": domain_code, "limit": 20 }).to_string()
                    }
                }]
            }
        }]
    })
}

/// Domain codes advertised by the rendered `agent.domain-discovery` inventory.
fn advertised_domain_codes(instructions: &str) -> Vec<String> {
    instructions
        .lines()
        .filter_map(|line| serde_json::from_str::<Vec<Value>>(line).ok())
        .flatten()
        .filter_map(|domain| domain["domainCode"].as_str().map(str::to_owned))
        .collect()
}

/// The host owns the tool taxonomy. Framework packs (bash/fs/search/terminal,
/// collaboration, skills, a2ui, remote plugins) mint bindings the host never
/// registers, so they carry no host per-binding label; the run request must
/// publish the authorized tool -> domain map instead. This proves that the
/// published taxonomy makes a framework-pack tool listable by `domainCode`,
/// that every advertised domain lists at least one tool, and that the
/// model-visible catalog stays the gateway pair on every step.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn host_taxonomy_makes_framework_pack_tools_domain_listable() {
    let codex_command = temp_command_path("codex-host-taxonomy");
    let opencode_command = temp_command_path("opencode-host-taxonomy");
    write_fake_command(&codex_command);
    write_fake_command(&opencode_command);

    // One `domainCode` search per advertised group: command/fs/search are
    // framework-pack tools (no host per-binding label), harness_workspace is a
    // product-catalog tool.
    let model_server = scripted_model_server(vec![
        domain_search_response("command"),
        domain_search_response("fs"),
        domain_search_response("search"),
        domain_search_response("harness_workspace"),
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "taxonomy verified"
                }
            }]
        }),
    ])
    .await;

    let mut config = test_config(&codex_command, &opencode_command);
    // Builtin prompts are seeded only when the database file name does not start
    // with `tron-` (see `src/app/builders.rs::should_seed_builtin_prompts`), so
    // rename the temp database exactly like the neighbouring prompt tests.
    let database = PathBuf::from(&config.database_url);
    config.database_url = database
        .with_file_name(format!(
            "seeded-{}",
            database.file_name().unwrap().to_string_lossy()
        ))
        .display()
        .to_string();
    let app = app_with_config(config);
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    // Narrow the saved policy so the run authorizes exactly one representative
    // per framework pack plus one product tool.
    let patch_policy = request_with_headers(
        &app,
        Method::PATCH,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/policy",
        Some(json!({ "toolAssignmentMode": "selected_tools" })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(patch_policy.status(), StatusCode::OK);
    let assign = request_with_headers(
        &app,
        Method::PUT,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/tool-assignments",
        Some(json!({
            "toolCodes": ["bash.exec", "fs.read_text", "search.grep", "workspace.page"]
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(assign.status(), StatusCode::OK);

    let run_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({
            "prompt": "List every capability group you can reach.",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "TEST_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": model_server.base_url,
                "apiKey": "sk-test-api",
                "modelName": "gpt-test"
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    let run_status = run_response.status();
    let run_body = response_json(run_response).await;
    assert_eq!(run_status, StatusCode::OK, "{run_body}");
    assert_eq!(
        run_body["data"]["finalOutput"],
        json!("taxonomy verified"),
        "{run_body}"
    );

    let recorded = model_server
        .requests
        .lock()
        .expect("recorded model requests mutex poisoned")
        .clone();
    assert_eq!(recorded.len(), 5, "one request per model step");

    // The model-visible catalog is the stable gateway pair on every step.
    for (index, request) in recorded.iter().enumerate() {
        let names: Vec<Value> = request["tools"]
            .as_array()
            .unwrap_or_else(|| panic!("request {index} tools should be an array"))
            .iter()
            .map(|tool| tool["function"]["name"].clone())
            .collect();
        assert_eq!(
            names,
            vec![json!("tool_execute"), json!("tool_search")],
            "request {index} must expose exactly the gateway pair"
        );
    }

    let instructions = recorded[0]["messages"]
        .as_array()
        .expect("first request messages should be an array")
        .iter()
        .filter_map(|message| message["content"].as_str())
        .find(|content| !advertised_domain_codes(content).is_empty())
        .unwrap_or_else(|| panic!("run instructions must carry the domain inventory"))
        .to_string();
    let mut advertised = advertised_domain_codes(&instructions);
    advertised.sort();
    advertised.dedup();
    assert_eq!(
        advertised,
        vec!["command", "fs", "harness_workspace", "search"],
        "only the authorized capability groups may be advertised: {instructions}"
    );

    // Every advertised group lists at least one wrapped tool; the
    // framework-pack members prove the host taxonomy crossed the pack boundary.
    let latest_tool_output = |request: &Value| -> String {
        request["messages"]
            .as_array()
            .expect("request messages should be an array")
            .iter()
            .filter(|message| message["role"] == json!("tool"))
            .last()
            .and_then(|message| message["content"].as_str())
            .expect("the newest step should have a tool result")
            .to_string()
    };
    for (index, (domain_code, tool_name)) in [
        ("command", "command_exec"),
        ("fs", "fs_read_text"),
        ("search", "search_grep"),
        ("harness_workspace", "workspace_page"),
    ]
    .into_iter()
    .enumerate()
    {
        let listing: Value = serde_json::from_str(&common::single_gateway_result(
            &latest_tool_output(&recorded[index + 1]),
        ))
        .unwrap_or_else(|error| panic!("`{domain_code}` search result JSON: {error}"));
        assert_eq!(
            listing["matched"],
            json!(1),
            "`{domain_code}` must list its wrapped tool: {listing}"
        );
        assert_eq!(listing["truncated"], json!(false), "{listing}");
        let call_names: Vec<&str> = listing["tools"]
            .as_array()
            .expect("grouped tools should be an array")
            .iter()
            .filter_map(|tool| tool["call_name"].as_str())
            .collect();
        assert_eq!(
            call_names,
            vec![tool_name],
            "`{domain_code}` must list exactly {tool_name}: {listing}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn internal_agent_domain_discovery_keeps_unexposed_effective_tools_authorized() {
    let codex_command = temp_command_path("codex-domain-effective-tools");
    let opencode_command = temp_command_path("opencode-domain-effective-tools");
    write_fake_command(&codex_command);
    write_fake_command(&opencode_command);

    let model_server = scripted_model_server(vec![
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [
                        {
                            "function": {
                                "name": "fs_write_text",
                                "arguments": "{\"path\":\"long_running_test.sh\",\"relativeTo\":\"workspaceRoot\",\"text\":\"echo test\"}"
                            }
                        },
                        {
                            "function": {
                                "name": "discovery_expand_tool_domain",
                                "arguments": "{\"domainCode\":\"fs\"}"
                            }
                        }
                    ]
                }
            }]
        }),
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "recovered with the discovered schema"
                }
            }]
        }),
    ])
    .await;
    let app = app_with_config(test_config(&codex_command, &opencode_command));
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let patch_policy = request_with_headers(
        &app,
        Method::PATCH,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/policy",
        Some(json!({})),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(patch_policy.status(), StatusCode::OK);

    let run_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({
            "prompt": "Create a script after discovering filesystem tools.",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "TEST_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": model_server.base_url,
                "apiKey": "sk-test-api",
                "modelName": "gpt-test"
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    let run_status = run_response.status();
    let run_body = response_json(run_response).await;
    assert_eq!(run_status, StatusCode::OK, "{run_body}");
    assert_eq!(
        run_body["data"]["finalOutput"],
        json!("recovered with the discovered schema")
    );

    let recorded_requests = model_server
        .requests
        .lock()
        .expect("recorded scripted model request mutex poisoned")
        .clone();
    assert_eq!(recorded_requests.len(), 2);

    // Gateway semantics: the schema never contains inner tools; execution is
    // authorized by the product catalog, not by schema visibility.
    for (index, request) in recorded_requests.iter().enumerate() {
        let tools = request["tools"]
            .as_array()
            .unwrap_or_else(|| panic!("request {index} tools should be an array"));
        let names: Vec<_> = tools
            .iter()
            .map(|tool| tool["function"]["name"].clone())
            .collect();
        assert_eq!(
            names,
            vec![json!("tool_execute"), json!("tool_search")],
            "request {index} must expose exactly the gateway pair"
        );
        assert!(
            !request.to_string().contains("\"name\":\"fs_write_text\""),
            "inner tools must never appear in the schema: {index}"
        );
    }
    let second_request = recorded_requests[1].to_string();
    // fs_write_text stayed authorized: its envelope executed through the
    // gateway even though it was never part of any request schema.
    assert!(second_request.contains("fs_write_text"), "{second_request}");
    assert!(
        !second_request.contains("fs_write_text is not allowed"),
        "{second_request}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn internal_agent_skill_view_loads_progressive_skill_detail() {
    let codex_command = temp_command_path("codex-skill-view");
    let opencode_command = temp_command_path("opencode-skill-view");
    write_fake_command(&codex_command);
    write_fake_command(&opencode_command);

    let registry_root = temp_command_path("skill-view-registry");
    let skill_dir = registry_root
        .join(".agents")
        .join("skills")
        .join("progressive-reviewer");
    std::fs::create_dir_all(skill_dir.join("references"))
        .expect("test skill references directory should be created");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\n\
name: progressive-reviewer\n\
description: Review progressive disclosure tool details.\n\
metadata:\n\
  version: 1.0.0\n\
---\n\
\n\
# Progressive Reviewer\n\
\n\
Progressive disclosure instructions loaded.\n",
    )
    .expect("test skill markdown should be written");
    std::fs::write(
        skill_dir.join("references").join("checklist.md"),
        "Confirm the selected skill details are loaded only after skill_view.\n",
    )
    .expect("test skill reference should be written");

    let model_server = scripted_model_server(vec![
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_skill_view",
                        "type": "function",
                        "function": {
                            "name": "skill_view",
                            "arguments": "{\"skillName\":\"progressive-reviewer\"}"
                        }
                    }]
                }
            }]
        }),
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "skill detail loaded"
                }
            }]
        }),
    ])
    .await;
    let mut config = test_config(&codex_command, &opencode_command);
    config.skills_registry_root = registry_root.display().to_string();
    let app = app_with_config(config);
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    common::enable_direct_list(&app).await;

    let tool_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/tools/skill.view",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(tool_response.status(), StatusCode::OK);
    let tool_body = response_json(tool_response).await;
    assert_eq!(tool_body["data"]["toolCode"], json!("skill.view"));
    assert_eq!(
        tool_body["data"]["inputSchema"]["required"],
        json!(["skillName"])
    );

    let rescan_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/skills/registry/rescan",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(rescan_response.status(), StatusCode::OK);

    let bind_response = request_with_headers(
        &app,
        Method::PUT,
        "/api/v1/agents/registry/internal/main-agent/skills",
        Some(json!({"skillNames": ["progressive-reviewer"]})),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(bind_response.status(), StatusCode::OK);

    let run_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({
            "prompt": "Load the selected skill details.",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "TEST_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": model_server.base_url,
                "apiKey": "sk-test-api",
                "modelName": "gpt-test",
                // Reserve room for the complete catalog and native capability schemas.
                "contextWindowTokens": 131072
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    let run_status = run_response.status();
    let run_body = response_json(run_response).await;
    assert_eq!(run_status, StatusCode::OK, "{run_body}");
    assert_eq!(
        run_body["data"]["finalOutput"],
        json!("skill detail loaded")
    );

    let recorded_requests = model_server
        .requests
        .lock()
        .expect("recorded scripted model request mutex poisoned")
        .clone();
    assert_eq!(recorded_requests.len(), 2);
    // Gateway semantics: skill_view is envelope inventory, never a schema
    // entry; its result still flows back through tool_execute.
    let first_tools = recorded_requests[0]["tools"]
        .as_array()
        .expect("first request tools should be an array");
    let names: Vec<_> = first_tools
        .iter()
        .map(|tool| tool["function"]["name"].clone())
        .collect();
    assert_eq!(
        names,
        vec![json!("tool_execute"), json!("tool_search")],
        "only the gateway pair is model-visible"
    );
    let second_messages = recorded_requests[1]["messages"]
        .as_array()
        .expect("second request messages should be an array");
    let skill_result = provider_tool_result(second_messages, "skill_view");
    assert!(
        skill_result.contains("Progressive disclosure instructions loaded."),
        "{skill_result}"
    );
    assert!(
        skill_result.contains("references/checklist.md"),
        "{skill_result}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn internal_agent_domain_discovery_uses_request_locale_for_discovery_schema() {
    let codex_command = temp_command_path("codex-domain-discovery-zh");
    let opencode_command = temp_command_path("opencode-domain-discovery-zh");
    write_fake_command(&codex_command);
    write_fake_command(&opencode_command);

    let model_server = recording_model_server().await;
    let app = app_with_config(test_config(&codex_command, &opencode_command));
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let patch_policy = request_with_headers(
        &app,
        Method::PATCH,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/policy",
        Some(json!({})),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(patch_policy.status(), StatusCode::OK);

    let run_response = request_with_locale(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({
            "prompt": "先看看有哪些 domain。",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "TEST_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": model_server.base_url,
                "apiKey": "sk-test-api",
                "modelName": "gpt-test"
            }
        })),
        &authorization,
        "zh-CN",
    )
    .await;
    assert_eq!(run_response.status(), StatusCode::OK);

    let recorded_requests = model_server
        .requests
        .lock()
        .expect("recorded model request mutex poisoned")
        .clone();
    assert_eq!(recorded_requests.len(), 1);

    let tools = recorded_requests[0]["tools"]
        .as_array()
        .expect("tools should be an array");
    let names: Vec<_> = tools
        .iter()
        .map(|tool| tool["function"]["name"].clone())
        .collect();
    assert_eq!(
        names,
        vec![json!("tool_execute"), json!("tool_search")],
        "only the gateway pair is model-visible"
    );
    // The gateway pair is locale-independent on purpose: the schema is the
    // stable cache anchor. Request locale flows through the system messages.
    let search_tool = tools
        .iter()
        .find(|tool| tool["function"]["name"] == json!("tool_search"))
        .expect("gateway search must be exposed");
    assert!(
        search_tool["function"]["description"]
            .as_str()
            .is_some_and(|description| description.contains("tool_execute")),
        "search description must teach the execute envelope"
    );
    let messages = recorded_requests[0]["messages"]
        .as_array()
        .expect("messages should be an array");
    assert!(
        messages.iter().any(|message| {
            message["role"] == json!("system")
                && message["content"]
                    .as_str()
                    .is_some_and(|content| content.contains("主智能体"))
        }),
        "the zh-CN locale must localize the system prompt"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn capability_catalog_exposes_installed_packs_and_extension_implementations() {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let packs = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/capabilities/packs",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(packs.status(), StatusCode::OK);
    let packs = response_json(packs).await;
    let packs = packs["data"].as_array().expect("pack catalog array");
    assert!(
        packs
            .iter()
            .any(|pack| pack["id"] == json!("framework.external-agent"))
    );
    assert!(
        packs
            .iter()
            .any(|pack| pack["id"] == json!("framework.local.scoped"))
    );
    assert!(packs.iter().any(|pack| pack["id"] == json!("tron.harness")));
    assert!(packs.iter().all(|pack| pack["effective"] == json!(true)));
    assert!(packs.iter().all(|pack| pack["version"].as_str().is_some()));

    let extensions = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/capabilities/extensions",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(extensions.status(), StatusCode::OK);
    let extensions = response_json(extensions).await;
    let extensions = extensions["data"]
        .as_array()
        .expect("extension catalog array");
    assert!(
        extensions
            .iter()
            .any(|row| row["extensionPoint"] == json!("model"))
    );
    assert!(
        extensions
            .iter()
            .any(|row| row["extensionPoint"] == json!("tool"))
    );
    assert!(
        extensions
            .iter()
            .any(|row| row["extensionPoint"] == json!("context"))
    );
    assert!(extensions.iter().all(|row| {
        row["description"]
            .as_str()
            .is_some_and(|description| !description.trim().is_empty())
    }));
    let tool_description = extensions
        .iter()
        .find(|row| row["extensionPoint"] == json!("tool"))
        .expect("tool extension implementation")
        .get("description")
        .and_then(serde_json::Value::as_str)
        .expect("package-provided tool description");
    assert!(
        tool_description.contains(
            eliteforge_agent_official_plugin_tools_local::TOOL_IMPLEMENTATION_DESCRIPTION
        )
    );
    let harness = packs
        .iter()
        .find(|pack| pack["id"] == "tron.harness")
        .unwrap();
    assert!(harness["contents"].as_array().unwrap().iter().any(|item| {
        item["kind"] == "Policy"
            && item["description"] == tron_harness_capability::TOOL_IMPLEMENTATION_DESCRIPTION
    }));
    assert!(extensions.iter().all(|row| row["effective"] == json!(true)));

    let disabled = request_with_headers(
        &app,
        Method::PUT,
        "/api/v1/capabilities/packs/framework.external-agent/enabled",
        Some(json!({ "enabled": false })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(disabled.status(), StatusCode::OK);
    let packs = response_json(disabled).await;
    assert!(packs["data"].as_array().unwrap().iter().any(|pack| {
        pack["id"] == json!("framework.external-agent") && pack["effective"] == json!(false)
    }));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn runtime_registry_capabilities_are_discoverable() {
    let codex_command = temp_command_path("codex-runtime-discovery");
    let opencode_command = temp_command_path("opencode-runtime-discovery");
    write_fake_command(&codex_command);
    write_fake_command(&opencode_command);

    let app = app_with_config(test_config(&codex_command, &opencode_command));
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let list_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/agents/runtimes",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = response_json(list_response).await;
    let rows = list_body["data"]
        .as_array()
        .expect("runtime capabilities should be an array");
    assert!(rows.iter().any(|row| row["adapterKey"] == json!("REACT")));
    assert!(
        rows.iter()
            .any(|row| row["adapterKey"] == json!("CODEX_EXEC"))
    );
    assert!(
        rows.iter()
            .any(|row| row["adapterKey"] == json!("OPENCODE_RUN"))
    );
    assert!(
        rows.iter()
            .any(|row| row["adapterKey"] == json!("CODEX_APP_SERVER"))
    );
    assert!(
        rows.iter()
            .any(|row| row["adapterKey"] == json!("OPENCODE_SERVE"))
    );
    assert!(
        rows.iter()
            .any(|row| row["adapterKey"] == json!("OPENCODE_ACP"))
    );

    let codex_row = rows
        .iter()
        .find(|row| row["adapterKey"] == json!("CODEX_EXEC"))
        .expect("codex runtime row should exist");
    assert_eq!(codex_row["available"], json!(true));
    assert_eq!(codex_row["enabled"], json!(true));
    assert_eq!(codex_row["supportsResume"], json!(true));
    assert_eq!(codex_row["supportsCancel"], json!(false));
    assert_eq!(codex_row["supportsSteer"], json!(false));
    let react_row = rows
        .iter()
        .find(|row| row["adapterKey"] == json!("REACT"))
        .expect("react runtime row should exist");
    assert_eq!(react_row["supportsSteer"], json!(true));

    let detail_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/agents/runtimes/CODEX_EXEC/capabilities",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail_body = response_json(detail_response).await;
    assert_eq!(detail_body["data"]["adapterKey"], json!("CODEX_EXEC"));
    assert_eq!(detail_body["data"]["supportsResume"], json!(true));
    assert_eq!(detail_body["data"]["supportsSteer"], json!(false));

    let missing_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/agents/runtimes/UNKNOWN_RUNTIME/capabilities",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(missing_response.status(), StatusCode::NOT_FOUND);
    let missing_body = response_json(missing_response).await;
    assert_eq!(missing_body["code"], json!("AGENT_RUNTIME_NOT_FOUND"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn tool_catalog_read_apis_are_available() {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let categories_response = request_with_locale(
        &app,
        Method::GET,
        "/api/v1/tools/categories",
        None,
        &authorization,
        "zh-CN",
    )
    .await;
    assert_eq!(categories_response.status(), StatusCode::OK);
    let categories_body = response_json(categories_response).await;
    assert_eq!(categories_body["code"], "0");
    let categories = categories_body["data"].as_array().unwrap();
    assert!(categories.iter().any(|row| row["code"] == "tron.platform"
        && row["name"] == "平台工具"
        && row["toolCount"].as_u64().unwrap() > 0));
    assert!(
        categories
            .iter()
            .any(|row| row["code"] == "system.command" && row["name"] == "系统命令工具")
    );
    assert!(categories.iter().any(
        |row| row["code"] == "agent.collaboration" && row["toolCount"].as_u64().unwrap() >= 15
    ));
    assert!(
        categories
            .iter()
            .any(|row| row["code"] == "interaction" && row["toolCount"].as_u64().unwrap() >= 2)
    );

    for (locale, expected) in [
        ("zh-CN", "将任务交给指定智能体"),
        ("en-US", "Delegate one focused task"),
    ] {
        let response = request_with_locale(
            &app,
            Method::GET,
            "/api/v1/tools/agent.spawn",
            None,
            &authorization,
            locale,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        assert!(
            body["data"]["description"]
                .as_str()
                .unwrap()
                .contains(expected)
        );
    }

    let domains_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/tools/domains",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(domains_response.status(), StatusCode::OK);
    let domains_body = response_json(domains_response).await;
    let domains = domains_body["data"]
        .as_array()
        .expect("tool domains should be an array");
    assert!(!domains.is_empty());
    assert!(
        domains
            .iter()
            .any(|row| row["domainCode"] == json!("harness_workspace"))
    );
    assert!(
        domains
            .iter()
            .any(|row| row["domainCode"] == json!("harness_automation"))
    );
    assert!(domains.iter().any(|row| row["domainCode"] == json!("harness_task")));
    assert!(domains.iter().any(|row| row["domainCode"] == json!("command")));
    assert!(domains.iter().any(|row| row["domainCode"] == json!("fs")));
    assert!(
        domains
            .iter()
            .any(|row| row["domainCode"] == json!("search"))
    );

    let tools_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/tools?domainCode=fs&search=read",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(tools_response.status(), StatusCode::OK);
    let tools_body = response_json(tools_response).await;
    let tools = tools_body["data"]
        .as_array()
        .expect("tools should be an array");
    assert!(
        tools
            .iter()
            .any(|row| row["toolCode"] == json!("fs.read_text"))
    );
    let read = tools
        .iter()
        .find(|row| row["toolCode"] == "fs.read_text")
        .unwrap();
    assert_eq!(read["category"]["code"], "system.command");
    assert_eq!(read["operationKind"], "QUERY");
    assert!(read["source"].is_object());

    let workspace_tools_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/tools?domainCode=harness_workspace",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(workspace_tools_response.status(), StatusCode::OK);
    let workspace_tools_body = response_json(workspace_tools_response).await;
    let workspace_tool_codes = workspace_tools_body["data"]
        .as_array()
        .expect("workspace tools should be an array")
        .iter()
        .map(|row| row["toolCode"].clone())
        .collect::<Vec<_>>();
    assert_eq!(
        workspace_tool_codes,
        vec![json!("workspace.page"), json!("workspace.get")]
    );

    let tool_response = request_with_locale(
        &app,
        Method::GET,
        "/api/v1/tools/bash.exec",
        None,
        &authorization,
        "en-US",
    )
    .await;
    assert_eq!(tool_response.status(), StatusCode::OK);
    let tool_body = response_json(tool_response).await;
    assert_eq!(tool_body["data"]["toolCode"], json!("bash.exec"));
    let official = eliteforge_agent_official_plugin_tools_local::local_tool_catalog()
        .into_iter()
        .find(|tool| tool.call_name == "command_exec")
        .unwrap();
    assert_eq!(tool_body["data"]["inputSchema"], official.resolve_display("en-US").input_schema);
    assert!(
        tool_body["data"]["description"]
            .as_str()
            .unwrap()
            .contains("10 seconds")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn tool_catalog_read_apis_follow_accept_language() {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let domains_response = request_with_locale(
        &app,
        Method::GET,
        "/api/v1/tools/domains",
        None,
        &authorization,
        "zh-CN",
    )
    .await;
    assert_eq!(domains_response.status(), StatusCode::OK);
    let domains_body = response_json(domains_response).await;
    let workspace_domain = domains_body["data"]
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row["domainCode"] == json!("harness_workspace"))
        })
        .expect("workspace domain should be present");
    assert_eq!(workspace_domain["name"], json!("工作区工具"));
    assert_eq!(
        workspace_domain["description"],
        json!("查看工作区层级和元数据。")
    );

    let tool_response = request_with_locale(
        &app,
        Method::GET,
        "/api/v1/tools/bash.exec",
        None,
        &authorization,
        "zh-CN",
    )
    .await;
    assert_eq!(tool_response.status(), StatusCode::OK);
    let tool_body = response_json(tool_response).await;
    assert_eq!(tool_body["data"]["name"], json!("执行命令"));
    let official = eliteforge_agent_official_plugin_tools_local::local_tool_catalog()
        .into_iter()
        .find(|tool| tool.call_name == "command_exec")
        .unwrap();
    let localized = official.resolve_display("zh-CN");
    assert_eq!(tool_body["data"]["description"], localized.description);
    assert_eq!(tool_body["data"]["inputSchema"], localized.input_schema);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workspace_internal_agent_policies_can_be_read_and_updated() {
    let codex_command = temp_command_path("codex-internal-policy");
    let opencode_command = temp_command_path("opencode-internal-policy");
    write_fake_command(&codex_command);
    write_fake_command(&opencode_command);

    let app = app_with_config(test_config(&codex_command, &opencode_command));
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let list_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/agents/workspaces/TRON_CORE/internal",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = response_json(list_response).await;
    let rows = list_body["data"]
        .as_array()
        .expect("internal agents should be an array");
    let main_row = rows
        .iter()
        .find(|row| row["agentCode"] == json!("main-agent"))
        .expect("main-agent should be in workspace internal agents");
    assert_eq!(main_row["source"], json!("PLATFORM_DEFAULT"));
    assert_eq!(main_row["name"], json!("Main Agent"));
    assert!(
        main_row["selectedToolCodes"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty())
    );
    assert!(
        main_row["selectedToolCodes"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|code| code == "bash.exec"))
    );
    assert!(
        main_row["selectedToolCodes"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|code| code == "fs.read_text"))
    );

    let get_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(get_response.status(), StatusCode::OK);
    let get_body = response_json(get_response).await;
    assert_eq!(get_body["data"]["agentCode"], json!("main-agent"));
    assert_eq!(
        get_body["data"]["toolAssignmentMode"],
        json!("ALL_PLATFORM_TOOLS")
    );
    assert_eq!(
        get_body["data"]["mcpAccessMode"],
        json!("ALL_ENABLED_SERVERS")
    );
    assert_eq!(get_body["data"]["selectedServerCodes"], json!([]));
    assert_eq!(get_body["data"]["maxSteps"], json!(100));
    assert_eq!(get_body["data"]["maxToolCalls"], json!(100));
    assert_eq!(get_body["data"]["maxParallelToolCalls"], json!(10));

    let create_knowledge_server = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/mcp/servers",
        Some(json!({
            "serverCode": "knowledge_tools",
            "name": "Knowledge Tools",
            "description": "Knowledge MCP",
            "url": "https://mcp.example.com/knowledge",
            "authKind": "none",
            "headers": {}
        })),
        None,
        Some(&authorization),
    )
    .await;
    if create_knowledge_server.status() != StatusCode::CREATED {
        let body = response_json(create_knowledge_server).await;
        panic!("create_knowledge_server failed: {body}");
    }

    let create_memory_server = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/mcp/servers",
        Some(json!({
            "serverCode": "memory_tools",
            "name": "Memory Tools",
            "description": "Memory MCP",
            "url": "https://mcp.example.com/memory",
            "authKind": "none",
            "headers": {}
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(create_memory_server.status(), StatusCode::CREATED);

    let patch_response = request_with_headers(
        &app,
        Method::PATCH,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/policy",
        Some(json!({
            "toolAssignmentMode": "selected_tools",
            "maxSteps": 12,
            "maxToolCalls": 6,
            "maxParallelToolCalls": 3
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(patch_response.status(), StatusCode::OK);
    let patched = response_json(patch_response).await;
    assert_eq!(
        patched["data"]["toolAssignmentMode"],
        json!("SELECTED_TOOLS")
    );
    assert_eq!(patched["data"]["maxSteps"], json!(12));
    assert_eq!(patched["data"]["maxToolCalls"], json!(6));
    assert_eq!(patched["data"]["maxParallelToolCalls"], json!(3));

    let mcp_patch_response = request_with_headers(
        &app,
        Method::PATCH,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/mcp-policy",
        Some(json!({
            "mcpAccessMode": "selected_servers"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(mcp_patch_response.status(), StatusCode::OK);
    let mcp_patched = response_json(mcp_patch_response).await;
    assert_eq!(
        mcp_patched["data"]["mcpAccessMode"],
        json!("SELECTED_SERVERS")
    );

    let assign_response = request_with_headers(
        &app,
        Method::PUT,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/tool-assignments",
        Some(json!({
            "toolCodes": ["app.page", "workspace.page", "app.page"]
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(assign_response.status(), StatusCode::OK);
    let assigned = response_json(assign_response).await;
    assert_eq!(assigned["data"]["assignedToolCount"], json!(2));
    assert_eq!(
        assigned["data"]["selectedToolCodes"],
        json!(["app.page", "workspace.page"])
    );

    let mcp_assign_response = request_with_headers(
        &app,
        Method::PUT,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/mcp-server-assignments",
        Some(json!({
            "serverCodes": ["knowledge_tools", "memory_tools", "knowledge_tools"]
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(mcp_assign_response.status(), StatusCode::OK);
    let mcp_assigned = response_json(mcp_assign_response).await;
    assert_eq!(
        mcp_assigned["data"]["selectedServerCodes"],
        json!(["knowledge_tools", "memory_tools"])
    );

    let delete_memory_server = request_with_headers(
        &app,
        Method::DELETE,
        "/api/v1/mcp/servers/memory_tools",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(delete_memory_server.status(), StatusCode::OK);

    let stale_mcp_assign_response = request_with_headers(
        &app,
        Method::PUT,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/mcp-server-assignments",
        Some(json!({
            "serverCodes": ["knowledge_tools", "memory_tools"]
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(stale_mcp_assign_response.status(), StatusCode::BAD_REQUEST);
    let stale_mcp_assigned = response_json(stale_mcp_assign_response).await;
    assert_eq!(
        stale_mcp_assigned["code"],
        json!("AGENT_INTERNAL_MCP_SERVER_ASSIGNMENT_STALE")
    );

    let refreshed_mcp_policy = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(refreshed_mcp_policy.status(), StatusCode::OK);
    let refreshed_mcp_policy_body = response_json(refreshed_mcp_policy).await;
    assert_eq!(
        refreshed_mcp_policy_body["data"]["selectedServerCodes"],
        json!(["knowledge_tools"])
    );

    let reset_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/reset-policy",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(reset_response.status(), StatusCode::OK);
    let reset = response_json(reset_response).await;
    assert_eq!(reset["data"]["source"], json!("PLATFORM_DEFAULT"));
    assert_eq!(reset["data"]["maxSteps"], json!(100));
    assert_eq!(reset["data"]["maxToolCalls"], json!(100));
    assert_eq!(reset["data"]["maxParallelToolCalls"], json!(10));
    assert_eq!(reset["data"]["mcpAccessMode"], json!("ALL_ENABLED_SERVERS"));
    assert_eq!(reset["data"]["selectedServerCodes"], json!([]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workspace_internal_agent_policy_limits_flow_into_runtime() {
    let codex_command = temp_command_path("codex-internal-policy-limits");
    let opencode_command = temp_command_path("opencode-internal-policy-limits");
    write_fake_command(&codex_command);
    write_fake_command(&opencode_command);

    let model_server = scripted_model_server(vec![
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "function": {
                            "name": "task_create",
                            "arguments": "{\"workspaceCode\":\"TRON_CORE\",\"title\":\"RuntimePolicyLimitWitness\"}"
                        }
                    }]
                }
            }]
        }),
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "should not be reached"
                }
            }]
        }),
    ])
    .await;
    let app = app_with_config(test_config(&codex_command, &opencode_command));
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let patch_policy = request_with_headers(
        &app,
        Method::PATCH,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/policy",
        Some(json!({
            "maxSteps": 1,
            "maxToolCalls": 1,
            "toolAssignmentMode": "selected_tools",
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(patch_policy.status(), StatusCode::OK);
    let assignment = internal_agent_api(
        &app,
        &authorization,
        Method::PUT,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/tool-assignments",
        Some(json!({"toolCodes":["task.create"]})),
    )
    .await;
    assert_eq!(assignment["selectedToolCodes"], json!(["task.create"]));
    assert_eq!(assignment["assignedToolCount"], 1);
    internal_agent_api(
        &app,
        &authorization,
        Method::PATCH,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/mcp-policy",
        Some(json!({"mcpAccessMode":"disabled"})),
    )
    .await;

    let run_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({
            "prompt": "Inspect the workspace list.",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "TEST_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": model_server.base_url,
                "apiKey": "sk-test-api",
                "modelName": "gpt-test",
                // Keep the original model capacity. This case needs only task_create;
                // catalog growth must not turn it into a schema-budget test.
                "contextWindowTokens": 65536
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    let run_status = run_response.status();
    let run_body = response_json(run_response).await;
    assert_eq!(run_status, StatusCode::OK, "{run_body}");
    assert_eq!(run_body["data"]["status"], json!("STEP_LIMIT"));
    let native = &run_body["data"]["native"];
    assert_eq!(native["status"], json!("StepLimit"));
    assert!(native["runId"].as_str().is_some_and(|id| !id.is_empty()));
    assert_eq!(run_body["data"]["stepCount"], json!(1));
    let requests = model_server.requests.lock().unwrap().clone();
    assert_eq!(
        requests.len(),
        1,
        "the step limit must stop the second model call"
    );
    // The stable gateway pair is the model-visible catalog; task_create runs
    // through the tool_execute envelope, not the request schema.
    let tool_names: Vec<_> = requests[0]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["function"]["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        tool_names
            .iter()
            .filter(|n| **n != "tool_search" && **n != "tool_execute")
            .count(),
        0,
        "only the gateway pair may be model-visible: {tool_names:?}"
    );
    // Tool observations are deliberately omitted from the HTTP DTO. A real
    // product-side effect proves the single permitted invocation executed.
    let tasks = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/tasks?workspaceCode=TRON_CORE&search=RuntimePolicyLimitWitness",
        None,
        None,
        Some(&authorization),
    )
    .await;
    let task_status = tasks.status();
    let tasks = response_json(tasks).await;
    assert_eq!(task_status, StatusCode::OK, "{tasks}");
    let records = tasks["data"]["records"]
        .as_array()
        .unwrap_or_else(|| panic!("task page: {tasks}"));
    assert_eq!(records.len(), 1, "{tasks}");
    assert_eq!(records[0]["title"], json!("RuntimePolicyLimitWitness"));
    assert_eq!(records[0]["workspaceCode"], json!("TRON_CORE"));

    let run_code = run_body["data"]["runCode"].as_str().expect("run code");
    let history = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/agents/runs/{run_code}"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(history.status(), StatusCode::OK);
    let history = response_json(history).await;
    assert_eq!(history["data"]["status"], json!("FAILED"));
    assert_eq!(history["data"]["outcome"], json!("step_limit"));
    assert_eq!(
        history["data"]["errorCode"],
        json!("AGENT_MAX_STEPS_EXCEEDED")
    );
    assert_eq!(history["data"]["stepCount"], json!(1));
    assert_eq!(history["data"]["observationCount"], json!(1));

    let recorded_requests = model_server
        .requests
        .lock()
        .expect("recorded scripted model request mutex poisoned")
        .clone();
    assert_eq!(recorded_requests.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn internal_agent_mcp_tools_materialize_and_keep_run_snapshots_stable() {
    let codex_command = temp_command_path("codex-mcp-runtime");
    let opencode_command = temp_command_path("opencode-mcp-runtime");
    write_fake_command(&codex_command);
    write_fake_command(&opencode_command);

    let blocking_mcp = blocking_mcp_server("search_docs", "knowledge snapshot").await;
    let BlockingMcpServer {
        base_url: mcp_base_url,
        requests: mcp_requests,
        release_call,
        tool_call_started,
        ..
    } = blocking_mcp;
    let model_server = scripted_model_server(vec![
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "function": {
                            "name": "mcp_knowledge_tools_search_docs",
                            "arguments": "{\"query\":\"snapshot\"}"
                        }
                    }]
                }
            }]
        }),
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "run one finished"
                }
            }]
        }),
    ])
    .await;
    let app = app_with_config(test_config(&codex_command, &opencode_command));
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    common::enable_direct_list(&app).await;

    let create_server = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/mcp/servers",
        Some(json!({
            "serverCode": "knowledge_tools",
            "name": "Knowledge Tools",
            "description": "Knowledge MCP",
            "url": mcp_base_url,
            "authKind": "none",
            "headers": {}
        })),
        None,
        Some(&authorization),
    )
    .await;
    if create_server.status() != StatusCode::CREATED {
        let body = response_json(create_server).await;
        panic!("create_server failed: {body}");
    }

    let patch_mcp_policy = request_with_headers(
        &app,
        Method::PATCH,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/mcp-policy",
        Some(json!({
            "mcpAccessMode": "all_enabled_servers"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(patch_mcp_policy.status(), StatusCode::OK);

    let run_app = app.clone();
    let run_authorization = authorization.clone();
    let run_model_base_url = model_server.base_url.clone();
    let run_task = tokio::spawn(async move {
        request_with_headers(
            &run_app,
            Method::POST,
            "/api/v1/agents/runs",
            Some(json!({
                "prompt": "Use the MCP knowledge server.",
                "workspaceCode": "TRON_CORE",
                "model": {
                    "modelCode": "TEST_MODEL",
                    "protocol": "CHAT_COMPLETIONS",
                    "baseUrl": run_model_base_url,
                    "apiKey": "sk-test-api",
                    "modelName": "gpt-test",
                    "contextWindowTokens": 131072
                }
            })),
            None,
            Some(&run_authorization),
        )
        .await
    });

    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(15), tool_call_started)
            .await
            .expect("MCP tool start deadline")
            .is_ok(),
        "first run should reach MCP tool execution"
    );

    let disable_server = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/mcp/servers/knowledge_tools/disable",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(disable_server.status(), StatusCode::OK);

    release_call.notify_waiters();

    let run_response = run_task
        .await
        .expect("first run task should join successfully");
    let run_status = run_response.status();
    let run_body = response_json(run_response).await;
    assert_eq!(run_status, StatusCode::OK, "{run_body}");
    assert_eq!(run_body["data"]["finalOutput"], json!("run one finished"));

    let recorded_model_requests = model_server
        .requests
        .lock()
        .expect("recorded scripted model request mutex poisoned")
        .clone();
    // Gateway semantics: MCP tools are envelope inventory, never schema
    // entries; the MCP transport still serves tools/list and tools/call.
    let first_run_tools = recorded_model_requests[0]["tools"]
        .as_array()
        .expect("first run tools should be an array");
    let first_names: Vec<_> = first_run_tools
        .iter()
        .map(|tool| tool["function"]["name"].clone())
        .collect();
    assert_eq!(
        first_names,
        vec![json!("tool_execute"), json!("tool_search")],
        "first run must expose exactly the gateway pair"
    );

    let recorded_mcp_requests = mcp_requests
        .lock()
        .expect("recorded MCP request mutex poisoned")
        .clone();
    assert!(
        recorded_mcp_requests
            .iter()
            .any(|request| request["method"] == json!("tools/list"))
    );
    assert!(
        recorded_mcp_requests
            .iter()
            .any(|request| request["method"] == json!("tools/call"))
    );

    let second_model_server = recording_model_server().await;
    let second_run = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({
            "prompt": "Run without the disabled MCP server.",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "TEST_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": second_model_server.base_url,
                "apiKey": "sk-test-api",
                "modelName": "gpt-test",
                    "contextWindowTokens": 131072
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(second_run.status(), StatusCode::OK);

    let second_run_requests = second_model_server
        .requests
        .lock()
        .expect("recorded second-run model request mutex poisoned")
        .clone();
    // The schema is identical (cache-stable) with or without MCP servers;
    // the disabled server only leaves the gateway inventory.
    let second_run_tools = second_run_requests[0]["tools"]
        .as_array()
        .expect("second run tools should be an array");
    let second_names: Vec<_> = second_run_tools
        .iter()
        .map(|tool| tool["function"]["name"].clone())
        .collect();
    assert_eq!(
        second_names,
        vec![json!("tool_execute"), json!("tool_search")],
        "second run must expose exactly the gateway pair"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn internal_agent_direct_list_materializes_and_executes_coding_tools() {
    let codex_command = temp_command_path("codex-coding-tools");
    let opencode_command = temp_command_path("opencode-coding-tools");
    write_fake_command(&codex_command);
    write_fake_command(&opencode_command);

    let workspace_dir = temp_workspace_dir("coding-tools-runtime");
    let source_dir = workspace_dir.join("src");
    std::fs::create_dir_all(&source_dir).expect("source directory should be created");
    let cargo_toml = workspace_dir.join("Cargo.toml");
    let lib_rs = source_dir.join("lib.rs");
    std::fs::write(&cargo_toml, "[package]\nname = \"coding-tools\"\n")
        .expect("Cargo.toml should be written");
    std::fs::write(&lib_rs, "pub struct ToolRegistry;\n").expect("lib.rs should be written");

    let model_server = scripted_model_server(vec![
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [
                        {
                            "id": "call_command_exec",
                            "type": "function",
                            "function": {
                                "name": "command_exec",
                                "arguments": serde_json::to_string(&json!({
                                    "command": "sed",
                                    "args": ["-n", "1,1p", cargo_toml.display().to_string()]
                                })).expect("command_exec arguments should serialize")
                            }
                        },
                        {
                            "id": "call_fs_read_text",
                            "type": "function",
                            "function": {
                                "name": "fs_read_text",
                                "arguments": serde_json::to_string(&json!({
                                    "path": lib_rs.display().to_string(),
                                    "startLine": 1,
                                    "endLine": 1
                                })).expect("fs_read_text arguments should serialize")
                            }
                        },
                        {
                            "id": "call_search_glob",
                            "type": "function",
                            "function": {
                                "name": "search_glob",
                                "arguments": serde_json::to_string(&json!({
                                    "patterns": ["*.rs"],
                                    "root": workspace_dir.display().to_string()
                                })).expect("search_glob arguments should serialize")
                            }
                        },
                        {
                            "id": "call_search_path",
                            "type": "function",
                            "function": {
                                "name": "search_path",
                                "arguments": serde_json::to_string(&json!({
                                    "patterns": ["src"],
                                    "root": workspace_dir.display().to_string(),
                                    "entryType": "directory"
                                })).expect("search_path arguments should serialize")
                            }
                        }
                    ]
                }
            }]
        }),
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "coding tools executed"
                }
            }]
        }),
    ])
    .await;
    let app = app_with_config(test_config(&codex_command, &opencode_command));
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    common::enable_direct_list(&app).await;

    let run_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({
            "prompt": "Read and inspect the coding workspace.",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "TEST_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": model_server.base_url,
                "apiKey": "sk-test-api",
                "modelName": "gpt-test",
                // Reserve room for the complete catalog and native capability schemas.
                "contextWindowTokens": 131072
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    let run_status = run_response.status();
    let run_body = response_json(run_response).await;
    assert_eq!(run_status, StatusCode::OK, "{run_body}");
    assert_eq!(
        run_body["data"]["finalOutput"],
        json!("coding tools executed")
    );

    let recorded_requests = model_server
        .requests
        .lock()
        .expect("recorded scripted model request mutex poisoned")
        .clone();
    assert_eq!(recorded_requests.len(), 2);

    // Gateway semantics: coding tools run through envelopes; the schema is
    // the stable pair for every request of the run.
    for (index, request) in recorded_requests.iter().enumerate() {
        let tools = request["tools"]
            .as_array()
            .unwrap_or_else(|| panic!("request {index} tools should be an array"));
        let names: Vec<_> = tools
            .iter()
            .map(|tool| tool["function"]["name"].clone())
            .collect();
        assert_eq!(
            names,
            vec![json!("tool_execute"), json!("tool_search")],
            "request {index} must expose exactly the gateway pair"
        );
    }

    let second_messages = recorded_requests[1]["messages"]
        .as_array()
        .expect("second request messages should be an array");
    let command_result: Value =
        serde_json::from_str(&provider_tool_result(second_messages, "command_exec"))
            .expect("command result JSON");
    assert_eq!(command_result["exitCode"], json!(0));
    assert!(
        command_result["stdout"]
            .as_str()
            .is_some_and(|stdout| stdout.contains("[package]")),
        "{command_result}"
    );
    let file_result: Value =
        serde_json::from_str(&provider_tool_result(second_messages, "fs_read_text"))
            .expect("file result JSON");
    assert!(
        file_result["content"]
            .as_str()
            .is_some_and(|content| content.contains("ToolRegistry")),
        "{file_result}"
    );
    let glob_result = provider_tool_result(second_messages, "search_glob");
    assert!(glob_result.contains("src/lib.rs"), "{glob_result}");
    let path_result = provider_tool_result(second_messages, "search_path");
    assert!(path_result.contains("src"), "{path_result}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn internal_agent_platform_mutations_reject_workspace_mismatches() {
    let codex_command = temp_command_path("codex-platform-workspace-guard");
    let opencode_command = temp_command_path("opencode-platform-workspace-guard");
    write_fake_command(&codex_command);
    write_fake_command(&opencode_command);

    let app = app_with_config(test_config(&codex_command, &opencode_command));
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    common::enable_direct_list(&app).await;

    let created_app = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/apps/workspaces/TRON_CORE",
        Some(json!({
            "name": "Guarded Tool App",
            "localDirectory": "/workspace/guarded-tool-app",
            "runtimeRole": "Backend"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(created_app.status(), StatusCode::CREATED);
    let created_app_body = response_json(created_app).await;
    let app_code = created_app_body["data"]["appCode"]
        .as_str()
        .expect("created app should return appCode")
        .to_string();

    let created_task = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/tasks",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Guarded tool task",
            "priority": "Medium",
            "taskType": "Feature",
            "sourceType": "Internal",
            "appCodes": [app_code.clone()]
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(created_task.status(), StatusCode::CREATED);
    let created_task_body = response_json(created_task).await;
    let task_code = created_task_body["data"]["taskCode"]
        .as_str()
        .expect("created task should return taskCode")
        .to_string();

    let huge_current = usize::MAX;
    let model_server = scripted_model_server(vec![
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [
                        {
                            "id": "call_app_page",
                            "type": "function",
                            "function": {
                                "name": "app_page",
                                "arguments": serde_json::to_string(&json!({
                                    "workspaceCode": "TRON_CORE",
                                    "current": huge_current,
                                    "size": 100
                                })).expect("app_page arguments should serialize")
                            }
                        },
                        {
                            "id": "call_task_page",
                            "type": "function",
                            "function": {
                                "name": "task_page",
                                "arguments": serde_json::to_string(&json!({
                                    "workspaceCode": "TRON_CORE",
                                    "current": huge_current,
                                    "size": 100
                                })).expect("task_page arguments should serialize")
                            }
                        },
                        {
                            "id": "call_app_update",
                            "type": "function",
                            "function": {
                                "name": "app_update",
                                "arguments": serde_json::to_string(&json!({
                                    "workspaceCode": "ENTERPRISE",
                                    "appCode": app_code.clone(),
                                    "name": "Cross Workspace Mutation"
                                })).expect("app_update arguments should serialize")
                            }
                        },
                        {
                            "id": "call_task_delete",
                            "type": "function",
                            "function": {
                                "name": "task_delete",
                                "arguments": serde_json::to_string(&json!({
                                    "workspaceCode": "ENTERPRISE",
                                    "taskCode": task_code.clone()
                                })).expect("task_delete arguments should serialize")
                            }
                        }
                    ]
                }
            }]
        }),
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "workspace guard applied"
                }
            }]
        }),
    ])
    .await;

    let run_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({
            "prompt": "Try guarded platform mutations.",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "TEST_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": model_server.base_url,
                "apiKey": "sk-test-api",
                "modelName": "gpt-test",
                // Reserve room for the complete catalog and native capability schemas.
                "contextWindowTokens": 131072
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    let run_status = run_response.status();
    let run_body = response_json(run_response).await;
    assert_eq!(run_status, StatusCode::OK, "{run_body}");
    assert_eq!(
        run_body["data"]["finalOutput"],
        json!("workspace guard applied")
    );

    let recorded_requests = model_server
        .requests
        .lock()
        .expect("recorded scripted model request mutex poisoned")
        .clone();
    assert_eq!(recorded_requests.len(), 2);
    let second_messages = recorded_requests[1]["messages"]
        .as_array()
        .expect("second request messages should be an array");
    for name in ["app_page", "task_page"] {
        let page: Value = serde_json::from_str(&provider_tool_result(second_messages, name))
            .expect("page result JSON");
        assert_eq!(page["current"], json!(huge_current), "{name}: {page}");
        assert_eq!(page["records"], json!([]), "{name}: {page}");
    }
    let update_result = provider_tool_result(second_messages, "app_update");
    assert!(
        update_result
            .contains("workspaceCode is required and must match the current authorized workspace"),
        "{update_result}"
    );
    let delete_result = provider_tool_result(second_messages, "task_delete");
    assert!(
        delete_result.contains("workspaceCode is required and must match the current authorized workspace"),
        "{delete_result}"
    );

    let app_detail = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/apps/{app_code}"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(app_detail.status(), StatusCode::OK);
    let app_detail_body = response_json(app_detail).await;
    assert_eq!(app_detail_body["data"]["name"], json!("Guarded Tool App"));

    let task_detail = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/tasks/{task_code}"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(task_detail.status(), StatusCode::OK);
    let task_detail_body = response_json(task_detail).await;
    assert_eq!(task_detail_body["data"]["status"], json!("NOT_STARTED"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn internal_agent_task_update_rejects_invalid_status_without_partial_update() {
    let codex_command = temp_command_path("codex-task-update-atomic");
    let opencode_command = temp_command_path("opencode-task-update-atomic");
    write_fake_command(&codex_command);
    write_fake_command(&opencode_command);

    let app = app_with_config(test_config(&codex_command, &opencode_command));
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    common::enable_direct_list(&app).await;

    let created_task = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/tasks",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Atomic task update",
            "priority": "Medium",
            "taskType": "Feature",
            "sourceType": "Internal"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(created_task.status(), StatusCode::CREATED);
    let created_task_body = response_json(created_task).await;
    let task_code = created_task_body["data"]["taskCode"]
        .as_str()
        .expect("created task should return taskCode")
        .to_string();

    let model_server = scripted_model_server(vec![
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_task_update",
                        "type": "function",
                        "function": {
                            "name": "task_update",
                            "arguments": serde_json::to_string(&json!({
                                "workspaceCode": "TRON_CORE",
                                "taskCode": task_code.clone(),
                                "title": "Should Not Persist",
                                "status": "STATUS_THAT_DOES_NOT_EXIST",
                                "progress": 40
                            })).expect("task_update arguments should serialize")
                        }
                    }]
                }
            }]
        }),
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "task update rejected"
                }
            }]
        }),
    ])
    .await;

    let run_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({
            "prompt": "Try invalid status update.",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "TEST_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": model_server.base_url,
                "apiKey": "sk-test-api",
                "modelName": "gpt-test",
                // Reserve room for the complete catalog and native capability schemas.
                "contextWindowTokens": 131072
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    let run_status = run_response.status();
    let run_body = response_json(run_response).await;
    assert_eq!(run_status, StatusCode::OK, "{run_body}");

    let recorded_requests = model_server
        .requests
        .lock()
        .expect("recorded scripted model request mutex poisoned")
        .clone();
    assert_eq!(recorded_requests.len(), 2);
    let second_messages = recorded_requests[1]["messages"]
        .as_array()
        .expect("second request messages should be an array");
    let update_result = provider_tool_result(second_messages, "task_update");
    assert!(update_result.contains("status"), "{update_result}");

    let task_detail = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/tasks/{task_code}"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(task_detail.status(), StatusCode::OK);
    let task_detail_body = response_json(task_detail).await;
    assert_eq!(
        task_detail_body["data"]["title"],
        json!("Atomic task update")
    );
    assert_eq!(task_detail_body["data"]["status"], json!("NOT_STARTED"));
    assert_eq!(task_detail_body["data"]["progress"], json!(0));
}

async fn internal_agent_api(
    router: &Router,
    authorization: &str,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> Value {
    let response = request_with_locale(router, method, uri, body, authorization, "zh-CN").await;
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(status, StatusCode::OK, "{uri}: {body}");
    body["data"].clone()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn internal_agent_platform_fields_inherit_override_restore_and_persist() {
    let config = AppConfig {
        database_url: temp_database_url("internal-agent-inheritance"),
        ..AppConfig::default()
    };
    let router = app_with_config(config.clone());
    let authorization = format!("Bearer {}", authenticated_bearer_token(&router).await);
    let created = internal_agent_api(
        &router,
        &authorization,
        Method::POST,
        "/api/v1/agents/registry/internal",
        Some(json!({"agentCode":"global-reviewer","name":"Global reviewer", "rolePrompt":"Global role v1"})),
    )
    .await;
    let code = created["agentCode"].as_str().unwrap();
    internal_agent_api(
        &router,
        &authorization,
        Method::PUT,
        &format!("/api/v1/agents/registry/{code}/workspace-grants"),
        Some(json!({"workspaceCodes":["TRON_CORE", "ENTERPRISE"]})),
    )
    .await;
    let platform = format!("/api/v1/agents/registry/internal/{code}");
    let workspace = format!("/api/v1/agents/workspaces/TRON_CORE/internal/{code}");
    let other = format!("/api/v1/agents/workspaces/ENTERPRISE/internal/{code}");
    internal_agent_api(
        &router,
        &authorization,
        Method::PATCH,
        &format!("{platform}/policy"),
        Some(json!({"maxSteps":50,"maxToolCalls":70,"toolAssignmentMode":"selected_tools"})),
    )
    .await;
    internal_agent_api(
        &router,
        &authorization,
        Method::PUT,
        &format!("{platform}/tool-assignments"),
        Some(json!({"toolCodes":["workspace.page"]})),
    )
    .await;
    let inherited =
        internal_agent_api(&router, &authorization, Method::GET, &workspace, None).await;
    assert_eq!(inherited["platformDefined"], true);
    assert_eq!(inherited["source"], "PLATFORM_DEFAULT");
    assert_eq!(inherited["name"], "Global reviewer");
    assert_eq!(inherited["rolePrompt"], "Global role v1");
    assert_eq!(inherited["maxSteps"], 50);
    assert_eq!(inherited["selectedToolCodes"], json!(["workspace.page"]));
    assert_eq!(inherited["assignedToolCount"], 1);
    internal_agent_api(
        &router,
        &authorization,
        Method::PATCH,
        &format!("{workspace}/definition"),
        Some(json!({"rolePrompt":"Workspace role"})),
    )
    .await;
    internal_agent_api(
        &router,
        &authorization,
        Method::PATCH,
        &format!("{workspace}/policy"),
        Some(json!({"maxSteps":12})),
    )
    .await;
    internal_agent_api(
        &router,
        &authorization,
        Method::PUT,
        &format!("{workspace}/tool-assignments"),
        Some(json!({"toolCodes":[]})),
    )
    .await;
    internal_agent_api(
        &router,
        &authorization,
        Method::PATCH,
        &format!("{platform}/definition"),
        Some(json!({"name":"Updated global reviewer","rolePrompt":"Global role v2"})),
    )
    .await;
    internal_agent_api(
        &router,
        &authorization,
        Method::PATCH,
        &format!("{platform}/policy"),
        Some(json!({"maxSteps":60,"maxToolCalls":90})),
    )
    .await;
    let overridden =
        internal_agent_api(&router, &authorization, Method::GET, &workspace, None).await;
    assert_eq!(overridden["source"], "WORKSPACE_OVERRIDE");
    assert_eq!(
        overridden["name"], "Updated global reviewer",
        "untouched names must follow platform edits"
    );
    assert_eq!(overridden["rolePrompt"], "Workspace role");
    assert_eq!(overridden["maxSteps"], 12);
    assert_eq!(
        overridden["maxToolCalls"], 90,
        "untouched limits must follow platform edits"
    );
    assert_eq!(
        overridden["selectedToolCodes"],
        json!([]),
        "an explicit empty override must not restore inherited tools"
    );
    let other_record = internal_agent_api(&router, &authorization, Method::GET, &other, None).await;
    assert_eq!(other_record["rolePrompt"], "Global role v2");
    assert_eq!(other_record["maxSteps"], 60);
    // Reloading storage must preserve both the definition and the field override mask.
    let reopened = app_with_config(config);
    let authorization = format!("Bearer {}", authenticated_bearer_token(&reopened).await);
    let persisted =
        internal_agent_api(&reopened, &authorization, Method::GET, &workspace, None).await;
    assert_eq!(persisted["rolePrompt"], "Workspace role");
    assert_eq!(persisted["maxSteps"], 12);
    assert_eq!(persisted["maxToolCalls"], 90);
    let restored = internal_agent_api(
        &reopened,
        &authorization,
        Method::POST,
        &format!("{workspace}/reset-policy"),
        None,
    )
    .await;
    assert_eq!(restored["source"], "PLATFORM_DEFAULT");
    assert_eq!(restored["name"], "Updated global reviewer");
    assert_eq!(restored["rolePrompt"], "Global role v2");
    assert_eq!(restored["maxSteps"], 60);
    assert_eq!(restored["selectedToolCodes"], json!(["workspace.page"]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn internal_agent_workspace_definitions_are_local_and_validate_names() {
    let router = app();
    let authorization = format!("Bearer {}", authenticated_bearer_token(&router).await);
    let created = internal_agent_api(
        &router,
        &authorization,
        Method::POST,
        "/api/v1/agents/workspaces/TRON_CORE/internal",
        Some(json!({"agentCode":"local-analyst","name":"Local analyst", "rolePrompt":"Explain evidence and uncertainty."})),
    )
    .await;
    let code = created["agentCode"].as_str().unwrap();
    let workspace = format!("/api/v1/agents/workspaces/TRON_CORE/internal/{code}");
    let local = internal_agent_api(&router, &authorization, Method::GET, &workspace, None).await;
    assert_eq!(local["platformDefined"], false);
    let created_session = request_with_headers(
        &router,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({"workspaceCode":"TRON_CORE", "title":"Local analyst chat", "agentCode":code})),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(created_session.status(), StatusCode::CREATED);
    let session = response_json(created_session).await["data"].clone();
    assert_eq!(session["agentCode"], code);
    let groups = internal_agent_api(
        &router,
        &authorization,
        Method::GET,
        "/api/v1/chats/session-groups?workspaceCode=TRON_CORE",
        None,
    )
    .await;
    assert!(
        groups
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|group| group["sessions"].as_array().unwrap())
            .any(|saved| saved["sessionCode"] == session["sessionCode"]
                && saved["agentCode"] == code)
    );

    let other = internal_agent_api(
        &router,
        &authorization,
        Method::GET,
        "/api/v1/agents/workspaces/ENTERPRISE/internal",
        None,
    )
    .await;
    assert!(
        !other
            .as_array()
            .unwrap()
            .iter()
            .any(|agent| agent["agentCode"] == code)
    );
    // Resetting a local policy must retain its locally owned definition.
    let reset = internal_agent_api(
        &router,
        &authorization,
        Method::POST,
        &format!("{workspace}/reset-policy"),
        None,
    )
    .await;
    assert_eq!(reset["name"], "Local analyst");
    assert_eq!(reset["rolePrompt"], "Explain evidence and uncertainty.");
    for path in [
        "/api/v1/agents/registry/internal",
        "/api/v1/agents/workspaces/TRON_CORE/internal",
    ] {
        let response = request_with_locale(
            &router,
            Method::POST,
            path,
            Some(json!({"agentCode":"empty-name","name":"  ","rolePrompt":""})),
            &authorization,
            "zh-CN",
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response_json(response).await;
        assert_eq!(body["code"], "101028001");
        assert!(body["message"].as_str().unwrap().contains("名称"));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn internal_agent_runs_use_selected_role_and_follow_next_run_configuration() {
    let server = recording_model_server().await;
    let router = app();
    let authorization = format!("Bearer {}", authenticated_bearer_token(&router).await);
    let first = internal_agent_api(
        &router,
        &authorization,
        Method::POST,
        "/api/v1/agents/registry/internal",
        Some(json!({"agentCode":"reviewer","name":"Reviewer","rolePrompt":"Unique reviewer instructions"})),
    )
    .await;
    internal_agent_api(
        &router,
        &authorization,
        Method::PUT,
        &format!(
            "/api/v1/agents/registry/{}/workspace-grants",
            first["agentCode"].as_str().unwrap()
        ),
        Some(json!({"workspaceCodes":["TRON_CORE"]})),
    )
    .await;
    let second = internal_agent_api(
        &router,
        &authorization,
        Method::POST,
        "/api/v1/agents/workspaces/TRON_CORE/internal",
        Some(json!({"agentCode":"analyst","name":"Analyst","rolePrompt":"Unique analyst instructions"})),
    )
    .await;
    for agent in [&first, &second] {
        let result = internal_agent_api(&router, &authorization, Method::POST, "/api/v1/agents/runs", Some(json!({
            "prompt":"Check the proposal.","workspaceCode":"TRON_CORE","agentCode":agent["agentCode"],
            "model":{"modelCode":"TEST_MODEL","protocol":"CHAT_COMPLETIONS","baseUrl":server.base_url,"apiKey":"sk-test","modelName":"gpt-test"}
        }))).await;
        assert_eq!(result["workspaceCode"], "TRON_CORE");
    }
    let code = first["agentCode"].as_str().unwrap();
    internal_agent_api(
        &router,
        &authorization,
        Method::PATCH,
        &format!("/api/v1/agents/registry/internal/{code}/definition"),
        Some(json!({"rolePrompt":"Updated reviewer instructions"})),
    )
    .await;
    internal_agent_api(&router, &authorization, Method::POST, "/api/v1/agents/runs", Some(json!({
        "prompt":"Check again.","workspaceCode":"TRON_CORE","agentCode":code,
        "model":{"modelCode":"TEST_MODEL","protocol":"CHAT_COMPLETIONS","baseUrl":server.base_url,"apiKey":"sk-test","modelName":"gpt-test"}
    }))).await;
    let requests = server.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 3);
    for (request, expected, absent) in [
        (
            &requests[0],
            "Unique reviewer instructions",
            "Unique analyst instructions",
        ),
        (
            &requests[1],
            "Unique analyst instructions",
            "Unique reviewer instructions",
        ),
        (
            &requests[2],
            "Updated reviewer instructions",
            "Unique reviewer instructions",
        ),
    ] {
        let prompt = request["messages"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|message| message["role"] == "system")
            .map(|message| message["content"].as_str().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(prompt.contains(expected), "{prompt}");
        assert!(
            !prompt.contains(absent),
            "roles must not leak across agent instances: {prompt}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn internal_agent_codes_validate_and_reject_conflicts_without_overwriting() {
    let router = app();
    let authorization = format!("Bearer {}", authenticated_bearer_token(&router).await);
    let platform = "/api/v1/agents/registry/internal";
    let workspace = "/api/v1/agents/workspaces/TRON_CORE/internal";
    for path in [platform, workspace] {
        for invalid in [
            "",
            "1agent",
            "UPPER",
            "has space",
            "中文",
            "path/agent",
            &"a".repeat(65),
        ] {
            let response = request_with_locale(
                &router,
                Method::POST,
                path,
                Some(json!({"agentCode":invalid,"name":"Valid name","rolePrompt":"Original"})),
                &authorization,
                "zh-CN",
            )
            .await;
            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "{path}: {invalid}"
            );
            let body = response_json(response).await;
            assert_eq!(body["code"], "101028009");
            assert!(body["message"].as_str().unwrap().contains("编码"));
        }
        for reserved in ["main-agent", "codex", "opencode"] {
            let response = request_with_locale(
                &router,
                Method::POST,
                path,
                Some(json!({"agentCode":reserved,"name":"Override","rolePrompt":"Replacement"})),
                &authorization,
                "en-US",
            )
            .await;
            assert_eq!(response.status(), StatusCode::CONFLICT);
            let body = response_json(response).await;
            assert_eq!(body["code"], "101028010");
            assert!(body["message"].as_str().unwrap().contains("already in use"));
        }
    }
    let created = internal_agent_api(&router, &authorization, Method::POST, platform,
        Some(json!({"agentCode":" global-reviewer_1 ","name":"Global reviewer","rolePrompt":"Original"}))).await;
    assert_eq!(created["agentCode"], "global-reviewer_1");
    let local = internal_agent_api(&router, &authorization, Method::POST, workspace,
        Some(json!({"agentCode":"local-reviewer","name":"Local reviewer","rolePrompt":"Local original"}))).await;
    assert_eq!(local["agentCode"], "local-reviewer");
    for (path, code) in [
        (platform, "global-reviewer_1"),
        (workspace, "global-reviewer_1"),
        (workspace, "local-reviewer"),
        (platform, "local-reviewer"),
    ] {
        let response = request_with_locale(
            &router,
            Method::POST,
            path,
            Some(json!({"agentCode":code,"name":"Replacement","rolePrompt":"Replaced"})),
            &authorization,
            "zh-CN",
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(response_json(response).await["code"], "101028010");
    }
    let saved = internal_agent_api(
        &router,
        &authorization,
        Method::GET,
        &format!("{platform}/global-reviewer_1"),
        None,
    )
    .await;
    assert_eq!(saved["name"], "Global reviewer");
    assert_eq!(saved["rolePrompt"], "Original");
    let saved = internal_agent_api(
        &router,
        &authorization,
        Method::GET,
        &format!("{workspace}/local-reviewer"),
        None,
    )
    .await;
    assert_eq!(saved["rolePrompt"], "Local original");
    // Local identities are scoped to a workspace and may be reused in another one.
    internal_agent_api(
        &router,
        &authorization,
        Method::POST,
        "/api/v1/agents/workspaces/ENTERPRISE/internal",
        Some(
            json!({"agentCode":"local-reviewer","name":"Other reviewer","rolePrompt":"Other role"}),
        ),
    )
    .await;
    for path in [
        format!("{platform}/global-reviewer_1/definition"),
        format!("{workspace}/local-reviewer/definition"),
    ] {
        let response = request_with_headers(
            &router,
            Method::PATCH,
            &path,
            Some(json!({"agentCode":"renamed","name":"Replacement"})),
            None,
            Some(&authorization),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_internal_agent_creation_preserves_the_first_definition() {
    let router = app();
    let authorization = format!("Bearer {}", authenticated_bearer_token(&router).await);
    for path in [
        "/api/v1/agents/registry/internal",
        "/api/v1/agents/workspaces/TRON_CORE/internal",
    ] {
        let code = if path.contains("workspaces") {
            "local-race"
        } else {
            "global-race"
        };
        let first_body = json!({"agentCode":code,"name":"First","rolePrompt":"First role",});
        let second_body = json!({"agentCode":code,"name":"Second","rolePrompt":"Second role",});
        let (first, second) = tokio::join!(
            request_with_headers(
                &router,
                Method::POST,
                path,
                Some(first_body),
                None,
                Some(&authorization)
            ),
            request_with_headers(
                &router,
                Method::POST,
                path,
                Some(second_body),
                None,
                Some(&authorization)
            ),
        );
        let statuses = [first.status(), second.status()];
        assert_eq!(
            statuses
                .iter()
                .filter(|status| **status == StatusCode::OK)
                .count(),
            1
        );
        assert_eq!(
            statuses
                .iter()
                .filter(|status| **status == StatusCode::CONFLICT)
                .count(),
            1
        );
        let winner = response_json(if first.status() == StatusCode::OK {
            first
        } else {
            second
        })
        .await;
        let saved = internal_agent_api(
            &router,
            &authorization,
            Method::GET,
            &format!("{path}/{code}"),
            None,
        )
        .await;
        assert_eq!(saved["name"], winner["data"]["name"]);
        assert_eq!(saved["rolePrompt"], winner["data"]["rolePrompt"]);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn internal_agent_avatars_create_override_restore_and_survive_restart() {
    let config = AppConfig {
        database_url: temp_database_url("internal-agent-avatars"),
        ..AppConfig::default()
    };
    let router = app_with_config(config.clone());
    let auth = format!("Bearer {}", authenticated_bearer_token(&router).await);
    let registry = "/api/v1/agents/registry/internal";
    let defaults = internal_agent_api(
        &router,
        &auth,
        Method::GET,
        &format!("{registry}/main-agent"),
        None,
    )
    .await;
    assert_eq!(defaults["avatarId"], "red-blue-pills");
    for id in [
        "robot",
        "fox",
        "cat",
        "owl",
        "astronaut",
        "wizard",
        "panda",
        "frog",
        "penguin",
        "raccoon",
        "rabbit",
        "dragon",
    ] {
        let created = internal_agent_api(
            &router,
            &auth,
            Method::POST,
            registry,
            Some(
                json!({"agentCode":format!("avatar-{id}"),"name":id,"rolePrompt":"","avatarId":id}),
            ),
        )
        .await;
        assert_eq!(created["avatarId"], id);
    }
    let platform = format!("{registry}/avatar-fox");
    let workspace = "/api/v1/agents/workspaces/TRON_CORE/internal/avatar-fox";
    internal_agent_api(
        &router,
        &auth,
        Method::PUT,
        "/api/v1/agents/registry/avatar-fox/workspace-grants",
        Some(json!({"workspaceCodes":["TRON_CORE"]})),
    )
    .await;
    let inherited = internal_agent_api(&router, &auth, Method::GET, workspace, None).await;
    assert_eq!(inherited["avatarId"], "fox");
    let updated = internal_agent_api(
        &router,
        &auth,
        Method::PATCH,
        &format!("{workspace}/definition"),
        Some(json!({"avatarId":"owl"})),
    )
    .await;
    assert_eq!(updated["avatarId"], "owl");
    internal_agent_api(
        &router,
        &auth,
        Method::PATCH,
        &format!("{platform}/definition"),
        Some(json!({"avatarId":"dragon"})),
    )
    .await;
    let local = internal_agent_api(
        &router,
        &auth,
        Method::POST,
        "/api/v1/agents/workspaces/TRON_CORE/internal",
        Some(json!({"agentCode":"avatar-local","name":"Local","rolePrompt":"","avatarId":"panda"})),
    )
    .await;
    assert_eq!(local["avatarId"], "panda");
    drop(router);
    let router = app_with_config(config);
    let auth = format!("Bearer {}", authenticated_bearer_token(&router).await);
    assert_eq!(
        internal_agent_api(&router, &auth, Method::GET, workspace, None).await["avatarId"],
        "owl"
    );
    assert_eq!(
        internal_agent_api(&router, &auth, Method::GET, &platform, None).await["avatarId"],
        "dragon"
    );
    assert_eq!(
        internal_agent_api(
            &router,
            &auth,
            Method::GET,
            "/api/v1/agents/workspaces/TRON_CORE/internal/avatar-local",
            None
        )
        .await["avatarId"],
        "panda"
    );
    internal_agent_api(
        &router,
        &auth,
        Method::POST,
        &format!("{workspace}/reset-policy"),
        Some(json!({})),
    )
    .await;
    assert_eq!(
        internal_agent_api(&router, &auth, Method::GET, workspace, None).await["avatarId"],
        "dragon"
    );
    for endpoint in [
        format!("{platform}/definition"),
        format!("{workspace}/definition"),
    ] {
        let response = request_with_locale(
            &router,
            Method::PATCH,
            &endpoint,
            Some(json!({"avatarId":"../../outside.png"})),
            &auth,
            "en-US",
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn internal_agent_group_api_persists_catalog_and_guards_builtin_definition_writes() {
    let router = app();
    let auth = format!("Bearer {}", authenticated_bearer_token(&router).await);
    for scope in ["registry", "workspaces/TRON_CORE"] {
        let endpoint = format!("/api/v1/agents/{scope}/internal-groups");
        let created = internal_agent_api(
            &router,
            &auth,
            Method::POST,
            &endpoint,
            Some(json!({"name":" Engineering "})),
        )
        .await;
        assert_eq!(created["name"], "Engineering");
        assert!(
            internal_agent_api(&router, &auth, Method::GET, &endpoint, None)
                .await
                .as_array()
                .unwrap()
                .contains(&json!("Engineering"))
        );
        let definition = format!("/api/v1/agents/{scope}/internal/main-agent/definition");
        for body in [
            json!({"groupName":"Engineering"}),
            json!({"name":"changed"}),
            json!({"avatarId":"fox"}),
        ] {
            let missing_loop = false;
            let denied = request_with_locale(
                &router,
                Method::PATCH,
                &definition,
                Some(body),
                &auth,
                "en-US",
            )
            .await;
            if scope == "registry" {
                assert_eq!(
                    denied.status(),
                    if missing_loop {
                        StatusCode::NOT_FOUND
                    } else {
                        StatusCode::OK
                    }
                );
            } else {
                assert_eq!(denied.status(), StatusCode::FORBIDDEN);
                let denied = response_json(denied).await;
                assert_eq!(denied["code"], "101028017");
                assert!(denied["data"].is_object());
            }
        }
    }
    let created = internal_agent_api(&router, &auth, Method::POST, "/api/v1/agents/registry/internal", Some(json!({"agentCode":"group-api","name":"Group API","rolePrompt":"","groupName":"Engineering"}))).await;
    assert_eq!(created["groupName"], "Engineering");
    assert_eq!(created["builtIn"], false);
    let cleared = internal_agent_api(
        &router,
        &auth,
        Method::PATCH,
        "/api/v1/agents/registry/internal/group-api/definition",
        Some(json!({"groupName":""})),
    )
    .await;
    assert!(cleared["groupName"].is_null());
    let invalid = request_with_locale(
        &router,
        Method::POST,
        "/api/v1/agents/registry/internal-groups",
        Some(json!({"name":""})),
        &auth,
        "zh-CN",
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(response_json(invalid).await["code"], "101028016");
}

#[tokio::test]
async fn category_assignments_persist_and_expand_as_a_union() {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    let response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/tools/categories",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    for body in [
        json!({"toolAssignmentMode":"SelectedCategories", "selectedCategoryCodes":["system.command","readonly"]}),
        json!({}),
    ] {
        let response = request_with_headers(
            &app,
            Method::PATCH,
            "/api/v1/agents/registry/internal/main-agent/policy",
            Some(body),
            None,
            Some(&authorization),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        assert_eq!(
            body["data"]["selectedCategoryCodes"],
            json!(["readonly", "system.command"])
        );
        let codes = body["data"]["selectedToolCodes"].as_array().unwrap();
        assert!(codes.contains(&json!("bash.exec")));
        assert!(codes.contains(&json!("fs.read_text")));
        assert!(!codes.contains(&json!("app.create")));
        let unique = codes
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(codes.len(), unique.len());
    }
    let response = request_with_headers(&app, Method::PATCH, "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/policy", Some(json!({"toolAssignmentMode":"SelectedCategories", "selectedCategoryCodes":["tron.platform"]})), None, Some(&authorization)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(
        body["data"]["selectedCategoryCodes"],
        json!(["tron.platform"])
    );
    let codes = body["data"]["selectedToolCodes"].as_array().unwrap();
    assert!(codes.contains(&json!("app.create")));
    assert!(!codes.contains(&json!("bash.exec")));
}
