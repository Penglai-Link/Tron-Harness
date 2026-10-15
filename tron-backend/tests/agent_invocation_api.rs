mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
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

async fn agent_chat_completion(
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> Response {
    let prompt = payload["messages"]
        .as_array()
        .and_then(|messages| {
            messages.iter().rev().find_map(|message| {
                (message["role"].as_str() == Some("user"))
                    .then(|| message["content"].as_str())
                    .flatten()
            })
        })
        .unwrap_or_default();
    let text = format!("mock: {prompt}");
    if payload["stream"].as_bool().unwrap_or(false) {
        // Real OpenAI-compatible servers answer stream requests with SSE.
        let chunks = [
            serde_json::json!({"choices":[{"index":0,"delta":{"role":"assistant","content":text}}]}),
            serde_json::json!({"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}),
        ];
        let mut body = String::new();
        for chunk in chunks {
            body.push_str(&format!("data: {chunk}\n\n"));
        }
        body.push_str("data: [DONE]\n\n");
        return (
            axum::http::StatusCode::OK,
            [(
                axum::http::header::CONTENT_TYPE,
                axum::http::header::HeaderValue::from_static("text/event-stream"),
            )],
            body,
        )
            .into_response();
    }
    axum::Json(serde_json::json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": text
            }
        }]
    }))
    .into_response()
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
            .is_some_and(|value| !value.trim().is_empty())
    );
    assert!(!body.to_string().contains("sk-test-api"));
}

#[tokio::test]
async fn agent_invocation_uses_active_profile_and_rejects_invalid_requests() {
    let app = app();

    let profile_request = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({"prompt": "x", "workspaceCode": "TRON_CORE"})),
        None,
        None,
    )
    .await;
    assert_eq!(profile_request.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(profile_request).await["code"],
        "AGENT_MODEL_REQUIRED"
    );

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
