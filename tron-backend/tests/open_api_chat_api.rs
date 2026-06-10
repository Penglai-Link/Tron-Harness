mod common;

use std::convert::Infallible;

use axum::{
    Json, Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
    response::{
        IntoResponse, Response,
        sse::{Event, Sse},
    },
    routing::post,
};
use serde_json::{Value, json};
use tower::ServiceExt;
use tokio::{net::TcpListener, task::JoinHandle};

use common::{app, authenticated_bearer_token, request_with_headers, response_json};

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
        let reasoning = json!({
            "choices": [{
                "delta": {
                    "reasoning_content": format!("thinking about {prompt}. done thinking.")
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
async fn open_api_chat_session_groups_and_messages_use_public_contract() {
    let app = app();
    let created = open_api_request(
        &app,
        Method::POST,
        "/open-api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "taskCode": "TASK-OPEN-ALPHA",
            "title": "Open chat"
        })),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = response_json(created).await;
    let session_code = created_body["data"]["sessionCode"]
        .as_str()
        .expect("session code should be returned")
        .to_string();
    assert_eq!(created_body["data"]["title"], json!("Open chat"));
    assert_eq!(created_body["data"]["workspaceCode"], json!("TRON_CORE"));
    assert_eq!(created_body["data"]["status"], json!("ACTIVE"));

    let groups = open_api_request(
        &app,
        Method::GET,
        "/open-api/v1/chats/session-groups?workspaceCode=TRON_CORE",
        None,
    )
    .await;
    assert_eq!(groups.status(), StatusCode::OK);
    let groups_body = response_json(groups).await;
    assert!(
        groups_body["data"]
            .as_array()
            .expect("groups should be an array")
            .iter()
            .any(|group| {
                group["taskCode"] == json!("TASK-OPEN-ALPHA")
                    && group["sessions"]
                        .as_array()
                        .expect("sessions should be an array")
                        .iter()
                        .any(|session| session["sessionCode"] == json!(session_code))
            })
    );

    let messages = open_api_request(
        &app,
        Method::GET,
        &format!("/open-api/v1/chats/sessions/{session_code}/messages"),
        None,
    )
    .await;
    assert_eq!(messages.status(), StatusCode::OK);
    let messages_body = response_json(messages).await;
    assert_eq!(messages_body["data"], json!([]));
}

#[tokio::test]
async fn open_api_chat_routes_support_browser_cors_preflight() {
    let app = app();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/open-api/v1/chats/sessions")
                .header("Origin", "http://127.0.0.1:8082")
                .header("Access-Control-Request-Method", "POST")
                .header(
                    "Access-Control-Request-Headers",
                    "content-type,x-tron-app-id,x-tron-app-key,x-tron-user-id",
                )
                .body(Body::empty())
                .expect("request should be constructible"),
        )
        .await
        .expect("router should return a response");

    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers();
    assert_eq!(
        headers
            .get("access-control-allow-origin")
            .and_then(|value| value.to_str().ok()),
        Some("*"),
    );
    assert_eq!(
        headers
            .get("access-control-allow-methods")
            .and_then(|value| value.to_str().ok()),
        Some("GET,POST,PATCH,OPTIONS"),
    );
}

#[tokio::test]
async fn open_api_chat_stream_and_agui_replay_follow_public_contract() {
    let app = app();
    let server = agent_model_mock_server().await;
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    let model_code = create_platform_text_model(&app, &authorization, &server.base_url).await;

    let created = open_api_request(
        &app,
        Method::POST,
        "/open-api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Open stream"
        })),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = response_json(created).await;
    let session_code = created_body["data"]["sessionCode"]
        .as_str()
        .expect("session code should be returned")
        .to_string();

    let stream = open_api_request(
        &app,
        Method::POST,
        &format!("/open-api/v1/chats/sessions/{session_code}/messages/stream"),
        Some(json!({
            "content": "stream via open api",
            "workspaceCode": "TRON_CORE",
            "modelCode": model_code
        })),
    )
    .await;
    assert_eq!(stream.status(), StatusCode::OK);
    let body = to_bytes(stream.into_body(), usize::MAX)
        .await
        .expect("stream body should be readable");
    let body = String::from_utf8(body.to_vec()).expect("stream body should be utf8");
    assert!(body.contains("event: RUN_STARTED"));
    assert!(body.contains("event: RUN_FINISHED"));
    assert!(body.contains("\"type\":\"RUN_FINISHED\""));
    assert!(body.contains("\"reply: stream via open api\""));

    let messages = open_api_request(
        &app,
        Method::GET,
        &format!("/open-api/v1/chats/sessions/{session_code}/messages"),
        None,
    )
    .await;
    assert_eq!(messages.status(), StatusCode::OK);
    let messages_body = response_json(messages).await;
    assert_eq!(messages_body["data"][0]["role"], json!("USER"));
    assert_eq!(messages_body["data"][1]["role"], json!("ASSISTANT"));
    assert_eq!(messages_body["data"][1]["channel"], json!("MAIN_AGENT"));
    assert_eq!(
        messages_body["data"][1]["content"],
        json!("chat reply: stream via open api")
    );

    let replay = open_api_request(
        &app,
        Method::GET,
        &format!("/open-api/v1/agents/agui/streams/{session_code}/protocol-events"),
        None,
    )
    .await;
    assert_eq!(replay.status(), StatusCode::OK);
    let replay_body = response_json(replay).await.to_string();
    assert!(replay_body.contains("RUN_FINISHED"));
    assert!(replay_body.contains("stream via open api"));
}

async fn create_platform_text_model(app: &Router, authorization: &str, base_url: &str) -> String {
    let response = request_with_headers(
        app,
        Method::POST,
        "/api/v1/admin/platform/models",
        Some(json!({
            "displayName": "Open API Chat Default",
            "vendor": "OpenAI",
            "protocol": "OPENAI_COMPATIBLE",
            "modelType": "TEXT",
            "modelCapabilities": ["TEXT_GENERATION"],
            "baseUrl": base_url,
            "apiPath": "/chat/completions",
            "apiKeyRef": "sk-open-api-chat",
            "modelName": "gpt-chat",
            "enabled": true,
            "description": "Open API chat model",
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

async fn open_api_request(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("X-Tron-App-Id", "eliteforge-kb")
        .header("X-Tron-App-Key", "dev-open-api-key")
        .header("X-Tron-User-Id", "alex.johnson");
    let request_body = if let Some(json_body) = body {
        builder = builder.header("Content-Type", "application/json");
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
