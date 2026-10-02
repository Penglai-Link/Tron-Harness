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
    routing::post,
};
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};
use tower::ServiceExt;
use tron_backend::config::{AppConfig, SystemAgentConfig};

use common::{
    app, app_with_config, authenticated_bearer_token, request_with_headers, response_json,
};

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

async fn record_chat_completion(
    axum::extract::State(requests): axum::extract::State<Arc<Mutex<Vec<Value>>>>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    requests
        .lock()
        .expect("recording model request mutex poisoned")
        .push(payload.clone());
    let prompt = payload["messages"]
        .as_array()
        .and_then(|messages| messages.last())
        .and_then(|message| message["content"].as_str())
        .unwrap_or_default();
    Json(json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": format!("mock: {prompt}")
            }
        }]
    }))
}

async fn scripted_chat_completion(
    axum::extract::State((requests, responses)): axum::extract::State<(
        Arc<Mutex<Vec<Value>>>,
        Arc<Mutex<VecDeque<Value>>>,
    )>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    requests
        .lock()
        .expect("recording model request mutex poisoned")
        .push(payload);
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
    Json(response)
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
async fn main_agent_config_is_persisted_and_only_main_agent_runs_receive_guidance() {
    let codex_command = temp_command_path("codex-main");
    let opencode_command = temp_command_path("opencode-main");
    write_fake_command(&codex_command);
    write_fake_command(&opencode_command);

    let model_server = recording_model_server().await;
    let app = app_with_config(test_config(&codex_command, &opencode_command));
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let update_response = request_with_headers(
        &app,
        Method::PATCH,
        "/api/v1/agents/registry/main-agent",
        Some(json!({
            "personality": "Calm, terse, and surgical.",
            "instructions": "Summarize the workspace problem before acting."
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(update_response.status(), StatusCode::OK);
    let update_body = response_json(update_response).await;
    assert_eq!(
        update_body["data"]["personality"],
        json!("Calm, terse, and surgical.")
    );

    let get_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/agents/registry/main-agent",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(get_response.status(), StatusCode::OK);
    let get_body = response_json(get_response).await;
    assert_eq!(
        get_body["data"]["instructions"],
        json!("Summarize the workspace problem before acting.")
    );

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
    assert_eq!(main_messages.len(), 4, "{main_messages:?}");
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
    assert_eq!(task_messages.len(), 2);
    assert!(
        !serde_json::to_string(task_messages)
            .unwrap()
            .contains("Calm, terse, and surgical.")
    );
    assert_eq!(task_messages.last().unwrap()["role"], json!("user"));
    assert_eq!(
        task_messages.last().unwrap()["content"],
        json!("Investigate latency for task.")
    );
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
    assert!(tool_description.contains(tron_agent_harness::TOOL_IMPLEMENTATION_DESCRIPTION));
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
