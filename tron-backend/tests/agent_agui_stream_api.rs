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
    adapters::outbound::persistence::SqliteAgentEventReplayStore,
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

#[tokio::test]
async fn short_term_memory_context_events_replay_as_agui_custom_events() {
    let database_path = temp_database_path("stm-agui-replay");
    let config = EventBusConfig::default();
    let sink = Arc::new(AguiEventSink::new(&config));
    let replay_store = Arc::new(
        SqliteAgentEventReplayStore::connect(&database_path.to_string_lossy())
            .expect("AG-UI replay store should connect"),
    );
    let event_bus =
        EventBusRuntimeBuilder::new(config, Arc::new(FixedSnowflakeIdProvider::new(480)))
            .register_sink(sink.clone())
            .register_sink(replay_store.clone())
            .build();
    let service = AgentAguiStreamService::with_replay_store(sink, replay_store);

    let outcome = event_bus.publish(
        BusEventDraft::agui("agent.context.assembled", "short_term_memory")
            .outcome(BusOutcome::Completed)
            .subject("agent_run", "RUN-STM-AGUI")
            .request(BusRequestRef::with_stream_key("SESSION-STM-AGUI"))
            .payload_field(
                "context_event_json",
                r#"{"estimatedTokens":12000,"budgetBreakdown":{"estimatedInputTokens":12000,"toolSchemaTokens":3000}}"#,
            ),
    );
    assert!(matches!(outcome, PublishOutcome::Accepted { .. }));
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let events = service
        .protocol_events("SESSION-STM-AGUI")
        .expect("context event should replay");
    let custom = events
        .iter()
        .find(|event| event.sse_event == "CUSTOM")
        .expect("context event should use AG-UI CUSTOM");
    assert_eq!(
        custom.data["name"],
        json!("tron.short_term_memory.context.assembled")
    );
    assert_eq!(custom.data["runId"], json!("RUN-STM-AGUI"));
    assert_eq!(
        custom.data["value"]["budgetBreakdown"]["toolSchemaTokens"],
        json!(3000)
    );
}

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
    assert_native_lifecycle_audit(records, protocol_records);

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
async fn generic_bus_records_do_not_generate_a_second_protocol_state_machine() {
    let config = EventBusConfig::default();
    let sink = Arc::new(AguiEventSink::new(&config));
    let bus = EventBusRuntimeBuilder::new(config, Arc::new(FixedSnowflakeIdProvider::new(450)))
        .register_sink(sink.clone())
        .build();
    let service = AgentAguiStreamService::new(sink);
    service.activate("SESSION").unwrap();
    for name in [
        "agent.run.started",
        "agent.model.requested",
        "agent.reasoning.delta",
        "agent.output.delta",
        "agent.tool.started",
        "agent.tool.failed",
        "agent.run.completed",
    ] {
        assert!(matches!(
            bus.publish(
                BusEventDraft::agui(name, "test")
                    .subject("agent_run", "RUN")
                    .request(BusRequestRef::with_stream_key("SESSION"))
            ),
            PublishOutcome::Accepted { .. }
        ));
    }
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(service.protocol_events("SESSION").unwrap().is_empty());
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
    let protocol_response = request_with_headers(
        &restarted,
        Method::GET,
        &format!("/api/v1/agents/agui/streams/{stream_key}/protocol-events"),
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(protocol_response.status(), StatusCode::OK);
    let protocol = response_json(protocol_response).await;
    assert_native_lifecycle_audit(records, protocol["data"].as_array().unwrap());
}

fn assert_native_lifecycle_audit(records: &[Value], protocol: &[Value]) {
    for (name, frame) in [
        ("agent.run.started", "RUN_STARTED"),
        ("agent.run.completed", "RUN_FINISHED"),
    ] {
        let audits: Vec<_> = records
            .iter()
            .filter(|event| event["eventName"] == name)
            .collect();
        let frames: Vec<_> = protocol
            .iter()
            .filter(|event| event["sseEvent"] == frame)
            .collect();
        assert_eq!(
            audits.len(),
            1,
            "one compatibility audit per actual native lifecycle frame"
        );
        assert_eq!(
            frames.len(),
            1,
            "one native lifecycle frame per SDK execution"
        );
        let audit = audits[0];
        let source = &frames[0]["data"];
        assert_eq!(audit["id"], source["metadata"]["tron"]["eventId"]);
        assert_eq!(audit["metadata"]["producer"], "framework.agui");
        assert_eq!(
            audit["metadata"]["nativeRunId"],
            source["metadata"]["framework"]["runId"]
        );
        assert_eq!(
            audit["metadata"]["executionId"],
            source["metadata"]["framework"]["executionId"]
        );
        assert_eq!(
            audit["metadata"]["sourceSequence"],
            source["metadata"]["framework"]["sourceSequence"]
                .as_u64()
                .unwrap()
                .to_string()
        );
    }
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
