mod common;

use std::{
    collections::VecDeque,
    convert::Infallible,
    future::poll_fn,
    path::PathBuf,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    body::{Body, HttpBody, to_bytes},
    extract::State,
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

#[derive(Clone)]
struct ScriptedAgentModelMockState {
    responses: Arc<Mutex<VecDeque<String>>>,
}

#[derive(Clone)]
struct DelayedSteerModelMockState {
    calls: Arc<AtomicUsize>,
    requests: Arc<Mutex<Vec<Value>>>,
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

async fn delayed_agent_model_mock_server() -> AgentModelMockServer {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("delayed agent mock server should bind");
    let address = listener
        .local_addr()
        .expect("delayed agent mock server should expose address");
    let app = Router::new().route(
        "/v1/chat/completions",
        post(|Json(payload): Json<Value>| async move {
            sleep(Duration::from_millis(750)).await;
            let prompt = payload["messages"]
                .as_array()
                .and_then(|messages| {
                    messages
                        .iter()
                        .rev()
                        .find_map(|message| message["content"].as_str())
                })
                .unwrap_or_default();
            Json(json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": format!("delayed reply: {prompt}")
                    }
                }]
            }))
        }),
    );
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("delayed agent mock server should stay available");
    });
    AgentModelMockServer {
        base_url: format!("http://{address}/v1"),
        _task: task,
    }
}

async fn scripted_agent_model_mock_server(responses: Vec<String>) -> AgentModelMockServer {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("agent mock server should bind");
    let address = listener
        .local_addr()
        .expect("agent mock server should expose address");
    let state = ScriptedAgentModelMockState {
        responses: Arc::new(Mutex::new(VecDeque::from(responses))),
    };
    let app = Router::new()
        .route("/v1/chat/completions", post(scripted_agent_chat_completion))
        .with_state(state);
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

async fn delayed_steer_model_mock_server() -> (AgentModelMockServer, DelayedSteerModelMockState) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("steer model server should bind");
    let address = listener
        .local_addr()
        .expect("steer model server should expose address");
    let state = DelayedSteerModelMockState {
        calls: Arc::new(AtomicUsize::new(0)),
        requests: Arc::new(Mutex::new(Vec::new())),
    };
    let app = Router::new()
        .route("/v1/chat/completions", post(delayed_steer_chat_completion))
        .with_state(state.clone());
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("steer model server should stay available");
    });
    (
        AgentModelMockServer {
            base_url: format!("http://{address}/v1"),
            _task: task,
        },
        state,
    )
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

async fn scripted_agent_chat_completion(
    State(state): State<ScriptedAgentModelMockState>,
    Json(payload): Json<Value>,
) -> Response {
    assert_ne!(payload["stream"].as_bool(), Some(true));
    let content = state
        .responses
        .lock()
        .expect("scripted mock responses should lock")
        .pop_front()
        .expect("scripted mock response should exist");
    Json(json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": content
            }
        }]
    }))
    .into_response()
}

async fn delayed_steer_chat_completion(
    State(state): State<DelayedSteerModelMockState>,
    Json(payload): Json<Value>,
) -> Response {
    state
        .requests
        .lock()
        .expect("steer model requests mutex should lock")
        .push(payload);
    let call = state.calls.fetch_add(1, Ordering::SeqCst);
    if call == 0 {
        sleep(Duration::from_millis(750)).await;
    }
    let content = if call == 0 {
        "first boundary output"
    } else {
        "steered final output"
    };
    let chunk = json!({
        "choices": [{
            "delta": {"content": content}
        }]
    });
    let stream = async_stream::stream! {
        yield Ok::<Event, Infallible>(Event::default().data(chunk.to_string()));
        yield Ok::<Event, Infallible>(Event::default().data("[DONE]"));
    };
    Sse::new(stream).into_response()
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
async fn terminal_sessions_are_hidden_from_chat_management_and_can_be_deleted() {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let created = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "sourceType": "TERMINAL",
            "workspaceCode": "TRON_CORE"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = response_json(created).await;
    let session_code = created["data"]["sessionCode"]
        .as_str()
        .expect("terminal session code should be returned")
        .to_string();
    assert_eq!(created["data"]["sourceType"], json!("TERMINAL"));

    let groups = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/chats/session-groups?workspaceCode=TRON_CORE",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(groups.status(), StatusCode::OK);
    let groups = response_json(groups).await;
    assert!(
        groups["data"]
            .as_array()
            .expect("groups should be an array")
            .iter()
            .flat_map(|group| group["sessions"].as_array().into_iter().flatten())
            .all(|session| session["sessionCode"] != session_code)
    );

    let deleted = request_with_headers(
        &app,
        Method::DELETE,
        &format!("/api/v1/chats/sessions/{session_code}"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::OK);

    let messages = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(messages.status(), StatusCode::NOT_FOUND);
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
async fn chat_steer_endpoint_uses_body_contract_and_rejects_when_no_run_is_active() {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    let created = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Steer contract"
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

    let response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions/steer-message",
        Some(json!({
            "sessionCode": session_code,
            "expectedRunCode": "RUN-NOT-ACTIVE",
            "content": "steer now",
            "attachments": []
        })),
        None,
        Some(&authorization),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body = response_json(response).await;
    assert_eq!(body["code"], json!("CHAT_STEER_NO_ACTIVE_RUN"));
    assert_eq!(body["data"], json!({}));
}

#[tokio::test]
async fn chat_run_control_routes_use_query_and_strict_body_contracts() {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    let created = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Run control contract"
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

    let active = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/chats/sessions/get-active-run?sessionCode={session_code}"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(active.status(), StatusCode::OK);
    let active_body = response_json(active).await;
    assert_eq!(active_body["data"]["sessionCode"], json!(session_code));
    assert_eq!(active_body["data"]["activeRun"], Value::Null);

    let malformed_query = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/chats/sessions/get-active-run?unexpected=true",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(malformed_query.status(), StatusCode::BAD_REQUEST);
    let malformed_query_body = response_json(malformed_query).await;
    assert_eq!(
        malformed_query_body["code"],
        json!("CHAT_ACTIVE_RUN_QUERY_INVALID")
    );
    assert_eq!(malformed_query_body["data"], json!({}));

    let missing_query = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/chats/sessions/get-active-run",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(missing_query.status(), StatusCode::BAD_REQUEST);
    let missing_query_body = response_json(missing_query).await;
    assert_eq!(
        missing_query_body["code"],
        json!("CHAT_RUN_CONTROL_SESSION_CODE_REQUIRED")
    );

    let missing_session = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/chats/sessions/get-active-run?sessionCode=SESSION-MISSING",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(missing_session.status(), StatusCode::NOT_FOUND);
    let missing_session_body = response_json(missing_session).await;
    assert_eq!(
        missing_session_body["code"],
        json!("CHAT_RUN_CONTROL_SESSION_NOT_FOUND")
    );

    let missing_expected_run = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions/stop-run",
        Some(json!({"sessionCode": session_code})),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(missing_expected_run.status(), StatusCode::BAD_REQUEST);
    let missing_expected_run_body = response_json(missing_expected_run).await;
    assert_eq!(
        missing_expected_run_body["code"],
        json!("CHAT_RUN_CONTROL_EXPECTED_RUN_CODE_REQUIRED")
    );

    let invalid = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions/stop-run",
        Some(json!({
            "sessionCode": session_code,
            "expectedRunCode": "RUN-NOT-ACTIVE",
            "unexpected": true
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    let invalid_body = response_json(invalid).await;
    assert_eq!(invalid_body["code"], json!("CHAT_STOP_RUN_REQUEST_INVALID"));
    assert_eq!(invalid_body["data"], json!({}));

    let no_active = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions/stop-run",
        Some(json!({
            "sessionCode": session_code,
            "expectedRunCode": "RUN-NOT-ACTIVE"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(no_active.status(), StatusCode::CONFLICT);
    let no_active_body = response_json(no_active).await;
    assert_eq!(
        no_active_body["code"],
        json!("CHAT_RUN_CONTROL_NO_ACTIVE_RUN")
    );

    let invalid_steer = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions/steer-message",
        Some(json!({
            "sessionCode": session_code,
            "expectedRunCode": "RUN-NOT-ACTIVE",
            "content": "invalid extra field",
            "attachments": [],
            "unexpected": true
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(invalid_steer.status(), StatusCode::BAD_REQUEST);
    let invalid_steer_body = response_json(invalid_steer).await;
    assert_eq!(
        invalid_steer_body["code"],
        json!("CHAT_STEER_REQUEST_INVALID")
    );
    assert_eq!(invalid_steer_body["data"], json!({}));
}

#[tokio::test]
async fn generic_agent_reservation_blocks_chat_before_user_message_persistence() {
    let app = app();
    let server = delayed_agent_model_mock_server().await;
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    let created = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Shared agent admission"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = response_json(created).await;
    let session_code = created_body["data"]["sessionCode"]
        .as_str()
        .expect("session code should be returned")
        .to_string();

    let run_app = app.clone();
    let run_authorization = authorization.clone();
    let run_session_code = session_code.clone();
    let run_base_url = server.base_url.clone();
    let generic_run = tokio::spawn(async move {
        request_with_headers(
            &run_app,
            Method::POST,
            "/api/v1/agents/runs",
            Some(json!({
                "prompt": "hold the shared conversation",
                "workspaceCode": "TRON_CORE",
                "conversationCode": run_session_code,
                "model": {
                    "modelCode": "GENERIC_DELAYED_MODEL",
                    "protocol": "CHAT_COMPLETIONS",
                    "baseUrl": run_base_url,
                    "apiKey": "sk-test",
                    "modelName": "generic-delayed"
                }
            })),
            None,
            Some(&run_authorization),
        )
        .await
    });

    let mut generic_is_active = false;
    for _ in 0..50 {
        let active = request_with_headers(
            &app,
            Method::GET,
            &format!("/api/v1/chats/sessions/get-active-run?sessionCode={session_code}"),
            None,
            None,
            Some(&authorization),
        )
        .await;
        let active_body = response_json(active).await;
        if active_body["data"]["activeRun"]["runCode"]
            .as_str()
            .is_some()
        {
            generic_is_active = true;
            break;
        }
        sleep(Duration::from_millis(20)).await;
    }
    if !generic_is_active {
        let generic_response = generic_run
            .await
            .expect("failed generic run request should join");
        let status = generic_response.status();
        let body = response_json(generic_response).await;
        panic!("generic run should own the shared admission; response {status}: {body}");
    }

    let chat_send = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        Some(json!({
            "content": "must not become an orphan user message"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(chat_send.status(), StatusCode::CONFLICT);
    let chat_send_body = response_json(chat_send).await;
    assert_eq!(chat_send_body["code"], json!("CHAT_RUN_ALREADY_ACTIVE"));

    let messages = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(messages.status(), StatusCode::OK);
    let messages_body = response_json(messages).await;
    assert_eq!(messages_body["data"], json!([]));

    let generic_response = generic_run.await.expect("generic run request should join");
    assert_eq!(generic_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn chat_steer_endpoint_persists_server_message_and_continues_the_active_stream_run() {
    let app = app();
    let (server, model_state) = delayed_steer_model_mock_server().await;
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    let model_code = create_platform_text_model(&app, &authorization, &server.base_url).await;
    let created = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Steer HTTP success"
        })),
        None,
        Some(&authorization),
    )
    .await;
    let created_body = response_json(created).await;
    let session_code = created_body["data"]["sessionCode"]
        .as_str()
        .expect("session code should be returned")
        .to_string();
    let stream = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages/stream"),
        Some(json!({
            "content": "initial request",
            "workspaceCode": "TRON_CORE",
            "modelCode": model_code
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(stream.status(), StatusCode::OK);

    let mut active_run_code = None;
    for _ in 0..50 {
        let replay = request_with_headers(
            &app,
            Method::GET,
            &format!("/api/v1/agents/agui/streams/{session_code}/protocol-events"),
            None,
            None,
            Some(&authorization),
        )
        .await;
        assert_eq!(replay.status(), StatusCode::OK);
        let replay_body = response_json(replay).await;
        active_run_code = replay_body["data"].as_array().and_then(|events| {
            events.iter().find_map(|event| {
                (event["sseEvent"] == json!("RUN_STARTED"))
                    .then(|| event["data"]["runId"].as_str().map(str::to_string))
                    .flatten()
            })
        });
        if active_run_code.is_some() {
            break;
        }
        sleep(Duration::from_millis(20)).await;
    }
    let active_run_code = active_run_code.expect("active run code should be observable");
    let steer = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions/steer-message",
        Some(json!({
            "sessionCode": session_code,
            "expectedRunCode": active_run_code,
            "content": "change direction in this run",
            "attachments": []
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(steer.status(), StatusCode::OK);
    let steer_body = response_json(steer).await;
    assert_eq!(steer_body["data"]["sessionCode"], json!(session_code));
    assert_eq!(steer_body["data"]["runCode"], json!(active_run_code));
    assert!(steer_body["data"]["message"]["id"].as_str().is_some());
    assert_eq!(
        steer_body["data"]["message"]["content"],
        json!("change direction in this run")
    );
    assert_eq!(
        steer_body["data"]["message"]["runCode"],
        json!(active_run_code)
    );
    let steer_message_code = steer_body["data"]["message"]["messageCode"]
        .as_str()
        .expect("server messageCode should be returned")
        .to_string();

    let stream_body = to_bytes(stream.into_body(), usize::MAX)
        .await
        .expect("steered stream body should be readable");
    let stream_body =
        String::from_utf8(stream_body.to_vec()).expect("steered stream body should be utf8");
    assert!(stream_body.contains("\"supportsSteer\":true"));
    assert!(stream_body.contains("steered final output"));
    assert!(stream_body.contains("event: RUN_FINISHED"));
    assert_eq!(model_state.calls.load(Ordering::SeqCst), 2);
    let model_requests = model_state
        .requests
        .lock()
        .expect("steer model requests mutex should lock");
    assert!(
        model_requests[1]
            .to_string()
            .contains("change direction in this run")
    );

    let messages = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    let messages_body = response_json(messages).await;
    assert!(messages_body["data"].as_array().is_some_and(|records| {
        records.iter().any(|message| {
            message["messageCode"] == json!(steer_message_code)
                && message["runCode"] == json!(active_run_code)
                && message["content"] == json!("change direction in this run")
        })
    }));
    assert!(messages_body["data"].as_array().is_some_and(|records| {
        records.iter().any(|message| {
            message["role"] == json!("ASSISTANT")
                && message["runCode"] == json!(active_run_code)
                && message["content"] == json!("first boundary outputsteered final output")
        })
    }));
}

#[tokio::test]
async fn chat_stop_run_cancels_runtime_cleans_lease_and_prevents_orphan_messages() {
    let app = app();
    let (server, model_state) = delayed_steer_model_mock_server().await;
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    let model_code = create_platform_text_model(&app, &authorization, &server.base_url).await;
    let created = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Stop active run"
        })),
        None,
        Some(&authorization),
    )
    .await;
    let created_body = response_json(created).await;
    let session_code = created_body["data"]["sessionCode"]
        .as_str()
        .expect("session code should be returned")
        .to_string();
    let stream = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages/stream"),
        Some(json!({
            "content": "long running request",
            "workspaceCode": "TRON_CORE",
            "modelCode": model_code
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(stream.status(), StatusCode::OK);

    let mut active_run_code = None;
    for _ in 0..50 {
        let active = request_with_headers(
            &app,
            Method::GET,
            &format!("/api/v1/chats/sessions/get-active-run?sessionCode={session_code}"),
            None,
            None,
            Some(&authorization),
        )
        .await;
        assert_eq!(active.status(), StatusCode::OK);
        let active_body = response_json(active).await;
        active_run_code = active_body["data"]["activeRun"]["runCode"]
            .as_str()
            .map(str::to_string);
        if active_run_code.is_some() && model_state.calls.load(Ordering::SeqCst) > 0 {
            assert_eq!(
                active_body["data"]["activeRun"]["runtimeAdapterKey"],
                json!("REACT")
            );
            assert_eq!(
                active_body["data"]["activeRun"]["supportsSteer"],
                json!(true)
            );
            assert_eq!(active_body["data"]["activeRun"]["state"], json!("RUNNING"));
            break;
        }
        sleep(Duration::from_millis(20)).await;
    }
    let active_run_code = active_run_code.expect("active run should be queryable");

    let conflicting_send = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages/stream"),
        Some(json!({
            "content": "must not be persisted",
            "workspaceCode": "TRON_CORE",
            "modelCode": model_code
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(conflicting_send.status(), StatusCode::CONFLICT);
    let conflicting_body = response_json(conflicting_send).await;
    assert_eq!(conflicting_body["code"], json!("CHAT_RUN_ALREADY_ACTIVE"));

    let before_stop_messages = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    let before_stop_messages = response_json(before_stop_messages).await;
    assert_eq!(
        before_stop_messages["data"]
            .as_array()
            .expect("messages should be an array")
            .iter()
            .filter(|message| message["role"] == json!("USER"))
            .count(),
        1,
        "a rejected concurrent send must not persist an orphan user message"
    );

    let mismatched_stop = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions/stop-run",
        Some(json!({
            "sessionCode": session_code,
            "expectedRunCode": "RUN-STALE"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(mismatched_stop.status(), StatusCode::CONFLICT);
    let mismatched_body = response_json(mismatched_stop).await;
    assert_eq!(
        mismatched_body["code"],
        json!("CHAT_RUN_CONTROL_RUN_MISMATCH")
    );

    let stop = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions/stop-run",
        Some(json!({
            "sessionCode": session_code,
            "expectedRunCode": active_run_code
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(stop.status(), StatusCode::OK);
    let stop_body = response_json(stop).await;
    assert_eq!(stop_body["data"]["sessionCode"], json!(session_code));
    assert_eq!(stop_body["data"]["runCode"], json!(active_run_code));

    let stream_body = to_bytes(stream.into_body(), usize::MAX)
        .await
        .expect("stopped stream body should complete");
    let stream_body =
        String::from_utf8(stream_body.to_vec()).expect("stopped stream should be utf8");
    assert!(stream_body.contains("event: RUN_ERROR"));
    assert!(stream_body.contains("AGENT_RUN_STOPPED"));

    let mut became_idle = false;
    for _ in 0..50 {
        let active = request_with_headers(
            &app,
            Method::GET,
            &format!("/api/v1/chats/sessions/get-active-run?sessionCode={session_code}"),
            None,
            None,
            Some(&authorization),
        )
        .await;
        let active_body = response_json(active).await;
        if active_body["data"]["activeRun"].is_null() {
            became_idle = true;
            break;
        }
        sleep(Duration::from_millis(20)).await;
    }
    assert!(became_idle, "stopped run should leave the active registry");

    let next_stream = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages/stream"),
        Some(json!({
            "content": "request after stop",
            "workspaceCode": "TRON_CORE",
            "modelCode": model_code
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(next_stream.status(), StatusCode::OK);
    let next_stream_body = to_bytes(next_stream.into_body(), usize::MAX)
        .await
        .expect("next stream should complete");
    let next_stream_body =
        String::from_utf8(next_stream_body.to_vec()).expect("next stream should be utf8");
    assert!(next_stream_body.contains("event: RUN_FINISHED"));
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

    let latest_goal = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/chats/sessions/get-latest-goal?sessionCode={session_code}"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(latest_goal.status(), StatusCode::OK);
    let latest_goal_body = response_json(latest_goal).await;
    assert_eq!(latest_goal_body["code"], json!("SUCCESS"));
    assert_eq!(latest_goal_body["data"], Value::Null);

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
async fn chat_goal_can_be_updated_while_unfinished() {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let created = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Goal update"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = response_json(created).await;
    let session_code = created_body["data"]["sessionCode"]
        .as_str()
        .expect("session code should be returned")
        .to_string();

    let goal_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions/set-goal",
        Some(json!({
            "sessionCode": session_code,
            "objective": "Initial goal",
            "maxRuns": 5
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(goal_response.status(), StatusCode::OK);
    let goal_payload = response_json(goal_response).await;
    assert!(goal_payload["data"].get("criteria").is_none());
    let goal_code = goal_payload["data"]["goal"]["goalCode"]
        .as_str()
        .expect("goal code should be returned")
        .to_string();

    let update_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions/update-goal",
        Some(json!({
            "sessionCode": session_code,
            "goalCode": goal_code,
            "objective": "Updated goal",
            "maxRuns": 9
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(update_response.status(), StatusCode::OK);
    let update_payload = response_json(update_response).await;
    assert_eq!(
        update_payload["data"]["goal"]["objectiveMarkdown"],
        json!("Updated goal")
    );
    assert_eq!(update_payload["data"]["goal"]["maxRuns"], json!(9));
    assert!(update_payload["data"].get("criteria").is_none());

    let latest_response = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/chats/sessions/get-latest-goal?sessionCode={session_code}"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(latest_response.status(), StatusCode::OK);
    let latest_payload = response_json(latest_response).await;
    assert_eq!(
        latest_payload["data"]["goal"]["objectiveMarkdown"],
        json!("Updated goal")
    );
    assert_eq!(latest_payload["data"]["goal"]["maxRuns"], json!(9));
    assert!(latest_payload["data"].get("criteria").is_none());

    let clear_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions/clear-goal",
        Some(json!({
            "sessionCode": session_code,
            "goalCode": goal_code
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(clear_response.status(), StatusCode::OK);

    let terminal_update_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions/update-goal",
        Some(json!({
            "sessionCode": session_code,
            "goalCode": goal_code,
            "objective": "Should not update after clear",
            "maxRuns": 9
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(terminal_update_response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn chat_goal_judge_failure_retries_without_business_continuation() {
    let database_url = temp_database_url("chat-goal-judge-failure-no-continuation");
    let app = app_with_config(AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    });
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
            "title": "Goal judge failure"
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

    let goal_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions/set-goal",
        Some(json!({
            "sessionCode": session_code,
            "objective": "Finish the one business run, then judge it.",
            "maxRuns": 5
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(goal_response.status(), StatusCode::OK);
    let goal_payload = response_json(goal_response).await;
    let goal_code = goal_payload["data"]["goal"]["goalCode"]
        .as_str()
        .expect("goal code should be returned")
        .to_string();

    let send_response = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        Some(json!({
            "content": "run once and let judge fail",
            "workspaceCode": "TRON_CORE",
            "modelCode": model_code
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(send_response.status(), StatusCode::OK);
    let sent = response_json(send_response).await;
    let messages = sent["data"]["messages"]
        .as_array()
        .expect("messages should be an array");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["role"], json!("USER"));
    assert_eq!(messages[1]["role"], json!("ASSISTANT"));
    assert_eq!(
        messages[1]["content"],
        json!("chat reply: run once and let judge fail")
    );

    let connection = Connection::open(&database_url).expect("goal judge db should reopen");
    let run_count = connection
        .query_row(
            "SELECT count(*) FROM agent_runs WHERE conversation_code = ?1",
            [session_code.as_str()],
            |row| row.get::<_, i64>(0),
        )
        .expect("agent run count should query");
    assert_eq!(run_count, 1);

    let persisted_message_count = connection
        .query_row(
            "SELECT count(*) FROM chat_messages WHERE session_code = ?1",
            [session_code.as_str()],
            |row| row.get::<_, i64>(0),
        )
        .expect("chat message count should query");
    assert_eq!(persisted_message_count, 2);

    let mut statement = connection
        .prepare(
            "SELECT event_kind
             FROM agent_session_goal_events
             WHERE goal_code = ?1
             ORDER BY event_seq ASC",
        )
        .expect("goal event query should prepare");
    let event_kinds = statement
        .query_map([goal_code.as_str()], |row| row.get::<_, String>(0))
        .expect("goal event query should run")
        .collect::<Result<Vec<_>, _>>()
        .expect("goal event rows should map");
    assert_eq!(
        event_kinds,
        vec![
            "GOAL_SET".to_string(),
            "GOAL_JUDGE_STARTED".to_string(),
            "GOAL_JUDGE_ATTEMPT_FAILED".to_string(),
            "GOAL_JUDGE_ATTEMPT_FAILED".to_string(),
            "JUDGE_FAILED".to_string(),
        ]
    );

    let (runs_used, consecutive_failures, last_verdict): (i64, i64, String) = connection
        .query_row(
            "SELECT runs_used, consecutive_judge_failures, last_verdict
             FROM agent_session_goals
             WHERE goal_code = ?1",
            [goal_code.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("goal state should query");
    assert_eq!(runs_used, 1);
    assert_eq!(consecutive_failures, 1);
    assert_eq!(last_verdict, "JUDGE_FAILED");
}

#[tokio::test]
async fn chat_goal_judge_retry_can_complete_without_business_continuation() {
    let database_url = temp_database_url("chat-goal-judge-retry-completed");
    let app = app_with_config(AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    });
    let server = scripted_agent_model_mock_server(vec![
        "chat reply: finish requested work".to_string(),
        "not valid judge json".to_string(),
        r#"{"verdict":"COMPLETED","reason":"The business run finished the requested work.","completionEvidence":"assistant produced the final response","missingEvidence":[],"requiredUserInput":null,"nextStepHint":null,"continuationPrompt":null}"#
            .to_string(),
    ])
    .await;
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    let model_code = create_platform_text_model(&app, &authorization, &server.base_url).await;

    let session_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Goal judge retry completed"
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

    let goal_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions/set-goal",
        Some(json!({
            "sessionCode": session_code,
            "objective": "Complete after one business run.",
            "maxRuns": 5
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(goal_response.status(), StatusCode::OK);
    let goal_payload = response_json(goal_response).await;
    let goal_code = goal_payload["data"]["goal"]["goalCode"]
        .as_str()
        .expect("goal code should be returned")
        .to_string();

    let send_response = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        Some(json!({
            "content": "finish requested work",
            "workspaceCode": "TRON_CORE",
            "modelCode": model_code
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(send_response.status(), StatusCode::OK);
    let sent = response_json(send_response).await;
    let messages = sent["data"]["messages"]
        .as_array()
        .expect("messages should be an array");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["role"], json!("USER"));
    assert_eq!(messages[1]["role"], json!("ASSISTANT"));
    assert_eq!(
        messages[1]["content"],
        json!("chat reply: finish requested work")
    );

    let connection = Connection::open(&database_url).expect("goal judge db should reopen");
    let run_count = connection
        .query_row(
            "SELECT count(*) FROM agent_runs WHERE conversation_code = ?1",
            [session_code.as_str()],
            |row| row.get::<_, i64>(0),
        )
        .expect("agent run count should query");
    assert_eq!(run_count, 1);

    let persisted_message_count = connection
        .query_row(
            "SELECT count(*) FROM chat_messages WHERE session_code = ?1",
            [session_code.as_str()],
            |row| row.get::<_, i64>(0),
        )
        .expect("chat message count should query");
    assert_eq!(persisted_message_count, 2);

    let mut statement = connection
        .prepare(
            "SELECT event_kind
             FROM agent_session_goal_events
             WHERE goal_code = ?1
             ORDER BY event_seq ASC",
        )
        .expect("goal event query should prepare");
    let event_kinds = statement
        .query_map([goal_code.as_str()], |row| row.get::<_, String>(0))
        .expect("goal event query should run")
        .collect::<Result<Vec<_>, _>>()
        .expect("goal event rows should map");
    assert_eq!(
        event_kinds,
        vec![
            "GOAL_SET".to_string(),
            "GOAL_JUDGE_STARTED".to_string(),
            "GOAL_JUDGE_ATTEMPT_FAILED".to_string(),
            "GOAL_COMPLETED".to_string(),
        ]
    );

    let (status, runs_used, consecutive_failures, last_verdict): (String, i64, i64, String) =
        connection
            .query_row(
                "SELECT status, runs_used, consecutive_judge_failures, last_verdict
             FROM agent_session_goals
             WHERE goal_code = ?1",
                [goal_code.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("goal state should query");
    assert_eq!(status, "COMPLETED");
    assert_eq!(runs_used, 1);
    assert_eq!(consecutive_failures, 0);
    assert_eq!(last_verdict, "COMPLETED");

    let active_goal_response = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/chats/sessions/get-goal?sessionCode={session_code}"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(active_goal_response.status(), StatusCode::OK);
    let active_goal_payload = response_json(active_goal_response).await;
    assert_eq!(active_goal_payload["data"], Value::Null);

    let latest_goal_response = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/chats/sessions/get-latest-goal?sessionCode={session_code}"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(latest_goal_response.status(), StatusCode::OK);
    let latest_goal_payload = response_json(latest_goal_response).await;
    assert_eq!(
        latest_goal_payload["data"]["goal"]["goalCode"],
        json!(goal_code)
    );
    assert_eq!(
        latest_goal_payload["data"]["goal"]["status"],
        json!("COMPLETED")
    );
    assert!(latest_goal_payload["data"].get("criteria").is_none());
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
    assert!(body.contains("\"supportsSteer\":true"));
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
    assert!(body.contains("\"supportsSteer\":true"));
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
async fn chat_send_uses_model_default_and_validates_reasoning_effort_override() {
    let database_url = temp_database_url("chat-reasoning-effort-observability");
    let app = app_with_config(AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    });
    let (server, model_state) = delayed_steer_model_mock_server().await;
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    let model_code = create_platform_text_model(&app, &authorization, &server.base_url).await;

    let session_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Reasoning effort chat"
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

    let mut stream_bodies = Vec::new();
    for (content, reasoning_effort) in [
        ("use the model default", None),
        ("use a high override", Some("High")),
    ] {
        let mut body = json!({
            "content": content,
            "workspaceCode": "TRON_CORE",
            "modelCode": model_code
        });
        if let Some(reasoning_effort) = reasoning_effort {
            body["reasoningEffort"] = json!(reasoning_effort);
        }
        let response = request_with_headers(
            &app,
            Method::POST,
            &format!("/api/v1/chats/sessions/{session_code}/messages/stream"),
            Some(body),
            None,
            Some(&authorization),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let stream_body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("reasoning effort stream should be readable");
        stream_bodies.push(String::from_utf8(stream_body.to_vec()).expect("stream should be utf8"));
    }

    assert!(stream_bodies[0].contains("\"reasoningEffort\":\"Medium\""));
    assert!(stream_bodies[1].contains("\"reasoningEffort\":\"High\""));

    let requests = model_state
        .requests
        .lock()
        .expect("model requests mutex should lock")
        .clone();
    assert_eq!(requests[0]["reasoning_effort"], json!("medium"));
    assert_eq!(requests[1]["reasoning_effort"], json!("high"));

    let connection = Connection::open(&database_url).expect("reasoning effort db should reopen");
    let persisted_efforts = connection
        .prepare("SELECT reasoning_effort FROM agent_runs WHERE conversation_code = ?1 ORDER BY id")
        .expect("agent run reasoning effort query should prepare")
        .query_map([session_code.as_str()], |row| {
            row.get::<_, Option<String>>(0)
        })
        .expect("agent run reasoning efforts should query")
        .collect::<Result<Vec<_>, _>>()
        .expect("agent run reasoning efforts should collect");
    assert_eq!(
        persisted_efforts,
        vec![Some("Medium".into()), Some("High".into())]
    );

    let started_payloads = connection
        .prepare(
            "SELECT payload_json FROM agent_transcript_events WHERE session_code = ?1 AND event_type = 'agent.run.started' ORDER BY id",
        )
        .expect("run started payload query should prepare")
        .query_map([session_code.as_str()], |row| row.get::<_, String>(0))
        .expect("run started payloads should query")
        .collect::<Result<Vec<_>, _>>()
        .expect("run started payloads should collect");
    assert!(started_payloads[0].contains("\"reasoningEffort\":\"Medium\""));
    assert!(started_payloads[1].contains("\"reasoningEffort\":\"High\""));

    let invalid_response = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages"),
        Some(json!({
            "content": "reject an unsupported override",
            "workspaceCode": "TRON_CORE",
            "modelCode": model_code,
            "reasoningEffort": "Ultra"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(invalid_response.status(), StatusCode::BAD_REQUEST);
    let invalid_payload = response_json(invalid_response).await;
    assert_eq!(invalid_payload["code"], json!("AGENT_RUNTIME_FAILED"));
    assert_eq!(
        invalid_payload["message"],
        json!("reasoningEffort is not supported by the selected model"),
    );
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
    assert!(body.contains("\"supportsSteer\":false"));
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
    let mut stream_body = stream_response.into_body();
    let mut observed_stream = String::new();
    for _ in 0..20 {
        let frame = tokio::time::timeout(
            Duration::from_secs(2),
            poll_fn(|context| Pin::new(&mut stream_body).poll_frame(context)),
        )
        .await
        .expect("RUN_STARTED should be emitted before disconnect")
        .expect("stream should remain open before disconnect")
        .expect("stream frame should be readable");
        if let Ok(data) = frame.into_data() {
            observed_stream.push_str(&String::from_utf8_lossy(&data));
        }
        if observed_stream.contains("event: RUN_STARTED") {
            break;
        }
    }
    assert!(observed_stream.contains("event: RUN_STARTED"));
    drop(stream_body);

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
async fn unpolled_chat_stream_drop_detaches_observer_without_aborting_run() {
    let app = app();
    let (server, _) = delayed_steer_model_mock_server().await;
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    let model_code = create_platform_text_model(&app, &authorization, &server.base_url).await;
    let created = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Unpolled stream detach"
        })),
        None,
        Some(&authorization),
    )
    .await;
    let created_body = response_json(created).await;
    let session_code = created_body["data"]["sessionCode"]
        .as_str()
        .expect("session code should be returned")
        .to_string();
    let stream = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/chats/sessions/{session_code}/messages/stream"),
        Some(json!({
            "content": "finish after unpolled disconnect",
            "workspaceCode": "TRON_CORE",
            "modelCode": model_code
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(stream.status(), StatusCode::OK);
    drop(stream);

    let snapshot = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/agents/agui/streams/{session_code}/events"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(snapshot.status(), StatusCode::OK);
    let snapshot_body = response_json(snapshot).await;
    assert_eq!(snapshot_body["data"]["activeStreams"], json!(0));

    let mut replay_payload = json!(null);
    for _ in 0..60 {
        let replay = request_with_headers(
            &app,
            Method::GET,
            &format!("/api/v1/agents/agui/streams/{session_code}/protocol-events"),
            None,
            None,
            Some(&authorization),
        )
        .await;
        replay_payload = response_json(replay).await;
        let serialized = replay_payload.to_string();
        if serialized.contains("RUN_FINISHED") && serialized.contains("first boundary output") {
            break;
        }
        sleep(Duration::from_millis(25)).await;
    }
    let replay_json = replay_payload.to_string();
    assert!(replay_json.contains("RUN_STARTED"));
    assert!(replay_json.contains("first boundary output"));
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
            "supportedReasoningEfforts": ["Low", "Medium", "High"],
            "defaultReasoningEffort": "Medium",
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
