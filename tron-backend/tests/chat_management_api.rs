mod common;

use std::{
    convert::Infallible,
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
    response::{
        IntoResponse, Response,
        sse::{Event, Sse},
    },
    routing::post,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle, time::sleep};
use tower::ServiceExt;
use tron_backend::{
    adapters::outbound::persistence::SqliteChatRepository,
    application::chat::{ChatService, CreateChatSessionCommand},
    config::{AppConfig, SystemAgentConfig},
    shared::id::FixedSnowflakeIdProvider,
};

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

#[test]
fn chat_session_code_uses_shared_snowflake_id_provider() {
    let database_url = temp_database_url("chat-session-snowflake-id");
    let repository = SqliteChatRepository::connect(database_url.to_str().expect("path utf8"))
        .expect("chat repository should initialize");
    let service = ChatService::with_id_provider(
        repository,
        Arc::new(FixedSnowflakeIdProvider::new(734_200_000_000_000_123)),
    );

    let session = service
        .create_session(
            CreateChatSessionCommand {
                agent_code: Some("main-agent".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                title: Some("Snowflake chat".into()),
                workspace_code: Some("TRON_CORE".into()),
            },
            Some("auth.operator".into()),
        )
        .expect("session should be created");

    assert_eq!(session.id, 734_200_000_000_000_123);
    assert_eq!(session.session_code, "SESSION-734200000000000123");
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
async fn short_term_memory_endpoints_return_uniform_response_for_success_and_bad_extractors() {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let created = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "STM API"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = response_json(created).await;
    let session_code = created_body["data"]["sessionCode"]
        .as_str()
        .expect("session code should be returned");

    let todos = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/chats/sessions/get-todos?sessionCode={session_code}"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(todos.status(), StatusCode::OK);
    let todos_body = response_json(todos).await;
    assert_eq!(todos_body["code"], json!("SUCCESS"));
    assert_eq!(todos_body["data"], json!([]));

    let missing_query = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/chats/sessions/get-todos",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(missing_query.status(), StatusCode::BAD_REQUEST);
    let missing_query_body = response_json(missing_query).await;
    assert_eq!(
        missing_query_body["code"],
        json!("SHORT_TERM_MEMORY_REQUEST_INVALID")
    );
    assert!(missing_query_body["message"].is_string());
    assert_eq!(missing_query_body["data"], json!({}));

    let malformed_json = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/chats/sessions/set-goal")
                .header(header::AUTHORIZATION, authorization)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{"))
                .expect("malformed request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(malformed_json.status(), StatusCode::BAD_REQUEST);
    let malformed_json_body = response_json(malformed_json).await;
    assert_eq!(
        malformed_json_body["code"],
        json!("SHORT_TERM_MEMORY_REQUEST_INVALID")
    );
    assert!(malformed_json_body["message"].is_string());
    assert_eq!(malformed_json_body["data"], json!({}));
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
    let model_code = create_platform_text_model(&app, &authorization, &server.base_url).await;

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
    assert_decimal_code_has_snowflake_shape("SESSION-", &session_code);

    let send_response = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        Some(json!({
            "content": "continue this session",
            "workspaceCode": "TRON_CORE",
            "modelCode": model_code.clone()
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
    let model_code = create_platform_text_model(&app, &authorization, &server.base_url).await;

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
            "modelCode": model_code.clone()
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
    assert!(body.contains("event: REASONING_TEXT_CONTENT"));
    assert!(body.contains("thinking about stream this session"));
    assert!(body.contains("done thinking."));
    assert!(!body.contains("event: REASONING_MESSAGE_CONTENT"));
    assert!(!body.contains("Analyzing request with gpt-chat."));
    assert!(body.contains("event: TEXT_MESSAGE_START"));
    assert!(body.contains("event: TEXT_MESSAGE_CONTENT"));
    assert!(body.contains("\"delta\":\"chat \""));
    assert!(body.contains("\"delta\":\"reply: stream this session\""));
    assert!(
        body.find("event: REASONING_MESSAGE_END")
            .expect("reasoning should end")
            < body
                .find("event: TEXT_MESSAGE_START")
                .expect("text should start")
    );
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
    assert_decimal_code_has_snowflake_shape(
        "MSG-",
        messages["data"][0]["messageCode"]
            .as_str()
            .expect("user message code should be returned"),
    );
    assert_decimal_code_has_snowflake_shape(
        "MSG-",
        messages["data"][1]["messageCode"]
            .as_str()
            .expect("assistant message code should be returned"),
    );
    assert_eq!(
        messages["data"][1]["content"],
        json!("chat reply: stream this session")
    );
}

#[tokio::test]
async fn chat_stream_downgrades_unreachable_mcp_server_to_agui_warning_and_finishes_run() {
    let app = app();
    let server = agent_model_mock_server().await;
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    let model_code = create_platform_text_model(&app, &authorization, &server.base_url).await;
    let hanging_command = temp_command_path("hanging-mcp-runtime");
    write_executable_script(&hanging_command, "#!/bin/sh\nsleep 10\n");

    let create_server = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/mcp/servers",
        Some(json!({
            "serverCode": "hanging_stdio",
            "name": "Hanging Stdio",
            "description": "Blocks during stdio initialize",
            "transportKind": "stdio",
            "url": "",
            "authKind": "none",
            "headers": {},
            "headersVariableRefs": {},
            "command": hanging_command.display().to_string(),
            "args": [],
            "envVariableRefs": {},
            "envPassthroughRefs": [],
            "workingDirectory": hanging_command.parent().map(|path| path.display().to_string())
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(create_server.status(), StatusCode::CREATED);

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

    let session_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Streaming chat with degraded MCP"
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
            "content": "stream with degraded mcp",
            "workspaceCode": "TRON_CORE",
            "modelCode": model_code
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(stream_response.status(), StatusCode::OK);
    let body = to_bytes(stream_response.into_body(), usize::MAX)
        .await
        .expect("stream body should be readable");
    let body = String::from_utf8(body.to_vec()).expect("stream body should be utf8");
    assert!(body.contains("event: TOOL_CALL_START"));
    assert!(body.contains("\"toolCallName\":\"MCP warning\""));
    assert!(body.contains("continuing without its tools"));
    assert!(body.contains("hanging_stdio"));
    assert!(body.contains("event: RUN_STARTED"));
    assert!(body.contains("event: RUN_FINISHED"));
    assert!(body.contains("reply: stream with degraded mcp"));

    let replay_response = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/agents/agui/streams/{session_code}/protocol-events"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(replay_response.status(), StatusCode::OK);
    let replay_json = response_json(replay_response).await.to_string();
    assert!(replay_json.contains("MCP warning"));
    assert!(replay_json.contains("continuing without its tools"));
    assert!(replay_json.contains("RUN_FINISHED"));
}

#[tokio::test]
async fn chat_stream_only_emits_events_for_current_run() {
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
            "modelCode": model_code.clone()
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
            "modelCode": model_code.clone()
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
    let model_code = create_platform_text_model(&app, &authorization, &server.base_url).await;

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
            "modelCode": model_code.clone()
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

#[tokio::test]
async fn chat_session_creation_persists_selected_agent_code() {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "System agent selection",
            "agentCode": "codex"
        })),
        None,
        Some(&authorization),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload = response_json(response).await;
    assert_eq!(payload["data"]["agentCode"], json!("codex"));
}

#[tokio::test]
async fn chat_send_uses_codex_system_agent_cli_runtime() {
    let codex_command = temp_command_path("codex");
    let opencode_command = temp_command_path("opencode");
    write_executable_script(
        &codex_command,
        "#!/bin/sh\necho '{\"type\":\"thread.started\",\"thread_id\":\"thr\"}'\necho '{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"codex system reply\"}}'\necho '{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":11,\"output_tokens\":17}}'\n",
    );
    write_executable_script(
        &opencode_command,
        "#!/bin/sh\necho '{\"type\":\"text\",\"part\":{\"text\":\"opencode fallback\"}}'\necho '{\"type\":\"step_finish\",\"part\":{\"tokens\":{\"input\":3,\"output\":5,\"total\":8}}}'\n",
    );

    let app =
        app_with_system_agent_commands("chat-system-codex", &codex_command, &opencode_command);
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let session_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Codex runtime session",
            "agentCode": "codex"
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
            "content": "execute codex system agent",
            "workspaceCode": "TRON_CORE",
            "systemModelId": "gpt-5-codex"
        })),
        None,
        Some(&authorization),
    )
    .await;

    assert_eq!(send_response.status(), StatusCode::OK);
    let payload = response_json(send_response).await;
    assert_eq!(
        payload["data"]["messages"][1]["content"],
        json!("codex system reply")
    );
    assert_eq!(
        payload["data"]["session"]["systemAgentSessionId"],
        json!("thr")
    );
    assert_eq!(payload["data"]["run"]["systemAgentSessionId"], json!("thr"));
    assert_eq!(
        payload["data"]["run"]["tokenUsage"]["totalTokens"],
        json!(28)
    );
}

#[tokio::test]
async fn chat_send_system_agent_requires_explicit_model_config() {
    let codex_command = temp_command_path("codex-model-required");
    let opencode_command = temp_command_path("opencode-model-required");
    write_executable_script(
        &codex_command,
        "#!/bin/sh\necho '{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"codex fallback\"}}'\necho '{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}'\n",
    );
    write_executable_script(
        &opencode_command,
        "#!/bin/sh\necho '{\"type\":\"text\",\"part\":{\"text\":\"opencode fallback\"}}'\necho '{\"type\":\"step_finish\",\"part\":{\"tokens\":{\"input\":1,\"output\":1,\"total\":2}}}'\n",
    );

    let app = app_with_system_agent_commands(
        "chat-system-model-required",
        &codex_command,
        &opencode_command,
    );
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let session_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "System model required",
            "agentCode": "codex"
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
            "content": "missing system model",
            "workspaceCode": "TRON_CORE",
            "modelCode": "PLATFORM_MODEL"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(send_response.status(), StatusCode::BAD_REQUEST);
    let payload = response_json(send_response).await;
    assert_eq!(payload["code"], json!("AGENT_MODEL_REQUIRED"));
}

#[tokio::test]
async fn chat_send_system_agent_rejects_model_outside_scanned_catalog() {
    let codex_command = temp_command_path("codex-model-scope");
    let opencode_command = temp_command_path("opencode-model-scope");
    write_executable_script(
        &codex_command,
        "#!/bin/sh\nif [ \"$1\" = \"models\" ]; then\n  echo '{\"models\":[{\"id\":\"gpt-5-codex\"}]}'\n  exit 0\nfi\necho '{\"type\":\"thread.started\",\"thread_id\":\"thr\"}'\necho '{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"codex scoped reply\"}}'\necho '{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":2,\"output_tokens\":3}}'\n",
    );
    write_executable_script(
        &opencode_command,
        "#!/bin/sh\necho '{\"type\":\"text\",\"part\":{\"text\":\"opencode fallback\"}}'\necho '{\"type\":\"step_finish\",\"part\":{\"tokens\":{\"input\":1,\"output\":1,\"total\":2}}}'\n",
    );

    let app = app_with_system_agent_commands(
        "chat-system-model-scope",
        &codex_command,
        &opencode_command,
    );
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let session_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "System model scope",
            "agentCode": "codex"
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
            "content": "use unsupported system model",
            "workspaceCode": "TRON_CORE",
            "systemModelId": "gpt-4.1-unsupported"
        })),
        None,
        Some(&authorization),
    )
    .await;

    assert_eq!(send_response.status(), StatusCode::BAD_REQUEST);
    let payload = response_json(send_response).await;
    assert_eq!(payload["code"], json!("AGENT_RUNTIME_FAILED"));
    let message = payload["message"]
        .as_str()
        .expect("error message should be returned");
    assert!(message.contains("is not available for agent"));
}

#[tokio::test]
async fn chat_send_uses_opencode_system_agent_cli_runtime() {
    let codex_command = temp_command_path("codex-open");
    let opencode_command = temp_command_path("opencode-open");
    write_executable_script(
        &codex_command,
        "#!/bin/sh\necho '{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"codex fallback\"}}'\necho '{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":1,\"output_tokens\":2}}'\n",
    );
    write_executable_script(
        &opencode_command,
        "#!/bin/sh\necho '{\"type\":\"step_start\",\"sessionID\":\"ses-opencode-1\",\"part\":{\"id\":\"step\",\"sessionID\":\"ses-opencode-1\"}}'\necho '{\"type\":\"text\",\"part\":{\"text\":\"opencode system reply\"}}'\necho '{\"type\":\"step_finish\",\"part\":{\"tokens\":{\"input\":9,\"output\":4,\"total\":13}}}'\n",
    );

    let app =
        app_with_system_agent_commands("chat-system-opencode", &codex_command, &opencode_command);
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let session_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "OpenCode runtime session",
            "agentCode": "opencode"
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
            "content": "execute opencode system agent",
            "workspaceCode": "TRON_CORE",
            "systemModelId": "openai/gpt-4.1"
        })),
        None,
        Some(&authorization),
    )
    .await;

    assert_eq!(send_response.status(), StatusCode::OK);
    let payload = response_json(send_response).await;
    assert_eq!(
        payload["data"]["messages"][1]["content"],
        json!("opencode system reply")
    );
    assert_eq!(
        payload["data"]["session"]["systemAgentSessionId"],
        json!("ses-opencode-1")
    );
    assert_eq!(
        payload["data"]["run"]["systemAgentSessionId"],
        json!("ses-opencode-1")
    );
    assert_eq!(
        payload["data"]["run"]["tokenUsage"]["totalTokens"],
        json!(13)
    );
}

#[tokio::test]
async fn chat_stream_uses_opencode_system_agent_cli_runtime_and_emits_live_events() {
    let codex_command = temp_command_path("codex-opencode-stream");
    let opencode_command = temp_command_path("opencode-stream");
    write_executable_script(
        &codex_command,
        "#!/bin/sh\necho '{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"codex fallback\"}}'\necho '{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":1,\"output_tokens\":2}}'\n",
    );
    write_executable_script(
        &opencode_command,
        "#!/bin/sh\necho '{\"type\":\"step_start\",\"sessionID\":\"ses-opencode-stream\",\"part\":{\"id\":\"step\",\"sessionID\":\"ses-opencode-stream\"}}'\necho '{\"type\":\"reasoning\",\"sessionID\":\"ses-opencode-stream\",\"part\":{\"text\":\"stream thinking\"}}'\necho '{\"type\":\"text\",\"part\":{\"text\":\"streamed \"}}'\necho '{\"type\":\"text\",\"part\":{\"text\":\"opencode reply\"}}'\necho '{\"type\":\"step_finish\",\"part\":{\"tokens\":{\"input\":10,\"output\":7,\"total\":17}}}'\n",
    );

    let app = app_with_system_agent_commands(
        "chat-system-opencode-stream",
        &codex_command,
        &opencode_command,
    );
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let session_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "OpenCode stream session",
            "agentCode": "opencode"
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
            "content": "stream opencode system agent",
            "workspaceCode": "TRON_CORE",
            "systemModelId": "openai/gpt-4.1"
        })),
        None,
        Some(&authorization),
    )
    .await;

    assert_eq!(stream_response.status(), StatusCode::OK);
    let body = to_bytes(stream_response.into_body(), usize::MAX)
        .await
        .expect("stream body should be readable");
    let body = String::from_utf8(body.to_vec()).expect("stream body should be utf8");
    assert!(body.contains("event: RUN_STARTED"));
    assert!(body.contains("event: REASONING_START"));
    assert!(body.contains("event: REASONING_MESSAGE_CONTENT"));
    assert!(body.contains("\"delta\":\"stream thinking\""));
    assert!(body.contains("event: TEXT_MESSAGE_START"));
    assert!(body.contains("\"delta\":\"streamed \""));
    assert!(body.contains("\"delta\":\"opencode reply\""));
    assert!(
        body.find("event: REASONING_MESSAGE_END")
            .expect("reasoning should end")
            < body
                .find("event: TEXT_MESSAGE_START")
                .expect("text should start")
    );
    assert!(body.contains("event: TEXT_MESSAGE_END"));
    assert!(body.contains("event: RUN_FINISHED"));
    assert!(!body.contains("event: RUN_ERROR"));
}

#[tokio::test]
async fn chat_stream_protocol_replay_survives_client_disconnect() {
    let codex_command = temp_command_path("codex-disconnect-replay");
    let opencode_command = temp_command_path("opencode-disconnect-replay");
    write_executable_script(
        &codex_command,
        "#!/bin/sh\necho '{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"codex fallback\"}}'\necho '{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":1,\"output_tokens\":2}}'\n",
    );
    write_executable_script(
        &opencode_command,
        "#!/bin/sh\necho '{\"type\":\"reasoning\",\"sessionID\":\"ses-disconnect\",\"part\":{\"text\":\"disconnect thinking\"}}'\nsleep 1\necho '{\"type\":\"text\",\"part\":{\"text\":\"disconnect \"}}'\necho '{\"type\":\"text\",\"part\":{\"text\":\"reply\"}}'\necho '{\"type\":\"step_finish\",\"part\":{\"tokens\":{\"input\":11,\"output\":7,\"total\":18}}}'\n",
    );

    let app = app_with_system_agent_commands(
        "chat-system-disconnect-replay",
        &codex_command,
        &opencode_command,
    );
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let session_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Disconnect replay session",
            "agentCode": "opencode"
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
            "content": "stream then disconnect",
            "workspaceCode": "TRON_CORE",
            "systemModelId": "openai/gpt-4.1"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(stream_response.status(), StatusCode::OK);
    drop(stream_response);

    let mut replay_payload = json!(null);
    for _ in 0..40 {
        let response = request_with_headers(
            &app,
            Method::GET,
            &format!("/api/v1/agents/agui/streams/{session_code}/protocol-events"),
            None,
            None,
            Some(&authorization),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        replay_payload = response_json(response).await;
        let serialized = replay_payload.to_string();
        if serialized.contains("RUN_FINISHED") && serialized.contains("disconnect reply") {
            break;
        }
        sleep(Duration::from_millis(50)).await;
    }

    let replay_json = replay_payload.to_string();
    assert!(replay_json.contains("RUN_STARTED"));
    assert!(replay_json.contains("REASONING_MESSAGE_CONTENT"));
    assert!(replay_json.contains("disconnect thinking"));
    assert!(replay_json.contains("TEXT_MESSAGE_CONTENT"));
    assert!(replay_json.contains("disconnect reply"));
    assert!(replay_json.contains("RUN_FINISHED"));
}

#[tokio::test]
async fn chat_send_resumes_codex_system_agent_session() {
    let codex_command = temp_command_path("codex-resume");
    let opencode_command = temp_command_path("opencode-resume");
    let args_log = temp_command_path("codex-resume-args");
    write_executable_script(
        &codex_command,
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"{}\"\nif [ \"$2\" = \"resume\" ]; then\n  echo '{{\"type\":\"thread.started\",\"thread_id\":\"thr-resumed\"}}'\n  echo '{{\"type\":\"item.completed\",\"item\":{{\"type\":\"agent_message\",\"text\":\"codex resumed reply\"}}}}'\nelse\n  echo '{{\"type\":\"thread.started\",\"thread_id\":\"thr-initial\"}}'\n  echo '{{\"type\":\"item.completed\",\"item\":{{\"type\":\"agent_message\",\"text\":\"codex initial reply\"}}}}'\nfi\necho '{{\"type\":\"turn.completed\",\"usage\":{{\"input_tokens\":4,\"output_tokens\":6}}}}'\n",
            args_log.display()
        ),
    );
    write_executable_script(
        &opencode_command,
        "#!/bin/sh\necho '{\"type\":\"text\",\"part\":{\"text\":\"opencode fallback\"}}'\necho '{\"type\":\"step_finish\",\"part\":{\"tokens\":{\"input\":1,\"output\":1,\"total\":2}}}'\n",
    );

    let app = app_with_system_agent_commands(
        "chat-system-codex-resume",
        &codex_command,
        &opencode_command,
    );
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let session_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Codex resume session",
            "agentCode": "codex"
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

    let first_send = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        Some(json!({
            "content": "first codex turn",
            "workspaceCode": "TRON_CORE",
            "systemModelId": "gpt-5-codex"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(first_send.status(), StatusCode::OK);
    let first_payload = response_json(first_send).await;
    assert_eq!(
        first_payload["data"]["messages"][1]["content"],
        json!("codex initial reply")
    );
    assert_eq!(
        first_payload["data"]["session"]["systemAgentSessionId"],
        json!("thr-initial")
    );

    let second_send = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        Some(json!({
            "content": "second codex turn",
            "workspaceCode": "TRON_CORE",
            "systemModelId": "gpt-5-codex"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(second_send.status(), StatusCode::OK);
    let second_payload = response_json(second_send).await;
    assert_eq!(
        second_payload["data"]["messages"][1]["content"],
        json!("codex resumed reply")
    );
    assert_eq!(
        second_payload["data"]["run"]["systemAgentSessionId"],
        json!("thr-resumed")
    );

    let args_lines = std::fs::read_to_string(&args_log).expect("codex args log should be readable");
    assert!(args_lines.contains("exec "));
    assert!(args_lines.contains("--json"));
    assert!(args_lines.contains("--skip-git-repo-check"));
    assert!(args_lines.contains("resume"));
    assert!(args_lines.contains("thr-initial"));
    assert!(!args_lines.contains("--color"));
}

#[tokio::test]
async fn chat_send_resumes_opencode_system_agent_session() {
    let codex_command = temp_command_path("codex-opencode-resume");
    let opencode_command = temp_command_path("opencode-session-resume");
    let args_log = temp_command_path("opencode-resume-args");
    write_executable_script(
        &codex_command,
        "#!/bin/sh\necho '{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"codex fallback\"}}'\necho '{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}'\n",
    );
    let opencode_script = r#"#!/bin/sh
printf '%s\n' "$*" >> "__ARGS_LOG__"
resume=0
session_id="ses-opencode-initial"
for arg in "$@"; do
  if [ "$arg" = "--session" ]; then
    resume=1
  fi
done
if [ "$resume" -eq 1 ]; then
  session_id="ses-opencode-resumed"
fi
echo "{\"type\":\"step_start\",\"sessionID\":\"${session_id}\",\"part\":{\"id\":\"step\",\"sessionID\":\"${session_id}\"}}"
echo '{"type":"text","part":{"text":"opencode resume reply"}}'
echo '{"type":"step_finish","part":{"tokens":{"input":5,"output":3,"total":8}}}'
"#
    .replace("__ARGS_LOG__", &args_log.display().to_string());
    write_executable_script(&opencode_command, &opencode_script);

    let app = app_with_system_agent_commands(
        "chat-system-opencode-resume",
        &codex_command,
        &opencode_command,
    );
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let session_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "OpenCode resume session",
            "agentCode": "opencode"
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

    let first_send = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        Some(json!({
            "content": "first opencode turn",
            "workspaceCode": "TRON_CORE",
            "systemModelId": "openai/gpt-4.1"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(first_send.status(), StatusCode::OK);
    let first_payload = response_json(first_send).await;
    assert_eq!(
        first_payload["data"]["session"]["systemAgentSessionId"],
        json!("ses-opencode-initial")
    );

    let second_send = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        Some(json!({
            "content": "second opencode turn",
            "workspaceCode": "TRON_CORE",
            "systemModelId": "openai/gpt-4.1"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(second_send.status(), StatusCode::OK);
    let second_payload = response_json(second_send).await;
    assert_eq!(
        second_payload["data"]["run"]["systemAgentSessionId"],
        json!("ses-opencode-resumed")
    );

    let args_lines =
        std::fs::read_to_string(&args_log).expect("opencode args log should be readable");
    assert!(args_lines.contains("--session"));
    assert!(args_lines.contains("ses-opencode-initial"));
}

#[tokio::test]
async fn chat_send_resumes_from_runtime_mapping_when_legacy_session_field_is_empty() {
    let database_url = temp_database_url("chat-runtime-mapping-resume");
    let codex_command = temp_command_path("codex-runtime-mapping");
    let opencode_command = temp_command_path("opencode-runtime-mapping");
    let args_log = temp_command_path("codex-runtime-mapping-args");
    write_executable_script(
        &codex_command,
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"{}\"\nif [ \"$2\" = \"resume\" ]; then\n  echo '{{\"type\":\"thread.started\",\"thread_id\":\"thr-resumed\"}}'\n  echo '{{\"type\":\"item.completed\",\"item\":{{\"type\":\"agent_message\",\"text\":\"codex resumed from mapping\"}}}}'\nelse\n  echo '{{\"type\":\"thread.started\",\"thread_id\":\"thr-initial\"}}'\n  echo '{{\"type\":\"item.completed\",\"item\":{{\"type\":\"agent_message\",\"text\":\"codex initial mapping\"}}}}'\nfi\necho '{{\"type\":\"turn.completed\",\"usage\":{{\"input_tokens\":2,\"output_tokens\":3}}}}'\n",
            args_log.display()
        ),
    );
    write_executable_script(
        &opencode_command,
        "#!/bin/sh\necho '{\"type\":\"text\",\"part\":{\"text\":\"opencode fallback\"}}'\necho '{\"type\":\"step_finish\",\"part\":{\"tokens\":{\"input\":1,\"output\":1,\"total\":2}}}'\n",
    );

    let app = app_with_config(AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        system_agents: SystemAgentConfig {
            codex_command: codex_command.display().to_string(),
            opencode_command: opencode_command.display().to_string(),
        },
        ..AppConfig::default()
    });
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let session_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Runtime mapping session",
            "agentCode": "codex"
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

    let first_send = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        Some(json!({
            "content": "mapping first turn",
            "workspaceCode": "TRON_CORE",
            "systemModelId": "gpt-5-codex"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(first_send.status(), StatusCode::OK);
    let first_payload = response_json(first_send).await;
    assert_eq!(
        first_payload["data"]["messages"][1]["content"],
        json!("codex initial mapping")
    );
    assert_eq!(
        first_payload["data"]["session"]["systemAgentSessionId"],
        json!("thr-initial")
    );

    let connection = Connection::open(&database_url).expect("runtime mapping db should reopen");
    let runtime_mapping = connection
        .query_row(
            "SELECT runtime_session_id
             FROM conversation_runtime_sessions
             WHERE conversation_code = ?1
               AND runtime_adapter_key = 'CODEX_EXEC'
               AND agent_code = 'codex'",
            [session_code.as_str()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .expect("runtime mapping query should succeed");
    assert_eq!(runtime_mapping.as_deref(), Some("thr-initial"));
    connection
        .execute(
            "UPDATE chat_sessions SET system_agent_session_id = NULL WHERE session_code = ?1",
            params![session_code],
        )
        .expect("legacy session field should clear");

    let second_send = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        Some(json!({
            "content": "mapping second turn",
            "workspaceCode": "TRON_CORE",
            "systemModelId": "gpt-5-codex"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(second_send.status(), StatusCode::OK);
    let second_payload = response_json(second_send).await;
    assert_eq!(
        second_payload["data"]["messages"][1]["content"],
        json!("codex resumed from mapping")
    );
    assert_eq!(
        second_payload["data"]["run"]["systemAgentSessionId"],
        json!("thr-resumed")
    );

    let args_lines =
        std::fs::read_to_string(&args_log).expect("codex runtime mapping args should be readable");
    assert!(args_lines.contains("exec "));
    assert!(args_lines.contains("resume"));
    assert!(args_lines.contains("thr-initial"));
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

fn app_with_system_agent_commands(
    test_name: &str,
    codex_command: &PathBuf,
    opencode_command: &PathBuf,
) -> Router {
    app_with_config(AppConfig {
        database_url: temp_database_url(test_name).to_string_lossy().into_owned(),
        system_agents: SystemAgentConfig {
            codex_command: codex_command.display().to_string(),
            opencode_command: opencode_command.display().to_string(),
        },
        ..AppConfig::default()
    })
}

fn temp_command_path(command_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("tron-chat-{command_name}-{nanos}"))
}

fn write_executable_script(path: &PathBuf, script: &str) {
    std::fs::write(path, script).expect("script should be written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)
            .expect("script metadata should load")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("script permissions should be updated");
    }
}

fn temp_database_url(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("tron-{test_name}-{nanos}.sqlite3"))
}

fn assert_decimal_code_has_snowflake_shape(prefix: &str, code: &str) {
    let raw_id = code
        .strip_prefix(prefix)
        .unwrap_or_else(|| panic!("code `{code}` should start with `{prefix}`"));
    assert!(
        raw_id.chars().all(|item| item.is_ascii_digit()),
        "code `{code}` should carry a decimal id"
    );
    let value = raw_id
        .parse::<i64>()
        .unwrap_or_else(|error| panic!("code `{code}` should parse as i64: {error}"));
    assert!(
        value > 1_000_000_000_000,
        "code `{code}` should use a generated snowflake id, not a table-local sequence"
    );
}
