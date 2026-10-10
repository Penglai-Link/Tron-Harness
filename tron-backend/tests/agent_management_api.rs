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
        state.knowledge_bases.stop_ingestion_workers()
    })
}

// Associate native provider results with their exact assistant call, rather than
// treating tool output as a synthetic user message containing a tool-name prefix.
fn provider_tool_result<'a>(messages: &'a [Value], name: &str) -> &'a str {
    let calls: Vec<_> = messages
        .iter()
        .filter(|message| message["role"] == json!("assistant"))
        .filter_map(|message| message["tool_calls"].as_array())
        .flatten()
        .filter(|call| call["function"]["name"] == json!(name))
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
    results[0]["content"]
        .as_str()
        .expect("tool output must be text")
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
    model_completion(&payload, response)
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
        },
        ..AppConfig::default()
    }
}

#[tokio::test]
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

#[tokio::test]
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
        json!([{ "modelId": "gpt-5-codex", "displayName": "GPT-5 Codex" }])
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
        json!([{ "modelId": "gpt-5-codex-next", "displayName": "GPT-5 Codex Next" }])
    );
}

#[tokio::test]
async fn legacy_main_agent_guidance_still_drives_runs_while_definition_api_is_read_only() {
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
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
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
    connection.execute("INSERT INTO platform_internal_agent_configs(agent_code, config_json) VALUES('main-agent',?1)", [legacy.to_string()]).unwrap();
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
    assert_eq!(main_messages[1]["role"], json!("system"));
    let output_guidance = main_messages[1]["content"]
        .as_str()
        .expect("main-agent output guidance should be text");
    assert!(output_guidance.contains("renders Thinking and tool progress separately"));
    assert!(output_guidance.contains("Do not repeat reasoning summaries"));
    assert_eq!(main_messages.last().unwrap()["role"], json!("user"));
    assert_eq!(
        main_messages.last().unwrap()["content"],
        json!("Investigate latency.")
    );

    let task_messages = recorded_requests[1]["messages"]
        .as_array()
        .expect("task-agent messages should be an array");
    assert!(
        serde_json::to_string(task_messages)
            .unwrap()
            .contains("Calm, terse, and surgical.")
    );
    assert_eq!(task_messages.last().unwrap()["role"], json!("user"));
    assert_eq!(
        task_messages.last().unwrap()["content"],
        json!("Investigate latency for task.")
    );

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

#[tokio::test]
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
                        "id": "call_discovery_expand_tool_domain",
                        "type": "function",
                        "function": {
                            "name": "discovery_expand_tool_domain",
                            "arguments": "{\"domainCode\":\"workspace\"}"
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
                            "arguments": "{}"
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
        Some(json!({
            "toolExposureMode": "domain_discovery",
            "expandedDomainPersistenceMode": "sticky_expanded"
        })),
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

    let first_tools = recorded_requests[0]["tools"]
        .as_array()
        .expect("first request tools should be an array");
    assert_eq!(first_tools.len(), 1);
    assert_eq!(
        first_tools[0]["function"]["name"],
        json!("discovery_expand_tool_domain")
    );
    let first_messages = recorded_requests[0]["messages"]
        .as_array()
        .expect("first request messages should be an array");
    assert!(first_messages.iter().any(|message| {
        message["role"] == json!("system")
            && message["content"].as_str().is_some_and(|content| {
                content.contains("domain_discovery")
                    && content.contains("discovery_expand_tool_domain")
                    && (content.contains("Never guess or call a hidden tool")
                        || content.contains("绝不要猜测工具名"))
            })
    }));
    assert!(first_messages.iter().any(|message| {
        message["role"] == json!("system")
            && message["content"]
                .as_str()
                .is_some_and(|content| content.contains("\"domainCode\":\"workspace\""))
    }));

    let second_tools = recorded_requests[1]["tools"]
        .as_array()
        .expect("second request tools should be an array");
    assert!(
        second_tools
            .iter()
            .any(|tool| { tool["function"]["name"] == json!("discovery_expand_tool_domain") })
    );
    assert!(
        second_tools
            .iter()
            .any(|tool| tool["function"]["name"] == json!("workspace_page"))
    );
    assert!(
        second_tools
            .iter()
            .any(|tool| tool["function"]["name"] == json!("workspace_get"))
    );
    for removed_tool in [
        "workspace_create",
        "workspace_update",
        "workspace_delete",
        "workspace_restore",
    ] {
        assert!(
            second_tools
                .iter()
                .all(|tool| tool["function"]["name"] != json!(removed_tool)),
            "{removed_tool} should not be available from workspace domain discovery"
        );
    }
    let second_messages = recorded_requests[1]["messages"]
        .as_array()
        .expect("second request messages should be an array");
    let expansion_result = provider_tool_result(second_messages, "discovery_expand_tool_domain");
    let expansion: Value = serde_json::from_str(expansion_result).expect("expansion result JSON");
    assert_eq!(expansion["status"], json!("expanded"));
    assert_eq!(expansion["domainCode"], json!("workspace"));
    assert!(
        expansion["toolCount"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
    assert!(!expansion_result.contains("inputSchema"));
    assert!(expansion.get("tools").is_none());

    let third_messages = recorded_requests[2]["messages"]
        .as_array()
        .expect("third request messages should be an array");
    let workspace_result: Value =
        serde_json::from_str(provider_tool_result(third_messages, "workspace_page"))
            .expect("workspace page result JSON");
    assert!(
        workspace_result["records"]
            .as_array()
            .is_some_and(|records| !records.is_empty())
    );
}

#[tokio::test]
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
        Some(json!({
            "toolExposureMode": "domain_discovery",
            "expandedDomainPersistenceMode": "sticky_expanded"
        })),
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

    let first_tools = recorded_requests[0]["tools"]
        .as_array()
        .expect("first request tools should be an array");
    assert_eq!(first_tools.len(), 1);
    assert_eq!(
        first_tools[0]["function"]["name"],
        json!("discovery_expand_tool_domain")
    );

    let second_tools = recorded_requests[1]["tools"]
        .as_array()
        .expect("second request tools should be an array");
    assert!(
        second_tools
            .iter()
            .any(|tool| tool["function"]["name"] == json!("fs_write_text"))
    );
    let second_request = recorded_requests[1].to_string();
    assert!(
        second_request.contains("Tool arguments do not match input schema"),
        "{second_request}"
    );
    assert!(second_request.contains("fs_write_text"));
    assert!(!second_request.contains("is not allowed"));
}

#[tokio::test]
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
        Method::POST,
        "/api/v1/skills/workspaces/TRON_CORE/bindings",
        Some(json!({
            "skillName": "progressive-reviewer",
            "enabled": true
        })),
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
                // Direct-list exposes the complete 102-tool schema within the SDK budget.
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
    let first_tools = recorded_requests[0]["tools"]
        .as_array()
        .expect("first request tools should be an array");
    assert!(
        first_tools
            .iter()
            .any(|tool| tool["function"]["name"] == json!("skill_view"))
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

#[tokio::test]
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
        Some(json!({
            "toolExposureMode": "domain_discovery",
            "expandedDomainPersistenceMode": "sticky_expanded"
        })),
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
    assert_eq!(tools.len(), 1);
    assert_eq!(
        tools[0]["function"]["name"],
        json!("discovery_expand_tool_domain")
    );
    assert_eq!(
        tools[0]["function"]["description"],
        json!("展开一个可见工具领域，加载其中可直接调用的具体工具。")
    );
    assert_eq!(
        tools[0]["function"]["parameters"]["properties"]["domainCode"]["description"],
        json!("需要展开的可见领域编码。")
    );

    let messages = recorded_requests[0]["messages"]
        .as_array()
        .expect("messages should be an array");
    assert!(messages.iter().any(|message| {
        message["role"] == json!("system")
            && message["content"].as_str().is_some_and(|content| {
                content.contains("当前 tool exposure mode 为 domain_discovery")
            })
    }));
}

#[tokio::test]
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
    let harness = packs.iter().find(|pack| pack["id"] == "tron.harness").unwrap();
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

#[tokio::test]
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

#[tokio::test]
async fn tool_catalog_read_apis_are_available() {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

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
            .any(|row| row["domainCode"] == json!("workspace"))
    );
    assert!(
        domains
            .iter()
            .any(|row| row["domainCode"] == json!("automation"))
    );
    assert!(domains.iter().any(|row| row["domainCode"] == json!("task")));
    assert!(domains.iter().any(|row| row["domainCode"] == json!("bash")));
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

    let workspace_tools_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/tools?domainCode=workspace",
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
        .find(|tool| tool.call_name == "system_exec")
        .unwrap();
    assert_eq!(tool_body["data"]["inputSchema"], official.input_schema);
    assert!(
        tool_body["data"]["description"]
            .as_str()
            .unwrap()
            .contains("10 seconds")
    );
}

#[tokio::test]
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
                .find(|row| row["domainCode"] == json!("workspace"))
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
    assert_eq!(tool_body["data"]["name"], json!("执行 Shell 命令"));
    assert!(
        tool_body["data"]["description"]
            .as_str()
            .unwrap()
            .contains("10 秒")
    );
    let official = eliteforge_agent_official_plugin_tools_local::local_tool_catalog()
        .into_iter()
        .find(|tool| tool.call_name == "system_exec")
        .unwrap();
    assert_eq!(tool_body["data"]["inputSchema"], official.input_schema);
}

#[tokio::test]
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
    assert_eq!(
        get_body["data"]["toolExposureMode"],
        json!("DOMAIN_DISCOVERY")
    );
    assert_eq!(get_body["data"]["maxSteps"], json!(100));
    assert_eq!(get_body["data"]["maxToolCalls"], json!(100));
    assert_eq!(get_body["data"]["maxParallelToolCalls"], json!(4));

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
            "toolExposureMode": "direct_list",
            "expandedDomainPersistenceMode": "ephemeral_expanded",
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
    assert_eq!(
        patched["data"]["expandedDomainPersistenceMode"],
        json!("EPHEMERAL_EXPANDED")
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
    assert_eq!(reset["data"]["maxParallelToolCalls"], json!(4));
    assert_eq!(reset["data"]["mcpAccessMode"], json!("ALL_ENABLED_SERVERS"));
    assert_eq!(reset["data"]["selectedServerCodes"], json!([]));
}

#[tokio::test]
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
            "toolExposureMode": "direct_list"
        })),
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
            "prompt": "Inspect the workspace list.",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "TEST_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": model_server.base_url,
                "apiKey": "sk-test-api",
                "modelName": "gpt-test",
                // The complete direct-list schema has the same capacity as the
                // other production API fixtures; execution limits stay 1/1.
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

#[tokio::test]
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
                    "modelName": "gpt-test"
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
    let first_run_tools = recorded_model_requests[0]["tools"]
        .as_array()
        .expect("first run tools should be an array");
    assert!(
        first_run_tools
            .iter()
            .any(|tool| tool["function"]["name"] == json!("mcp_knowledge_tools_search_docs"))
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
                "modelName": "gpt-test"
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
    let second_run_tools = second_run_requests[0]["tools"]
        .as_array()
        .expect("second run tools should be an array");
    assert!(
        second_run_tools
            .iter()
            .all(|tool| tool["function"]["name"] != json!("mcp_knowledge_tools_search_docs"))
    );
}

#[tokio::test]
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
                            "id": "call_bash_exec",
                            "type": "function",
                            "function": {
                                "name": "bash_exec",
                                "arguments": serde_json::to_string(&json!({
                                    "command": "sed",
                                    "args": ["-n", "1,1p", cargo_toml.display().to_string()]
                                })).expect("bash_exec arguments should serialize")
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
                // Direct-list exposes the complete 102-tool schema within the SDK budget.
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

    let first_tools = recorded_requests[0]["tools"]
        .as_array()
        .expect("first request tools should be an array");
    assert!(
        first_tools
            .iter()
            .any(|tool| tool["function"]["name"] == json!("bash_exec"))
    );
    assert!(
        first_tools
            .iter()
            .any(|tool| tool["function"]["name"] == json!("fs_read_text"))
    );
    assert!(
        first_tools
            .iter()
            .any(|tool| tool["function"]["name"] == json!("search_glob"))
    );
    assert!(
        first_tools
            .iter()
            .any(|tool| tool["function"]["name"] == json!("search_path"))
    );

    let second_messages = recorded_requests[1]["messages"]
        .as_array()
        .expect("second request messages should be an array");
    let command_result: Value =
        serde_json::from_str(provider_tool_result(second_messages, "bash_exec"))
            .expect("command result JSON");
    assert_eq!(command_result["exitCode"], json!(0));
    assert!(
        command_result["stdout"]
            .as_str()
            .is_some_and(|stdout| stdout.contains("[package]")),
        "{command_result}"
    );
    let file_result: Value =
        serde_json::from_str(provider_tool_result(second_messages, "fs_read_text"))
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

#[tokio::test]
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
                // Direct-list exposes the complete 102-tool schema within the SDK budget.
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
        let page: Value = serde_json::from_str(provider_tool_result(second_messages, name))
            .expect("page result JSON");
        assert_eq!(page["current"], json!(huge_current), "{name}: {page}");
        assert_eq!(page["records"], json!([]), "{name}: {page}");
    }
    let update_result = provider_tool_result(second_messages, "app_update");
    assert!(
        update_result
            .contains("workspaceCode `ENTERPRISE` does not match app workspace `TRON_CORE`"),
        "{update_result}"
    );
    let delete_result = provider_tool_result(second_messages, "task_delete");
    assert!(
        delete_result
            .contains("workspaceCode `ENTERPRISE` does not match task workspace `TRON_CORE`"),
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

#[tokio::test]
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
                // Direct-list exposes the complete 102-tool schema within the SDK budget.
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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
        let first_body = json!({"agentCode":code,"name":"First","rolePrompt":"First role"});
        let second_body = json!({"agentCode":code,"name":"Second","rolePrompt":"Second role"});
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

#[tokio::test]
async fn agent_loop_runs_follow_stage_order_and_new_versions() {
    let server = recording_model_server().await;
    let router = app();
    let auth = format!("Bearer {}", authenticated_bearer_token(&router).await);
    let loop_body = json!({"loopCode":"staged-review","name":"Staged review","kind":"Workflow","nodes":[
        {"id":"prepare","kind":"Model","instructions":"LOOP_PREPARE_UNIQUE","join":"Selected","condition":null},
        {"id":"finish","kind":"Model","instructions":"LOOP_FINISH_UNIQUE","join":"Selected","condition":null}
    ],"edges":[],"maxParallelNodes":1});
    internal_agent_api(
        &router,
        &auth,
        Method::POST,
        "/api/v1/agents/loops",
        Some(loop_body.clone()),
    )
    .await;
    internal_agent_api(&router,&auth,Method::POST,"/api/v1/agents/workspaces/TRON_CORE/internal",Some(json!({"agentCode":"staged-agent","name":"Staged","rolePrompt":"SHARED_AGENT_ROLE","loopCode":"staged-review"}))).await;
    let run = || {
        json!({"prompt":"Check the proposal.","workspaceCode":"TRON_CORE","agentCode":"staged-agent",
        "model":{"modelCode":"TEST_MODEL","protocol":"CHAT_COMPLETIONS","baseUrl":server.base_url,"apiKey":"sk-test","modelName":"gpt-test"}})
    };
    let result = internal_agent_api(
        &router,
        &auth,
        Method::POST,
        "/api/v1/agents/runs",
        Some(run()),
    )
    .await;
    assert_eq!(result["stepCount"], 2);
    let mut update = loop_body;
    update.as_object_mut().unwrap().remove("loopCode");
    update["expectedVersion"] = json!(1);
    update["nodes"][1]["instructions"] = json!("LOOP_FINISH_V2");
    internal_agent_api(
        &router,
        &auth,
        Method::PUT,
        "/api/v1/agents/loops/staged-review",
        Some(update),
    )
    .await;
    internal_agent_api(
        &router,
        &auth,
        Method::POST,
        "/api/v1/agents/runs",
        Some(run()),
    )
    .await;
    let requests = server.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 4);
    for (index, marker) in [
        "LOOP_PREPARE_UNIQUE",
        "LOOP_FINISH_UNIQUE",
        "LOOP_PREPARE_UNIQUE",
        "LOOP_FINISH_V2",
    ]
    .iter()
    .enumerate()
    {
        let messages = requests[index]["messages"].to_string();
        assert!(messages.contains(marker), "{messages}");
        assert!(messages.contains("SHARED_AGENT_ROLE"), "{messages}");
    }
}

#[tokio::test]
async fn agent_loop_dag_branches_skip_unselected_nodes() {
    let server = recording_model_server().await;
    let router = app();
    let auth = format!("Bearer {}", authenticated_bearer_token(&router).await);
    let model_node = |id: &str, prompt: &str| json!({"id":id,"kind":"Model","instructions":prompt,"join":"Selected","condition":null});
    internal_agent_api(&router,&auth,Method::POST,"/api/v1/agents/loops",Some(json!({"loopCode":"branch-review","name":"Branch review","kind":"Dag","nodes":[
        model_node("source","DAG_SOURCE_UNIQUE"),
        {"id":"route","kind":"Branch","instructions":"","join":"Selected","condition":{"sourceNode":"source","pointer":"/absent","equals":true,"matchTarget":"approved","elseTarget":"rejected"}},
        model_node("approved","DAG_APPROVED_UNIQUE"), model_node("rejected","DAG_REJECTED_UNIQUE"),model_node("finish","DAG_FINISH_UNIQUE")
    ],"edges":[{"from":"source","to":"route"},{"from":"route","to":"approved"},{"from":"route","to":"rejected"},{"from":"approved","to":"finish"},{"from":"rejected","to":"finish"}],"maxParallelNodes":2}))).await;
    internal_agent_api(&router,&auth,Method::POST,"/api/v1/agents/workspaces/TRON_CORE/internal",Some(json!({"agentCode":"branch-agent","name":"Branch","rolePrompt":"BRANCH_ROLE","loopCode":"branch-review"}))).await;
    let result=internal_agent_api(&router,&auth,Method::POST,"/api/v1/agents/runs",Some(json!({"prompt":"Check the proposal.","workspaceCode":"TRON_CORE","agentCode":"branch-agent",
        "model":{"modelCode":"TEST_MODEL","protocol":"CHAT_COMPLETIONS","baseUrl":server.base_url,"apiKey":"sk-test","modelName":"gpt-test"}}))).await;
    assert_eq!(result["stepCount"], 3);
    let requests = server.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 3);
    assert!(
        requests[0]["messages"]
            .to_string()
            .contains("DAG_SOURCE_UNIQUE")
    );
    assert!(
        requests[1]["messages"]
            .to_string()
            .contains("DAG_REJECTED_UNIQUE")
    );
    assert!(
        requests[2]["messages"]
            .to_string()
            .contains("DAG_FINISH_UNIQUE")
    );
    assert!(
        !requests
            .iter()
            .any(|r| r["messages"].to_string().contains("DAG_APPROVED_UNIQUE"))
    );
}

#[tokio::test]
async fn agent_loop_dag_runs_parallel_roots_with_isolated_contexts() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    async fn completion(
        axum::extract::State((requests, barrier)): axum::extract::State<(
            Arc<Mutex<Vec<Value>>>,
            Arc<tokio::sync::Barrier>,
        )>,
        Json(payload): Json<Value>,
    ) -> Response {
        let index = {
            let mut requests = requests.lock().unwrap();
            requests.push(payload.clone());
            requests.len()
        };
        if index <= 2 {
            // A serial implementation cannot reach this barrier's second participant.
            tokio::time::timeout(std::time::Duration::from_secs(10), barrier.wait())
                .await
                .expect("DAG roots must execute concurrently");
        }
        model_completion(
            &payload,
            json!({"choices":[{"message":{"role":"assistant","content":format!("parallel-result-{index}")}}]}),
        )
    }
    let server = Router::new()
        .route("/v1/chat/completions", post(completion))
        .with_state((requests.clone(), barrier));
    let task = tokio::spawn(async move {
        axum::serve(listener, server).await.unwrap();
    });
    let router = app();
    let auth = format!("Bearer {}", authenticated_bearer_token(&router).await);
    let node = |id: &str| json!({"id":id,"kind":"Model","instructions":format!("ISOLATED_{id}"),"join":"All","condition":null});
    internal_agent_api(&router, &auth, Method::POST, "/api/v1/agents/loops", Some(json!({
        "loopCode":"parallel-review","name":"Parallel review","kind":"Dag",
        "nodes":[node("left"),node("right"),node("finish")],
        "edges":[{"from":"left","to":"finish"},{"from":"right","to":"finish"}],"maxParallelNodes":2
    }))).await;
    internal_agent_api(&router, &auth, Method::POST, "/api/v1/agents/workspaces/TRON_CORE/internal", Some(json!({"agentCode":"parallel-agent","name":"Parallel","rolePrompt":"Parallel review","loopCode":"parallel-review"}))).await;
    let result = internal_agent_api(&router, &auth, Method::POST, "/api/v1/agents/runs", Some(json!({
        "prompt":"Review this proposal.","workspaceCode":"TRON_CORE","agentCode":"parallel-agent",
        "model":{"modelCode":"TEST_MODEL","protocol":"CHAT_COMPLETIONS","baseUrl":format!("http://{address}/v1"),"apiKey":"sk-test","modelName":"gpt-test"}
    }))).await;
    assert_eq!(result["stepCount"], 3);
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    let roots = requests[..2]
        .iter()
        .map(|r| r["messages"].to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        roots.iter().filter(|s| s.contains("ISOLATED_left")).count(),
        1
    );
    assert_eq!(
        roots
            .iter()
            .filter(|s| s.contains("ISOLATED_right"))
            .count(),
        1
    );
    assert!(roots.iter().all(|s| !s.contains("ISOLATED_finish")));
    let finish = requests[2]["messages"].to_string();
    assert!(
        finish.contains("parallel-result-1") && finish.contains("parallel-result-2"),
        "{finish}"
    );
    assert!(finish.contains("ISOLATED_finish"));
    task.abort();
}

#[tokio::test]
async fn agent_loop_dag_matches_json_fields_from_model_output() {
    let response =
        |content: &str| json!({"choices":[{"message":{"role":"assistant","content":content}}]});
    let server = scripted_model_server(vec![
        response(r#"{"approved":true}"#),
        response("Approved result"),
    ])
    .await;
    let router = app();
    let auth = format!("Bearer {}", authenticated_bearer_token(&router).await);
    let node = |id: &str| json!({"id":id,"kind":"Model","instructions":format!("JSON_BRANCH_{id}"),"join":"Selected","condition":null});
    internal_agent_api(&router, &auth, Method::POST, "/api/v1/agents/loops", Some(json!({
        "loopCode":"json-branch","name":"JSON branch","kind":"Dag","nodes":[node("source"),
        {"id":"route","kind":"Branch","instructions":"","join":"Selected","condition":{"sourceNode":"source","pointer":"/approved","equals":true,"matchTarget":"approved","elseTarget":"rejected"}},
        node("approved"),node("rejected")],"edges":[{"from":"source","to":"route"},{"from":"route","to":"approved"},{"from":"route","to":"rejected"}],"maxParallelNodes":1
    }))).await;
    internal_agent_api(&router, &auth, Method::POST, "/api/v1/agents/workspaces/TRON_CORE/internal", Some(json!({"agentCode":"json-agent","name":"JSON","rolePrompt":"JSON","loopCode":"json-branch"}))).await;
    let result = internal_agent_api(&router, &auth, Method::POST, "/api/v1/agents/runs", Some(json!({
        "prompt":"Review this proposal.","workspaceCode":"TRON_CORE","agentCode":"json-agent",
        "model":{"modelCode":"TEST_MODEL","protocol":"CHAT_COMPLETIONS","baseUrl":server.base_url,"apiKey":"sk-test","modelName":"gpt-test"}
    }))).await;
    assert_eq!(result["stepCount"], 2);
    let requests = server.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(
        requests[1]["messages"]
            .to_string()
            .contains("JSON_BRANCH_approved")
    );
    assert!(
        !requests[1]["messages"]
            .to_string()
            .contains("JSON_BRANCH_rejected")
    );
}

#[tokio::test]
async fn agent_loop_dag_preserves_all_terminal_outputs() {
    let server = recording_model_server().await;
    let router = app();
    let auth = format!("Bearer {}", authenticated_bearer_token(&router).await);
    let node = |id: &str| json!({"id":id,"kind":"Model","instructions":id,"join":"Selected","condition":null});
    internal_agent_api(&router, &auth, Method::POST, "/api/v1/agents/loops", Some(json!({
        "loopCode":"two-results","name":"Two results","kind":"Dag","nodes":[node("first"),node("second")],"edges":[],"maxParallelNodes":2
    }))).await;
    internal_agent_api(&router, &auth, Method::POST, "/api/v1/agents/workspaces/TRON_CORE/internal", Some(json!({"agentCode":"two-results-agent","name":"Two results","rolePrompt":"Review","loopCode":"two-results"}))).await;
    let result = internal_agent_api(&router, &auth, Method::POST, "/api/v1/agents/runs", Some(json!({
        "prompt":"Review this proposal.","workspaceCode":"TRON_CORE","agentCode":"two-results-agent",
        "model":{"modelCode":"TEST_MODEL","protocol":"CHAT_COMPLETIONS","baseUrl":server.base_url,"apiKey":"sk-test","modelName":"gpt-test"}
    }))).await;
    assert_eq!(result["stepCount"], 2);
    let outputs: Value = serde_json::from_str(
        result["finalOutput"]
            .as_str()
            .expect("nonempty terminal outputs"),
    )
    .unwrap();
    assert!(outputs["first"].as_str().is_some());
    assert!(outputs["second"].as_str().is_some());
}

#[tokio::test]
async fn agent_loop_custom_workflow_runs_all_stages() {
    let server = recording_model_server().await;
    let router = app();
    let auth = format!("Bearer {}", authenticated_bearer_token(&router).await);
    let design = serde_json::json!({"loopCode":"review-three","name":"Review","kind":"Workflow","version":0,"updatedAt":0,"maxParallelNodes":1,"edges":[],"nodes":[{"id":"draft","kind":"Model","instructions":"Draft stage","join":"Selected","condition":null},{"id":"review","kind":"Model","instructions":"Review stage","join":"Selected","condition":null},{"id":"finalize","kind":"Model","instructions":"Finalize stage","join":"Selected","condition":null}]});
    internal_agent_api(&router, &auth, Method::POST, "/api/v1/agents/loops", Some(design)).await;
    internal_agent_api(&router, &auth, Method::POST, "/api/v1/agents/workspaces/TRON_CORE/internal", Some(json!({"agentCode":"custom-review","name":"Review","rolePrompt":"CUSTOM_ROLE","loopCode":"review-three"}))).await;
    let result = internal_agent_api(&router, &auth, Method::POST, "/api/v1/agents/runs", Some(json!({
        "prompt":"Draft a proposal.","workspaceCode":"TRON_CORE","agentCode":"custom-review",
        "model":{"modelCode":"TEST_MODEL","protocol":"CHAT_COMPLETIONS","baseUrl":server.base_url,"apiKey":"sk-test","modelName":"gpt-test"}
    }))).await;
    assert_eq!(result["stepCount"], 3);
    let catalog =
        internal_agent_api(&router, &auth, Method::GET, "/api/v1/agents/loops", None).await;
    let design = catalog
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["loopCode"] == "review-three")
        .unwrap();
    let requests = server.requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    for (index, node) in design["nodes"].as_array().unwrap().iter().enumerate() {
        assert!(
            requests[index]["messages"]
                .as_array()
                .unwrap()
                .iter()
                .any(|message| message["content"] == node["instructions"])
        );
        assert!(
            requests[index]["messages"]
                .to_string()
                .contains("CUSTOM_ROLE")
        );
    }
}

#[tokio::test]
async fn agent_loop_custom_analysis_joins_roots_and_selects_each_exit() {
    let app_config = AppConfig { database_url: temp_database_url("custom-analysis-loop"), ..AppConfig::default() };
    for approved in [true, false] {
        let response = |content: String| json!({"choices":[{"message":{"role":"assistant","content":content}}]});
        let design = serde_json::json!({"loopCode":"analysis-dag","name":"Analysis","kind":"Dag","version":0,"updatedAt":0,"maxParallelNodes":2,"nodes":[{"id":"solution","kind":"Model","instructions":"Solution analysis","join":"Selected","condition":null},{"id":"risks","kind":"Model","instructions":"Risk analysis","join":"Selected","condition":null},{"id":"decision","kind":"Model","instructions":"Decision stage","join":"All","condition":null},{"id":"route","kind":"Branch","instructions":"","join":"Selected","condition":{"sourceNode":"decision","pointer":"/approved","equals":true,"matchTarget":"action","elseTarget":"clarify"}},{"id":"action","kind":"Model","instructions":"Action stage","join":"All","condition":null},{"id":"clarify","kind":"Model","instructions":"Clarification stage","join":"All","condition":null}],"edges":[{"from":"solution","to":"decision"},{"from":"risks","to":"decision"},{"from":"decision","to":"route"},{"from":"route","to":"action"},{"from":"route","to":"clarify"},{"from":"decision","to":"action"},{"from":"decision","to":"clarify"}]});
        let server = scripted_model_server(vec![
            response("BUILTIN_ANALYSIS_FIRST".into()),
            response("BUILTIN_ANALYSIS_SECOND".into()),
            response(json!({"approved":approved,"summary":"Decision"}).to_string()),
            response(format!("FINAL_{approved}")),
        ])
        .await;
        let router = app_with_config(app_config.clone());
        let auth = format!("Bearer {}", authenticated_bearer_token(&router).await);
        internal_agent_api(&router, &auth, Method::POST, "/api/v1/agents/loops", Some(design.clone())).await;
        internal_agent_api(&router, &auth, Method::POST, "/api/v1/agents/workspaces/TRON_CORE/internal", Some(json!({"agentCode":"custom-analysis","name":"Analysis","rolePrompt":"CUSTOM_ROLE","loopCode":"analysis-dag"}))).await;
        let result = internal_agent_api(&router, &auth, Method::POST, "/api/v1/agents/runs", Some(json!({
            "prompt":"Review a proposal.","workspaceCode":"TRON_CORE","agentCode":"custom-analysis",
            "model":{"modelCode":"TEST_MODEL","protocol":"CHAT_COMPLETIONS","baseUrl":server.base_url,"apiKey":"sk-test","modelName":"gpt-test"}
        }))).await;
        assert_eq!(result["stepCount"], 4);
        assert_eq!(result["finalOutput"], format!("FINAL_{approved}"));
        let catalog =
            internal_agent_api(&router, &auth, Method::GET, "/api/v1/agents/loops", None).await;
        let design = catalog
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["loopCode"] == "analysis-dag")
            .unwrap();
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 4);
        let merged = requests[2]["messages"].to_string();
        assert!(
            merged.contains("BUILTIN_ANALYSIS_FIRST") && merged.contains("BUILTIN_ANALYSIS_SECOND"),
            "{merged}"
        );
        let target = if approved { "action" } else { "clarify" };
        let other = if approved { "clarify" } else { "action" };
        let instruction = |id| {
            design["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|n| n["id"] == id)
                .unwrap()["instructions"]
                .clone()
        };
        let messages = requests[3]["messages"].as_array().unwrap();
        assert!(
            messages
                .iter()
                .any(|message| message["content"] == instruction(target))
        );
        assert!(
            !messages
                .iter()
                .any(|message| message["content"] == instruction(other))
        );
    }
}

#[tokio::test]
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
    assert_eq!(defaults["avatarId"], "robot");
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

#[tokio::test]
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
        assert_eq!(
            internal_agent_api(&router, &auth, Method::GET, &endpoint, None).await,
            json!(["Engineering"])
        );
        let definition = format!("/api/v1/agents/{scope}/internal/main-agent/definition");
        for body in [
            json!({"groupName":"Engineering"}),
            json!({"name":"changed"}),
            json!({"avatarId":"fox"}),
            json!({"loopCode":"missing-loop"}),
        ] {
            let denied = request_with_locale(
                &router,
                Method::PATCH,
                &definition,
                Some(body),
                &auth,
                "en-US",
            )
            .await;
            assert_eq!(denied.status(), StatusCode::FORBIDDEN);
            let denied = response_json(denied).await;
            assert_eq!(denied["code"], "101028017");
            assert!(denied["data"].is_object());
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
