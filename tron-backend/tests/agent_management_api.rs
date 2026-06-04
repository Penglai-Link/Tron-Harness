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
    std::env::temp_dir().join(format!("tron-agent-mgmt-{command_name}-{nanos}"))
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
    use std::os::unix::fs::PermissionsExt;

    std::fs::write(path, script).expect("fake command should be written");
    let mut permissions = std::fs::metadata(path)
        .expect("fake command metadata should exist")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("fake command should be executable");
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
async fn startup_scan_workspace_policies_and_rescan_unavailable_state_work() {
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
    let codex_after_rescan = platform_after_rescan_body["data"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["agentCode"] == json!("codex")))
        .expect("codex row should still exist after rescan");
    assert_eq!(codex_after_rescan["status"], json!("UNAVAILABLE"));

    let unavailable_workspace_mutation = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/workspaces/TRON_CORE/system/codex/disable",
        Some(json!({})),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(
        unavailable_workspace_mutation.status(),
        StatusCode::BAD_REQUEST
    );
    let unavailable_workspace_body = response_json(unavailable_workspace_mutation).await;
    assert_eq!(
        unavailable_workspace_body["code"],
        json!("AGENT_SYSTEM_UNAVAILABLE")
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

    let main_run = request_with_headers(
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
        None,
        Some(&authorization),
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
    assert_eq!(main_messages[0]["role"], json!("system"));
    let main_guidance = main_messages[0]["content"]
        .as_str()
        .expect("main-agent guidance should be text");
    assert!(main_guidance.contains("Calm, terse, and surgical."));
    assert!(main_guidance.contains("Summarize the workspace problem before acting."));
    assert_eq!(main_messages[1]["role"], json!("user"));
    assert_eq!(main_messages[1]["content"], json!("Investigate latency."));

    let task_messages = recorded_requests[1]["messages"]
        .as_array()
        .expect("task-agent messages should be an array");
    assert_eq!(task_messages.len(), 1);
    assert_eq!(task_messages[0]["role"], json!("user"));
    assert_eq!(
        task_messages[0]["content"],
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
                        "function": {
                            "name": "discovery.expand_tool_domain",
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
                        "function": {
                            "name": "workspace.list",
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
    assert_eq!(run_response.status(), StatusCode::OK);
    let run_body = response_json(run_response).await;
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
        json!("discovery.expand_tool_domain")
    );
    let first_messages = recorded_requests[0]["messages"]
        .as_array()
        .expect("first request messages should be an array");
    assert!(first_messages.iter().any(|message| {
        message["role"] == json!("system")
            && message["content"]
                .as_str()
                .is_some_and(|content| content.contains("domain_discovery"))
    }));
    assert!(first_messages.iter().any(|message| {
        message["role"] == json!("system")
            && message["content"]
                .as_str()
                .is_some_and(|content| content.contains("\"domainCode\": \"workspace\""))
    }));

    let second_tools = recorded_requests[1]["tools"]
        .as_array()
        .expect("second request tools should be an array");
    assert!(
        second_tools
            .iter()
            .any(|tool| { tool["function"]["name"] == json!("discovery.expand_tool_domain") })
    );
    assert!(
        second_tools
            .iter()
            .any(|tool| tool["function"]["name"] == json!("workspace.list"))
    );
    assert!(
        second_tools
            .iter()
            .any(|tool| tool["function"]["name"] == json!("workspace.get"))
    );
    let second_messages = recorded_requests[1]["messages"]
        .as_array()
        .expect("second request messages should be an array");
    assert!(second_messages.iter().any(|message| {
        message["role"] == json!("user")
            && message["content"]
                .as_str()
                .is_some_and(|content| content.contains("discovery.expand_tool_domain"))
    }));

    let third_messages = recorded_requests[2]["messages"]
        .as_array()
        .expect("third request messages should be an array");
    assert!(third_messages.iter().any(|message| {
        message["role"] == json!("user")
            && message["content"]
                .as_str()
                .is_some_and(|content| content.contains("workspace.list"))
    }));
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
        json!("discovery.expand_tool_domain")
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
    assert!(domains.iter().any(|row| row["domainCode"] == json!("task")));
    assert!(domains.iter().any(|row| row["domainCode"] == json!("bash")));
    assert!(domains.iter().any(|row| row["domainCode"] == json!("fs")));
    assert!(domains.iter().any(|row| row["domainCode"] == json!("search")));

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

    let tool_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/tools/bash.exec",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(tool_response.status(), StatusCode::OK);
    let tool_body = response_json(tool_response).await;
    assert_eq!(tool_body["data"]["toolCode"], json!("bash.exec"));
    assert_eq!(
        tool_body["data"]["inputSchema"]["description"],
        json!("Run one shell command directly when structured tools are not enough.")
    );
    assert!(
        tool_body["data"]["inputSchema"]["examples"]
            .as_array()
            .is_some_and(|examples| !examples.is_empty())
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
    assert_eq!(tool_body["data"]["name"], json!("执行命令"));
    assert_eq!(
        tool_body["data"]["description"],
        json!("在结构化工具不够用时直接执行一条 shell 命令。")
    );
    assert_eq!(
        tool_body["data"]["inputSchema"]["description"],
        json!("在结构化工具不够用时直接执行一条 shell 命令。")
    );
    assert_eq!(
        tool_body["data"]["inputSchema"]["properties"]["command"]["description"],
        json!("要执行的程序名或绝对路径；需要 shell 解析时请使用 bash 并传入 -c。")
    );
    assert!(
        tool_body["data"]["inputSchema"]["examples"]
            .as_array()
            .is_some_and(|examples| !examples.is_empty())
    );
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
    assert!(main_row["selectedToolCodes"]
        .as_array()
        .is_some_and(|rows| rows.iter().any(|code| code == "bash.exec")));
    assert!(main_row["selectedToolCodes"]
        .as_array()
        .is_some_and(|rows| rows.iter().any(|code| code == "fs.read_text")));

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
    assert_eq!(get_body["data"]["toolExposureMode"], json!("DIRECT_LIST"));
    assert_eq!(get_body["data"]["maxSteps"], json!(100));
    assert_eq!(get_body["data"]["maxToolCalls"], json!(100));
    assert_eq!(get_body["data"]["maxParallelToolCalls"], json!(4));

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

    let assign_response = request_with_headers(
        &app,
        Method::PUT,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/tool-assignments",
        Some(json!({
            "toolCodes": ["app.list", "workspace.list", "app.list"]
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
        json!(["app.list", "workspace.list"])
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
                            "name": "workspace.list",
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
            "maxToolCalls": 1
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
                "modelName": "gpt-test"
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(run_response.status(), StatusCode::BAD_REQUEST);
    let run_body = response_json(run_response).await;
    assert_eq!(run_body["code"], json!("AGENT_MAX_STEPS_EXCEEDED"));

    let recorded_requests = model_server
        .requests
        .lock()
        .expect("recorded scripted model request mutex poisoned")
        .clone();
    assert_eq!(recorded_requests.len(), 1);
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
    std::fs::write(&lib_rs, "pub struct ToolRegistry;\n")
        .expect("lib.rs should be written");

    let model_server = scripted_model_server(vec![
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [
                        {
                            "function": {
                                "name": "bash.exec",
                                "arguments": serde_json::to_string(&json!({
                                    "command": "sed",
                                    "args": ["-n", "1,1p", cargo_toml.display().to_string()]
                                })).expect("bash.exec arguments should serialize")
                            }
                        },
                        {
                            "function": {
                                "name": "fs.read_text",
                                "arguments": serde_json::to_string(&json!({
                                    "path": lib_rs.display().to_string(),
                                    "startLine": 1,
                                    "endLine": 1
                                })).expect("fs.read_text arguments should serialize")
                            }
                        },
                        {
                            "function": {
                                "name": "search.glob",
                                "arguments": serde_json::to_string(&json!({
                                    "patterns": ["*.rs"],
                                    "root": workspace_dir.display().to_string()
                                })).expect("search.glob arguments should serialize")
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
                "modelName": "gpt-test"
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(run_response.status(), StatusCode::OK);
    let run_body = response_json(run_response).await;
    assert_eq!(run_body["data"]["finalOutput"], json!("coding tools executed"));

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
            .any(|tool| tool["function"]["name"] == json!("bash.exec"))
    );
    assert!(
        first_tools
            .iter()
            .any(|tool| tool["function"]["name"] == json!("fs.read_text"))
    );
    assert!(
        first_tools
            .iter()
            .any(|tool| tool["function"]["name"] == json!("search.glob"))
    );

    let second_messages = recorded_requests[1]["messages"]
        .as_array()
        .expect("second request messages should be an array");
    assert!(second_messages.iter().any(|message| {
        message["role"] == json!("user")
            && message["content"]
                .as_str()
                .is_some_and(|content| content.contains("bash.exec") && content.contains("[package]"))
    }));
    assert!(second_messages.iter().any(|message| {
        message["role"] == json!("user")
            && message["content"]
                .as_str()
                .is_some_and(|content| content.contains("fs.read_text") && content.contains("ToolRegistry"))
    }));
    assert!(second_messages.iter().any(|message| {
        message["role"] == json!("user")
            && message["content"]
                .as_str()
                .is_some_and(|content| content.contains("search.glob") && content.contains("src/lib.rs"))
    }));
}
