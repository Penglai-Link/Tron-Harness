mod common;

use axum::{
    Json, Router,
    http::{Method, StatusCode},
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

async fn agent_chat_completion(Json(payload): Json<Value>) -> Json<Value> {
    let prompt = payload["messages"][0]["content"]
        .as_str()
        .unwrap_or_default();
    Json(json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": format!("agui completion for {prompt} with sk-agui-secret")
            }
        }]
    }))
}

#[tokio::test]
async fn agui_stream_adapter_converts_replayed_agent_events_safely() {
    let app = app();
    let server = agent_model_mock_server().await;
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    let stream_key = "CONV-AGUI";

    let activate = request_with_headers(
        &app,
        Method::POST,
        &format!("/api/v1/agents/agui/streams/{stream_key}"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(activate.status(), StatusCode::OK);
    let activated = response_json(activate).await;
    assert_eq!(activated["data"]["streamKey"], json!(stream_key));
    assert_eq!(activated["data"]["activeStreams"], json!(1));

    let create_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({
            "prompt": "stream this raw prompt",
            "workspaceCode": "TRON_CORE",
            "conversationCode": stream_key,
            "model": {
                "modelCode": "AGUI_MODEL",
                "protocol": "CHAT_COMPLETIONS",
                "baseUrl": server.base_url,
                "apiKey": "sk-agui-api-key",
                "modelName": "gpt-agui"
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

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let events_response = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/agents/agui/streams/{stream_key}/events"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(events_response.status(), StatusCode::OK);
    let events = response_json(events_response).await;
    assert_eq!(events["code"], json!("SUCCESS"));
    assert_eq!(events["data"]["streamKey"], json!(stream_key));
    let records = events["data"]["events"]
        .as_array()
        .expect("events should be an array");
    assert!(
        records
            .iter()
            .any(|event| event["eventName"] == "agent.run.started")
    );
    assert!(
        records
            .iter()
            .any(|event| event["eventName"] == "agent.run.completed")
    );
    assert!(
        records
            .iter()
            .all(|event| event["runCode"].as_str() == Some(run_code.as_str()))
    );
    assert!(
        records
            .iter()
            .all(|event| event["streamKey"].as_str() == Some(stream_key))
    );
    assert!(records.iter().all(|event| {
        event["type"]
            .as_str()
            .is_some_and(|value| value.starts_with("agent."))
    }));

    let event_json = events.to_string();
    assert!(!event_json.contains("stream this raw prompt"));
    assert!(!event_json.contains("sk-agui-api-key"));
    assert!(!event_json.contains("sk-agui-secret"));
    assert!(!event_json.contains("request_body"));

    let deactivate = request_with_headers(
        &app,
        Method::DELETE,
        &format!("/api/v1/agents/agui/streams/{stream_key}"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(deactivate.status(), StatusCode::OK);
    let deactivated = response_json(deactivate).await;
    assert_eq!(deactivated["data"]["streamKey"], json!(stream_key));
}
