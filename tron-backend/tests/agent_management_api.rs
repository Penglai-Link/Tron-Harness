mod common;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    http::{Method, StatusCode},
    routing::post,
};
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};
use tron_backend::config::{AppConfig, SystemAgentConfig};

use common::{app_with_config, authenticated_bearer_token, request_with_headers, response_json};

struct RecordingModelServer {
    base_url: String,
    requests: Arc<Mutex<Vec<Value>>>,
    _task: JoinHandle<()>,
}

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

fn temp_database_url(test_name: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    std::env::temp_dir()
        .join(format!("tron-agent-mgmt-{test_name}-{nanos}.sqlite3"))
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

fn write_fake_command(path: &PathBuf) {
    std::fs::write(path, "#!/bin/sh\nexit 0\n").expect("fake command should be written");
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
    assert!(platform_rows.iter().all(|row| row["status"] == json!("ENABLED")));

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
    assert!(workspace_rows.iter().all(|row| row["agentType"] == json!("SYSTEM")));
    assert!(workspace_rows.iter().all(|row| row["inheritedFrom"] == json!("Platform")));
    assert!(workspace_rows.iter().all(|row| row["agentCode"] != json!("main-agent")));

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
    assert_eq!(disabled_workspace_row["data"]["platformEnabled"], json!(true));
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
    let platform_after_workspace_disable_body = response_json(platform_after_workspace_disable).await;
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
    assert_eq!(enabled_workspace_row["data"]["platformEnabled"], json!(true));
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
    let workspace_after_platform_disable_body = response_json(workspace_after_platform_disable).await;
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
    assert_eq!(unavailable_workspace_mutation.status(), StatusCode::BAD_REQUEST);
    let unavailable_workspace_body = response_json(unavailable_workspace_mutation).await;
    assert_eq!(unavailable_workspace_body["code"], json!("AGENT_SYSTEM_UNAVAILABLE"));
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
    assert_eq!(task_messages[0]["content"], json!("Investigate latency for task."));
}
