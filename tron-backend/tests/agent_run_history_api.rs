mod common;

use axum::{
    Json, Router,
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::post,
};
use serde_json::{Value, json};
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
    axum::extract::Json(payload): axum::extract::Json<Value>,
) -> axum::response::Response {
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
    let text = format!("history completion for {prompt} with sk-history-secret");
    if payload["stream"].as_bool().unwrap_or(false) {
        // Real OpenAI-compatible servers answer stream requests with SSE.
        let chunks = [
            json!({"choices":[{"index":0,"delta":{"role":"assistant","content":text}}]}),
            json!({"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}),
        ];
        let mut body = String::new();
        for chunk in chunks {
            body.push_str(&format!("data: {chunk}\n\n"));
        }
        body.push_str("data: [DONE]\n\n");
        return (
            StatusCode::OK,
            [(
                axum::http::header::CONTENT_TYPE,
                axum::http::header::HeaderValue::from_static("text/event-stream"),
            )],
            body,
        )
            .into_response();
    }
    Json(json!({
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
async fn agent_invocation_persists_safe_history_and_query_apis() {
    let app = app();
    let server = agent_model_mock_server().await;
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    let raw_prompt = "Persist this raw prompt body";

    let create_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({
            "prompt": raw_prompt,
            "workspaceCode": "TRON_CORE",
            "conversationCode": "CONV-HISTORY",
            "model": {
                "modelCode": "HISTORY_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": server.base_url,
                "apiKey": "sk-history-api-key",
                "modelName": "gpt-history"
            }
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(create_response.status(), StatusCode::OK);
    let created = response_json(create_response).await;
    let run_code = created["data"]["runCode"]
        .as_str()
        .expect("run code should be returned")
        .to_string();

    let list_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/agents/runs?workspaceCode=TRON_CORE&conversationCode=CONV-HISTORY&current=1&size=10",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list = response_json(list_response).await;
    assert_eq!(list["code"], json!("SUCCESS"));
    assert_eq!(list["data"]["current"], json!(1));
    assert_eq!(list["data"]["size"], json!(10));
    assert_eq!(list["data"]["total"], json!(1));
    assert_eq!(list["data"]["records"][0]["runCode"], json!(run_code));
    assert_eq!(list["data"]["records"][0]["status"], json!("COMPLETED"));
    assert_eq!(
        list["data"]["records"][0]["workspaceCode"],
        json!("TRON_CORE")
    );
    assert_eq!(
        list["data"]["records"][0]["conversationCode"],
        json!("CONV-HISTORY")
    );
    assert_eq!(
        list["data"]["records"][0]["modelCode"],
        json!("HISTORY_MODEL")
    );
    assert_eq!(list["data"]["records"][0]["stepCount"], json!(1));
    assert_eq!(list["data"]["records"][0]["observationCount"], json!(0));

    let detail_response = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/agents/runs/{run_code}"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail = response_json(detail_response).await;
    assert_eq!(detail["data"]["runCode"], json!(run_code));
    assert!(detail["data"]["outputPreview"].as_str().is_some());

    let history_json = detail.to_string();
    assert!(!history_json.contains(raw_prompt));
    assert!(!history_json.contains("sk-history-api-key"));
    assert!(!history_json.contains("sk-history-secret"));
    assert!(!history_json.contains("request_body"));
}

#[tokio::test]
async fn missing_agent_run_history_returns_stable_error() {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/agents/runs/RUN-MISSING",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response_json(response).await;
    assert_eq!(body["code"], json!("AGENT_RUN_NOT_FOUND"));
}
