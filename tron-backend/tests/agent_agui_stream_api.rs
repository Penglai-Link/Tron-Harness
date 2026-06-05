mod common;

use std::{
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    http::{Method, StatusCode},
    routing::post,
};
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};
use tron_backend::{
    application::agent::AgentAguiStreamService,
    config::AppConfig,
    shared::{
        event_bus::{
            AguiEventSink, BusEventDraft, BusOutcome, BusRequestRef, EventBusConfig,
            EventBusRuntimeBuilder, PublishOutcome,
        },
        id::FixedSnowflakeIdProvider,
    },
};

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

    let protocol_response = request_with_headers(
        &app,
        Method::GET,
        &format!("/api/v1/agents/agui/streams/{stream_key}/protocol-events"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(protocol_response.status(), StatusCode::OK);
    let protocol_events = response_json(protocol_response).await;
    assert_eq!(protocol_events["code"], json!("SUCCESS"));
    let protocol_records = protocol_events["data"]
        .as_array()
        .expect("protocol events should be an array");
    assert!(
        protocol_records
            .iter()
            .any(|event| event["sseEvent"] == "RUN_STARTED")
    );
    assert!(
        protocol_records
            .iter()
            .any(|event| event["sseEvent"] == "TEXT_MESSAGE_END")
    );

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

#[tokio::test]
async fn agui_protocol_separates_reasoning_summary_and_raw_text() {
    let config = EventBusConfig::default();
    let sink = Arc::new(AguiEventSink::new(&config));
    let event_bus =
        EventBusRuntimeBuilder::new(config, Arc::new(FixedSnowflakeIdProvider::new(450)))
            .register_sink(sink.clone())
            .build();
    let service = AgentAguiStreamService::new(sink);
    let stream_key = "CONV-REASONING-AGUI";
    let run_code = "RUN-REASONING-AGUI";
    service
        .activate(stream_key)
        .expect("stream should activate");

    let publish = |event_name: &'static str, delta: &'static str| {
        event_bus.publish(
            BusEventDraft::agui(event_name, "test")
                .outcome(BusOutcome::Progress)
                .subject("agent_run", run_code)
                .request(BusRequestRef::with_stream_key(stream_key))
                .payload_field(
                    "delta_json",
                    serde_json::to_string(delta).expect("delta should serialize"),
                ),
        )
    };

    assert!(matches!(
        publish("agent.reasoning.delta", "summary reasoning"),
        PublishOutcome::Accepted { .. }
    ));
    assert!(matches!(
        publish("agent.reasoning.raw_delta", "raw reasoning"),
        PublishOutcome::Accepted { .. }
    ));

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let events = service
        .protocol_events(stream_key)
        .expect("protocol events should be available");
    let event = |event_type: &str| {
        events
            .iter()
            .find(|event| event.sse_event == event_type)
            .expect("expected protocol event")
    };
    assert_eq!(
        event("REASONING_MESSAGE_CONTENT").data["delta"],
        json!("summary reasoning")
    );
    assert_eq!(
        event("REASONING_TEXT_CONTENT").data["delta"],
        json!("raw reasoning")
    );
}

#[tokio::test]
async fn agui_protocol_closes_reasoning_before_text_output() {
    let config = EventBusConfig::default();
    let sink = Arc::new(AguiEventSink::new(&config));
    let event_bus =
        EventBusRuntimeBuilder::new(config, Arc::new(FixedSnowflakeIdProvider::new(475)))
            .register_sink(sink.clone())
            .build();
    let service = AgentAguiStreamService::new(sink);
    let stream_key = "CONV-REASONING-TEXT-ORDER";
    let run_code = "RUN-REASONING-TEXT-ORDER";
    service
        .activate(stream_key)
        .expect("stream should activate");

    let publish = |event_name: &'static str, outcome: BusOutcome, delta: Option<&'static str>| {
        let mut draft = BusEventDraft::agui(event_name, "test")
            .outcome(outcome)
            .subject("agent_run", run_code)
            .request(BusRequestRef::with_stream_key(stream_key));
        if let Some(delta) = delta {
            draft = draft.payload_field(
                "delta_json",
                serde_json::to_string(delta).expect("delta should serialize"),
            );
        }
        event_bus.publish(draft)
    };

    assert!(matches!(
        publish("agent.model.requested", BusOutcome::Started, None),
        PublishOutcome::Accepted { .. }
    ));
    assert!(matches!(
        publish(
            "agent.reasoning.delta",
            BusOutcome::Progress,
            Some("checking workspace")
        ),
        PublishOutcome::Accepted { .. }
    ));
    assert!(matches!(
        publish("agent.output.started", BusOutcome::Started, None),
        PublishOutcome::Accepted { .. }
    ));
    assert!(matches!(
        publish(
            "agent.output.delta",
            BusOutcome::Progress,
            Some("visible answer")
        ),
        PublishOutcome::Accepted { .. }
    ));
    assert!(matches!(
        publish("agent.model.responded", BusOutcome::Completed, None),
        PublishOutcome::Accepted { .. }
    ));

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let events = service
        .protocol_events(stream_key)
        .expect("protocol events should be available");
    let event_types = events
        .iter()
        .map(|event| event.sse_event.as_str())
        .collect::<Vec<_>>();
    let position = |event_type: &str| {
        event_types
            .iter()
            .position(|current| *current == event_type)
            .expect("expected protocol event")
    };

    assert!(position("REASONING_MESSAGE_CONTENT") < position("REASONING_MESSAGE_END"));
    assert!(position("REASONING_MESSAGE_END") < position("REASONING_END"));
    assert!(position("REASONING_END") < position("TEXT_MESSAGE_START"));
    assert!(position("TEXT_MESSAGE_START") < position("TEXT_MESSAGE_CONTENT"));
    assert_eq!(
        event_types
            .iter()
            .filter(|event_type| **event_type == "REASONING_MESSAGE_END")
            .count(),
        1
    );
    assert_eq!(
        event_types
            .iter()
            .filter(|event_type| **event_type == "REASONING_END")
            .count(),
        1
    );
}

#[tokio::test]
async fn agui_protocol_tool_events_keep_correlation_fields() {
    let config = EventBusConfig::default();
    let sink = Arc::new(AguiEventSink::new(&config));
    let event_bus =
        EventBusRuntimeBuilder::new(config, Arc::new(FixedSnowflakeIdProvider::new(400)))
            .register_sink(sink.clone())
            .build();
    let service = AgentAguiStreamService::new(sink);
    let stream_key = "CONV-TOOL-AGUI";
    let run_code = "RUN-TOOL-AGUI";
    service
        .activate(stream_key)
        .expect("stream should activate");

    let publish = |event_name: &'static str, outcome: BusOutcome, result: &'static str| {
        event_bus.publish(
            BusEventDraft::agui(event_name, "test")
                .outcome(outcome)
                .subject("agent_run", run_code)
                .request(BusRequestRef::with_stream_key(stream_key))
                .summary("shell.exec")
                .payload_field("tool_call_id", "RUN-TOOL-AGUI-tool-1")
                .payload_field("tool_name", "shell.exec")
                .payload_field("tool_args_json", "{\"command\":\"cargo test\"}")
                .payload_field("tool_result_json", result),
        )
    };

    assert!(matches!(
        publish("agent.tool.selected", BusOutcome::Progress, "selected"),
        PublishOutcome::Accepted { .. }
    ));
    assert!(matches!(
        publish("agent.tool.started", BusOutcome::Started, "started"),
        PublishOutcome::Accepted { .. }
    ));
    assert!(matches!(
        publish("agent.tool.activity", BusOutcome::Progress, "live line"),
        PublishOutcome::Accepted { .. }
    ));
    assert!(matches!(
        publish(
            "agent.observation.created",
            BusOutcome::Progress,
            "observation"
        ),
        PublishOutcome::Accepted { .. }
    ));
    assert!(matches!(
        publish("agent.tool.completed", BusOutcome::Completed, "ok"),
        PublishOutcome::Accepted { .. }
    ));

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let events = service
        .protocol_events(stream_key)
        .expect("protocol events should be available");
    let event = |event_type: &str| {
        events
            .iter()
            .find(|event| event.sse_event == event_type)
            .expect("expected protocol event")
    };
    assert_eq!(
        event("TOOL_CALL_START").data["toolCallId"],
        json!("RUN-TOOL-AGUI-tool-1")
    );
    assert_eq!(
        event("TOOL_CALL_START").data["toolCallArgs"]["command"],
        json!("cargo test")
    );
    let activity_events = events
        .iter()
        .filter(|event| event.sse_event == "ACTIVITY_DELTA")
        .collect::<Vec<_>>();
    assert_eq!(activity_events.len(), 2);
    assert!(activity_events.iter().any(|event| {
        event.data["toolCallId"] == json!("RUN-TOOL-AGUI-tool-1")
            && event.data["delta"][0]["value"] == json!("live line")
    }));
    assert_eq!(event("TOOL_CALL_RESULT").data["content"], json!("ok"));
    assert_eq!(
        event("TOOL_CALL_END").data["toolCallId"],
        json!("RUN-TOOL-AGUI-tool-1")
    );
}

#[tokio::test]
async fn agui_protocol_tool_failure_events_emit_result_and_end() {
    let config = EventBusConfig::default();
    let sink = Arc::new(AguiEventSink::new(&config));
    let event_bus =
        EventBusRuntimeBuilder::new(config, Arc::new(FixedSnowflakeIdProvider::new(401)))
            .register_sink(sink.clone())
            .build();
    let service = AgentAguiStreamService::new(sink);
    let stream_key = "CONV-TOOL-AGUI-FAIL";
    let run_code = "RUN-TOOL-AGUI-FAIL";
    service
        .activate(stream_key)
        .expect("stream should activate");

    let publish = |event_name: &'static str| {
        event_bus.publish(
            BusEventDraft::agui(event_name, "test")
                .outcome(BusOutcome::Failure)
                .subject("agent_run", run_code)
                .request(BusRequestRef::with_stream_key(stream_key))
                .summary("shell.exec")
                .payload_field("tool_call_id", "RUN-TOOL-AGUI-FAIL-tool-1")
                .payload_field("tool_name", "shell.exec")
                .payload_field("tool_result_json", "command exited with status 1"),
        )
    };

    assert!(matches!(
        publish("agent.tool.failed"),
        PublishOutcome::Accepted { .. }
    ));

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let events = service
        .protocol_events(stream_key)
        .expect("protocol events should be available");
    let event = |event_type: &str| {
        events
            .iter()
            .find(|event| event.sse_event == event_type)
            .expect("expected protocol event")
    };
    assert_eq!(
        event("TOOL_CALL_RESULT").data["toolCallId"],
        json!("RUN-TOOL-AGUI-FAIL-tool-1")
    );
    assert_eq!(
        event("TOOL_CALL_RESULT").data["content"],
        json!("command exited with status 1")
    );
    assert_eq!(
        event("TOOL_CALL_END").data["toolCallId"],
        json!("RUN-TOOL-AGUI-FAIL-tool-1")
    );
}

#[tokio::test]
async fn agui_stream_events_can_replay_from_persistent_store_after_restart() {
    let database_path = temp_database_path("agui-replay");
    let mut config = AppConfig::default();
    config.database_url = database_path.to_string_lossy().into_owned();
    let app = common::app_with_config(config.clone());
    let server = agent_model_mock_server().await;
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    let stream_key = "CONV-AGUI-PERSIST";

    let create_response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(json!({
            "prompt": "persist this stream replay",
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
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;

    let restarted = common::app_with_config(config);
    let token = authenticated_bearer_token(&restarted).await;
    let authorization = format!("Bearer {token}");
    let events_response = request_with_headers(
        &restarted,
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
}

fn temp_database_path(test_name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be monotonic")
        .as_nanos();
    path.push(format!("tron-{test_name}-{nanos}.sqlite3"));
    path
}
