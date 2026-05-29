mod common;

use std::{convert::Infallible, path::PathBuf};

use axum::{
    Json, Router,
    body::to_bytes,
    http::{Method, StatusCode},
    response::{
        IntoResponse, Response,
        sse::{Event, Sse},
    },
    routing::post,
};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};
use tron_backend::config::AppConfig;

use common::{
    app, app_with_config, authenticated_bearer_token, request_with_headers, response_json,
};

struct AgentModelMockServer {
    base_url: String,
    _task: JoinHandle<()>,
}

async fn agent_model_mock_server() -> AgentModelMockServer {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("agent mock server should bind");
    let address = listener
        .local_addr()
        .expect("agent mock server should expose address");
    let app = Router::new().route("/v1/chat/completions", post(agent_chat_completion));
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("agent mock server should stay available");
    });
    AgentModelMockServer {
        base_url: format!("http://{address}/v1"),
        _task: task,
    }
}

async fn agent_chat_completion(Json(payload): Json<Value>) -> Response {
    let prompt = payload["messages"]
        .as_array()
        .and_then(|messages| {
            messages
                .iter()
                .rev()
                .find_map(|message| message["content"].as_str())
        })
        .unwrap_or_default();
    if payload["stream"].as_bool() == Some(true) {
        assert_eq!(payload["enable_thinking"], json!(true));
        let reasoning = json!({
            "choices": [{
                "delta": {
                    "reasoning_content": format!(
                        "thinking about {prompt}. {}done thinking.",
                        "reasoning detail ".repeat(12)
                    )
                }
            }]
        });
        let first = json!({
            "choices": [{
                "delta": {
                    "content": "chat "
                }
            }]
        });
        let second = json!({
            "choices": [{
                "delta": {
                    "content": format!("reply: {prompt}")
                }
            }]
        });
        let stream = async_stream::stream! {
            yield Ok::<Event, Infallible>(Event::default().data(reasoning.to_string()));
            yield Ok::<Event, Infallible>(Event::default().data(first.to_string()));
            yield Ok::<Event, Infallible>(Event::default().data(second.to_string()));
            yield Ok::<Event, Infallible>(Event::default().data("[DONE]"));
        };
        return Sse::new(stream).into_response();
    }
    Json(json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": format!("chat reply: {prompt}")
            }
        }]
    }))
    .into_response()
}

#[tokio::test]
async fn chat_session_groups_and_messages_are_persisted_history() {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let assigned = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "taskCode": "TASK-ALPHA",
            "title": "Assigned chat"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(assigned.status(), StatusCode::CREATED);
    let assigned_body = response_json(assigned).await;
    let assigned_session = assigned_body["data"]["sessionCode"]
        .as_str()
        .expect("session code should be returned")
        .to_string();

    let unassigned = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Unassigned chat"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(unassigned.status(), StatusCode::CREATED);

    let groups_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/chats/session-groups?workspaceCode=TRON_CORE",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(groups_response.status(), StatusCode::OK);
    let groups = response_json(groups_response).await;
    assert_eq!(groups["code"], json!("SUCCESS"));
    assert!(
        groups["data"]
            .as_array()
            .expect("groups should be an array")
            .iter()
            .any(|group| group["taskCode"] == json!("TASK-ALPHA"))
    );
    assert!(
        groups["data"]
            .as_array()
            .expect("groups should be an array")
            .iter()
            .any(|group| group["taskTitle"] == json!("Chat"))
    );

    let messages_response = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/chats/sessions/{assigned_session}/messages"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(messages_response.status(), StatusCode::OK);
    let messages = response_json(messages_response).await;
    assert_eq!(messages["data"], json!([]));
}

#[tokio::test]
async fn chat_message_history_requires_an_existing_session() {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/chats/sessions/SESSION-MISSING/messages",
        None,
        None,
        Some(&authorization),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response_json(response).await;
    assert_eq!(body["code"], json!("CHAT_SESSION_NOT_FOUND"));
}

#[tokio::test]
async fn chat_session_groups_do_not_require_workspace_access_grants() {
    let database_url = temp_database_url("chat-admin-workspace-access");
    let router = app_with_config(AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    });
    seed_workspace_without_access(&database_url, "WS-CHAT");
    let token = authenticated_bearer_token(&router).await;
    let authorization = format!("Bearer {token}");

    let response = request_with_headers(
        &router,
        Method::GET,
        "/api/v1/chats/session-groups?workspaceCode=WS-CHAT",
        None,
        None,
        Some(&authorization),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["code"], json!("SUCCESS"));
    assert_eq!(body["data"], json!([]));
}

#[tokio::test]
async fn chat_send_persists_messages_and_invokes_agent_run() {
    let app = app();
    let server = agent_model_mock_server().await;
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let session_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Run chat"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(session_response.status(), StatusCode::CREATED);
    let session = response_json(session_response).await;
    let session_code = session["data"]["sessionCode"]
        .as_str()
        .expect("session code should be returned")
        .to_string();

    let send_response = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        Some(json!({
            "content": "continue this session",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "CHAT_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": server.base_url,
                "apiKey": "sk-chat-api",
                "modelName": "gpt-chat",
                "thinkingEnabled": true
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(send_response.status(), StatusCode::OK);
    let sent = response_json(send_response).await;
    let run_code = sent["data"]["run"]["runCode"]
        .as_str()
        .expect("run code should be returned")
        .to_string();
    assert_eq!(sent["data"]["run"]["conversationCode"], json!(session_code));
    assert_eq!(sent["data"]["messages"][0]["role"], json!("USER"));
    assert_eq!(
        sent["data"]["messages"][0]["content"],
        json!("continue this session")
    );
    assert_eq!(sent["data"]["messages"][1]["role"], json!("ASSISTANT"));
    assert_eq!(
        sent["data"]["messages"][1]["content"],
        json!("chat reply: continue this session")
    );
    assert_eq!(sent["data"]["session"]["lastRunCode"], json!(run_code));

    let history_response = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/agents/runs?conversationCode={session_code}"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(history_response.status(), StatusCode::OK);
    let history = response_json(history_response).await;
    assert_eq!(history["data"]["total"], json!(1));
    assert_eq!(history["data"]["records"][0]["runCode"], json!(run_code));
    assert!(!sent.to_string().contains("sk-chat-api"));
}

#[tokio::test]
async fn chat_stream_persists_messages_and_emits_deltas() {
    let app = app();
    let server = agent_model_mock_server().await;
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let session_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Streaming chat"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(session_response.status(), StatusCode::CREATED);
    let session = response_json(session_response).await;
    let session_code = session["data"]["sessionCode"]
        .as_str()
        .expect("session code should be returned")
        .to_string();

    let stream_response = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages/stream"),
        Some(json!({
            "content": "stream this session",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "CHAT_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": server.base_url,
                "apiKey": "sk-chat-api",
                "modelName": "gpt-chat",
                "thinkingEnabled": true
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(stream_response.status(), StatusCode::OK);
    assert!(
        stream_response.headers()["content-type"]
            .to_str()
            .expect("content-type should be valid")
            .starts_with("text/event-stream")
    );
    let body = to_bytes(stream_response.into_body(), usize::MAX)
        .await
        .expect("stream body should be readable");
    let body = String::from_utf8(body.to_vec()).expect("stream body should be utf8");
    assert!(body.contains("event: RUN_STARTED"));
    assert!(body.contains("\"type\":\"RUN_STARTED\""));
    assert!(body.contains("event: REASONING_MESSAGE_CONTENT"));
    assert!(body.contains("thinking about stream this session"));
    assert!(body.contains("done thinking."));
    assert!(body.matches("event: REASONING_MESSAGE_CONTENT").count() > 1);
    assert!(!body.contains("Analyzing request with gpt-chat."));
    assert!(body.contains("event: TEXT_MESSAGE_START"));
    assert!(body.contains("event: TEXT_MESSAGE_CONTENT"));
    assert!(body.contains("\"delta\":\"chat \""));
    assert!(body.contains("\"delta\":\"reply: stream this session\""));
    assert!(body.contains("event: TEXT_MESSAGE_END"));
    assert!(body.contains("event: RUN_FINISHED"));
    assert!(body.contains("\"type\":\"RUN_FINISHED\""));
    assert!(!body.contains("event: delta"));
    assert!(!body.contains("event: done"));

    let messages_response = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(messages_response.status(), StatusCode::OK);
    let messages = response_json(messages_response).await;
    assert_eq!(messages["data"][0]["role"], json!("USER"));
    assert_eq!(messages["data"][1]["role"], json!("ASSISTANT"));
    assert_eq!(
        messages["data"][1]["content"],
        json!("chat reply: stream this session")
    );
}

#[tokio::test]
async fn chat_stream_only_emits_events_for_current_run() {
    let app = app();
    let server = agent_model_mock_server().await;
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let session_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Replay guard chat"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(session_response.status(), StatusCode::CREATED);
    let session = response_json(session_response).await;
    let session_code = session["data"]["sessionCode"]
        .as_str()
        .expect("session code should be returned")
        .to_string();

    let first_stream_response = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages/stream"),
        Some(json!({
            "content": "first stream replay guard",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "CHAT_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": server.base_url,
                "apiKey": "sk-chat-api",
                "modelName": "gpt-chat",
                "thinkingEnabled": true
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(first_stream_response.status(), StatusCode::OK);
    let first_body = to_bytes(first_stream_response.into_body(), usize::MAX)
        .await
        .expect("first stream body should be readable");
    let first_body = String::from_utf8(first_body.to_vec()).expect("first stream should be utf8");
    assert!(first_body.contains("\"delta\":\"reply: first stream replay guard\""));

    let second_stream_response = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages/stream"),
        Some(json!({
            "content": "second stream replay guard",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "CHAT_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": server.base_url,
                "apiKey": "sk-chat-api",
                "modelName": "gpt-chat",
                "thinkingEnabled": true
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(second_stream_response.status(), StatusCode::OK);
    let second_body = to_bytes(second_stream_response.into_body(), usize::MAX)
        .await
        .expect("second stream body should be readable");
    let second_body =
        String::from_utf8(second_body.to_vec()).expect("second stream should be utf8");
    assert!(second_body.contains("\"delta\":\"reply: second stream replay guard\""));
    assert!(!second_body.contains("first stream replay guard"));
}

#[tokio::test]
async fn chat_send_does_not_reuse_existing_run_codes_after_restart() {
    let database_url = temp_database_url("chat-run-code-collision");
    let mut config = AppConfig::default();
    config.database_url = database_url.to_string_lossy().into_owned();
    let app = app_with_config(config);
    seed_agent_run_codes(&database_url);

    let server = agent_model_mock_server().await;
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let session_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Run code collision"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(session_response.status(), StatusCode::CREATED);
    let session = response_json(session_response).await;
    let session_code = session["data"]["sessionCode"]
        .as_str()
        .expect("session code should be returned")
        .to_string();

    let send_response = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        Some(json!({
            "content": "avoid a restarted run code collision",
            "workspaceCode": "TRON_CORE",
            "model": {
                "modelCode": "CHAT_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": server.base_url,
                "apiKey": "sk-chat-api",
                "modelName": "gpt-chat"
            }
        })),
        None,
        Some(&authorization),
    )
    .await;

    assert_eq!(send_response.status(), StatusCode::OK);
    let sent = response_json(send_response).await;
    let run_code = sent["data"]["run"]["runCode"]
        .as_str()
        .expect("run code should be returned");
    assert!(run_code.starts_with("RUN-"));
    assert_ne!(run_code, "RUN-1");
    assert_eq!(
        sent["data"]["messages"][1]["content"],
        json!("chat reply: avoid a restarted run code collision")
    );
}

#[tokio::test]
async fn chat_send_resolves_model_code_through_model_management() {
    let app = app();
    let server = agent_model_mock_server().await;
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    let model_code = create_platform_text_model(&app, &authorization, &server.base_url).await;

    let session_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Model code chat"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(session_response.status(), StatusCode::CREATED);
    let session = response_json(session_response).await;
    let session_code = session["data"]["sessionCode"]
        .as_str()
        .expect("session code should be returned")
        .to_string();

    let send_response = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        Some(json!({
            "content": "use the selected model",
            "workspaceCode": "TRON_CORE",
            "modelCode": model_code
        })),
        None,
        Some(&authorization),
    )
    .await;

    assert_eq!(send_response.status(), StatusCode::OK);
    let sent = response_json(send_response).await;
    assert_eq!(
        sent["data"]["messages"][1]["content"],
        json!("chat reply: use the selected model")
    );
    assert!(!sent.to_string().contains("sk-chat-api"));
}

fn seed_workspace_without_access(database_url: &PathBuf, workspace_code: &str) {
    let connection = Connection::open(database_url).expect("chat test db should reopen");
    connection
        .execute(
            "INSERT INTO workspaces (id, workspace_code, name, description, parent_workspace_code, status, created_at, updated_at, archived_at)
             VALUES (?1, ?2, ?3, NULL, NULL, 'active', ?4, ?4, NULL)",
            params![
                735_200_000_000_030_001_i64,
                workspace_code,
                "Admin visible workspace",
                1_779_235_200_000_i64,
            ],
        )
        .expect("workspace without direct access should insert");
}

fn seed_agent_run_codes(database_url: &PathBuf) {
    let connection = Connection::open(database_url).expect("chat test db should reopen");
    for index in 1..=5_i64 {
        connection
            .execute(
                "INSERT INTO agent_runs (
                    id, run_code, conversation_code, workspace_code, user_code, app_code, task_code,
                    model_code, status, outcome, step_count, observation_count, output_preview,
                    error_code, error_message, started_at, finished_at, created_at
                 )
                 VALUES (?1, ?2, NULL, 'TRON_CORE', 'USR-1', NULL, NULL, 'CHAT_MODEL', 'FAILED',
                    'failed', 0, 0, NULL, 'SEEDED', 'seeded run', ?3, ?3, ?3)",
                params![
                    735_100_000_000_010_000_i64 + index,
                    format!("RUN-{index}"),
                    1_779_235_200_000_i64 + index,
                ],
            )
            .expect("seeded agent run should insert");
    }
}

async fn create_platform_text_model(app: &Router, authorization: &str, base_url: &str) -> String {
    let response = request_with_headers(
        app,
        Method::POST,
        "/api/v1/admin/platform/models",
        Some(json!({
            "displayName": "Chat Default",
            "vendor": "OpenAI",
            "protocol": "OPENAI_COMPATIBLE",
            "modelType": "TEXT",
            "modelCapabilities": ["TEXT_GENERATION"],
            "baseUrl": base_url,
            "apiPath": "/chat/completions",
            "apiKeyRef": "sk-chat-api",
            "modelName": "gpt-chat",
            "enabled": true,
            "description": "Chat model",
            "thinkingEnabled": true,
            "structuredOutputEnabled": false,
            "temperature": 0.2,
            "maxTokens": 1024,
            "topP": 1.0,
            "timeoutMs": 1000,
            "maxRetries": 0
        })),
        None,
        Some(authorization),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let payload = response_json(response).await;
    payload["data"]["modelCode"]
        .as_str()
        .expect("created model should return modelCode")
        .to_string()
}

fn temp_database_url(test_name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("tron-{test_name}-{nanos}.sqlite3"))
}
