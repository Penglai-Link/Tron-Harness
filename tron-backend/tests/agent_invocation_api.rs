mod common;

use axum::{
    Json, Router,
    http::{Method, StatusCode},
    routing::post,
};
use serde_json::json;
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

async fn agent_chat_completion(Json(payload): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let prompt = payload["messages"][0]["content"]
        .as_str()
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

#[tokio::test]
async fn protected_agent_invocation_returns_uniform_run_output() {
    let app = app();
    let server = agent_model_mock_server().await;
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({
            "prompt": "Say done",
            "workspaceCode": "TRON_CORE",
            "conversationCode": "CONV-API",
            "model": {
                "modelCode": "TEST_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": server.base_url,
                "apiKey": "sk-test-api",
                "modelName": "gpt-test"
            }
        })),
        None,
        Some(&authorization),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["code"], json!("SUCCESS"));
    assert_eq!(body["data"]["workspaceCode"], json!("TRON_CORE"));
    assert_eq!(body["data"]["conversationCode"], json!("CONV-API"));
    assert_eq!(body["data"]["status"], json!("COMPLETED"));
    assert_eq!(body["data"]["finalOutput"], json!("mock: Say done"));
    assert!(
        body["data"]["runCode"]
            .as_str()
            .is_some_and(|value| value.starts_with("RUN-"))
    );
    assert!(!body.to_string().contains("sk-test-api"));
}

#[tokio::test]
async fn agent_invocation_rejects_unauthenticated_and_invalid_requests() {
    let app = app();

    let unauthenticated = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({"prompt": "x", "workspaceCode": "TRON_CORE"})),
        None,
        None,
    )
    .await;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    for (payload, code) in [
        (
            json!({"workspaceCode": "TRON_CORE"}),
            "AGENT_PROMPT_REQUIRED",
        ),
        (json!({"prompt": "x"}), "AGENT_WORKSPACE_REQUIRED"),
        (
            json!({"prompt": "x", "workspaceCode": "MISSING"}),
            "AGENT_WORKSPACE_INVALID",
        ),
        (
            json!({"prompt": "x", "workspaceCode": "TRON_CORE"}),
            "AGENT_MODEL_REQUIRED",
        ),
    ] {
        let response = request_with_headers(
            &app,
            Method::POST,
            "/api/v1/agents/runs",
            Some(payload),
            None,
            Some(&authorization),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response_json(response).await;
        assert_eq!(body["code"], json!(code));
    }
}
