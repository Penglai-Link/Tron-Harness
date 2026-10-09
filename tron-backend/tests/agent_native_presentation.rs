use serde_json::json;
use std::sync::Arc;
use tron_backend::{
    adapters::outbound::persistence::SqliteAgentEventReplayStore,
    application::agent::{
        AgentAguiReplayStore, AgentAguiStreamError, AgentAguiStreamService, protocol_event_identity,
    },
    domain::agent::{AgentAguiEvent, AgentAguiProtocolEvent},
    shared::event_bus::{
        AguiEventSink, BusEventDraft, BusRequestRef, EventBusConfig, EventBusRuntimeBuilder,
        PublishOutcome,
    },
    shared::id::FixedSnowflakeIdProvider,
};
fn event(sequence: u64) -> AgentAguiProtocolEvent {
    AgentAguiProtocolEvent {
        sse_event: "TEXT_MESSAGE_CONTENT".into(),
        data: json!({
            "type":"TEXT_MESSAGE_CONTENT","threadId":"SESSION","runId":"RUN","messageId":"MESSAGE","timestamp":1,"delta":"ha",
            "metadata":{"framework":{"runId":"NATIVE-RUN","executionId":"EXECUTION","sourceSequence":sequence},"tron":{"eventId":format!("native:EXECUTION:{sequence}:0")}}
        }),
    }
}
#[test]
fn sqlite_replay_preserves_repeated_chunks_and_deduplicates_only_same_identity() {
    let store = Arc::new(replay_store(&database_path("memory")));
    let service = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(&EventBusConfig::default())),
        store.clone(),
    );
    for item in [event(1), event(2), event(1)] {
        service.record_protocol_event("SESSION", item).unwrap();
    }
    let replay = service.protocol_events("SESSION").unwrap();
    assert_eq!(replay.len(), 2);
    assert_eq!(
        replay
            .iter()
            .map(|event| event.data["delta"].as_str().unwrap())
            .collect::<String>(),
        "haha"
    );
    assert_ne!(
        protocol_event_identity(&replay[0]),
        protocol_event_identity(&replay[1])
    );
}
struct BrokenReplay;
impl AgentAguiReplayStore for BrokenReplay {
    fn append_protocol_event(&self, _: &str, _: AgentAguiProtocolEvent) -> Result<(), String> {
        Err("write failure".into())
    }
    fn replay_agui_events(&self, _: &str) -> Result<Vec<AgentAguiEvent>, String> {
        Err("read failure".into())
    }
    fn replay_protocol_events(&self, _: &str) -> Result<Vec<AgentAguiProtocolEvent>, String> {
        Err("read failure".into())
    }
}
#[test]
fn replay_errors_are_not_successful_empty_history_or_successful_writes() {
    let service = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(&EventBusConfig::default())),
        Arc::new(BrokenReplay),
    );
    assert!(matches!(
        service.events("SESSION"),
        Err(AgentAguiStreamError::ReplayReadFailed(_))
    ));
    assert!(matches!(
        service.protocol_events("SESSION"),
        Err(AgentAguiStreamError::ReplayReadFailed(_))
    ));
    assert!(matches!(
        service.record_protocol_event("SESSION", event(1)),
        Err(AgentAguiStreamError::ReplayWriteFailed(_))
    ));
}
#[test]
fn rejected_oversize_delivery_exposes_incomplete_replay() {
    let store = Arc::new(replay_store(&database_path("memory")));
    let service = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(&EventBusConfig::default())),
        store.clone(),
    );
    let bus = EventBusRuntimeBuilder::new(
        EventBusConfig {
            max_name_bytes: 1,
            ..EventBusConfig::default()
        },
        Arc::new(FixedSnowflakeIdProvider::new(987)),
    )
    .register_sink(store)
    .build();
    let draft = || {
        BusEventDraft::agui("agent.context.assembled", "test")
            .request(BusRequestRef::with_stream_key("SESSION"))
    };
    assert!(matches!(
        bus.publish(draft()),
        PublishOutcome::RejectedOversize { .. }
    ));
    assert!(service.ensure_complete("SESSION").is_err());
    assert!(service.protocol_events("SESSION").unwrap().is_empty());
}

#[test]
fn canonical_subtotal_remains_available_with_explicit_incomplete_flag() {
    let store = Arc::new(replay_store(&database_path("memory")));
    let service = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(&EventBusConfig::default())),
        store,
    );
    service.mark_incomplete("SESSION", "dropped presentation");
    let snapshot = service.events("SESSION").unwrap();
    assert!(snapshot.replay_incomplete);
    assert!(service.ensure_complete("SESSION").is_err());
    assert!(service.protocol_events("SESSION").unwrap().is_empty());
}

#[test]
fn historical_delivery_failure_does_not_block_a_later_run_or_restart_replay() {
    let path = database_path("retry-after-incomplete");
    {
        let store = Arc::new(replay_store(&path));
        let service = AgentAguiStreamService::with_replay_store(
            Arc::new(AguiEventSink::new(&EventBusConfig::default())),
            store,
        );
        service.record_protocol_event("SESSION", event(1)).unwrap();
        service.mark_incomplete("SESSION", "previous output dropped");
        let mut next = event(2);
        next.data["runId"] = json!("RETRY-RUN");
        next.data["delta"] = json!("你好");
        service.record_protocol_event("SESSION", next).unwrap();
        let frames = service.protocol_events("SESSION").unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[1].data["delta"], "你好");
        assert!(service.events("SESSION").unwrap().replay_incomplete);
    }
    let service = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(&EventBusConfig::default())),
        Arc::new(replay_store(&path)),
    );
    assert_eq!(service.protocol_events("SESSION").unwrap()[1].data["runId"], "RETRY-RUN");
    let _ = std::fs::remove_file(path);
}

fn terminal(sequence: u64, kind: &str, outcome: serde_json::Value) -> AgentAguiProtocolEvent {
    let mut item = event(sequence);
    item.sse_event = kind.into();
    item.data["type"] = json!(kind);
    item.data["outcome"] = outcome;
    item
}
fn database_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "tron-native-replay-{label}-{}.sqlite",
        eliteforge_agent_core::RunId::new_v4()
    ))
}
#[test]
fn successful_write_persists_prior_delivery_failure_across_restart() {
    let path = database_path("health");
    {
        let store = replay_store(&path);
        store.record_delivery_failure("SESSION", "delivery lost");
        store.append_protocol_event("SESSION", event(1)).unwrap();
    }
    let reopened = replay_store(&path);
    assert!(reopened.replay_failure("SESSION").is_some());
    drop(reopened);
    let _ = std::fs::remove_file(path);
}
#[test]
fn restart_detects_unfinished_native_execution_but_accepts_awaiting_input_terminal() {
    for (label, finish, expected_incomplete) in
        [("unfinished", false, true), ("paused", true, false)]
    {
        let path = database_path(label);
        {
            let store = replay_store(&path);
            store
                .append_protocol_event(
                    "SESSION",
                    terminal(1, "RUN_STARTED", serde_json::Value::Null),
                )
                .unwrap();
            if finish {
                store.append_protocol_event("SESSION",terminal(2,"RUN_FINISHED",json!({"type":"interrupt","interrupts":[{"id":"INPUT","reason":"input_required"}]}))).unwrap();
            }
        }
        let reopened = replay_store(&path);
        assert_eq!(
            reopened.replay_failure("SESSION").is_some(),
            expected_incomplete,
            "{label}"
        );
        drop(reopened);
        let _ = std::fs::remove_file(path);
    }
}

#[tokio::test]
async fn last_delivery_failure_is_persisted_without_a_later_model_or_event_write() {
    let path = database_path("last-failure");
    let store = Arc::new(replay_store(&path));
    store
        .append_protocol_event(
            "SESSION",
            terminal(1, "RUN_STARTED", serde_json::Value::Null),
        )
        .unwrap();
    store
        .append_protocol_event(
            "SESSION",
            terminal(2, "RUN_FINISHED", json!({"type":"success"})),
        )
        .unwrap();
    store.record_delivery_failure("SESSION", "last canonical event was rejected");
    // Read only the durable acknowledgment, independent of the live registry.
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let host = tron_backend::application::agent::FrameworkHost::persistent(
                path.to_str().unwrap(),
                None,
            );
            let sessions = host.storage_framework().await.unwrap().sessions().unwrap();
            if sessions
                .read_context("tron/presentation-health", "tron.replay-failure/SESSION")
                .await
                .unwrap()
                .is_some()
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the final rejection must not wait for another model call to persist health");
    drop(store);
    let reopened = replay_store(&path);
    assert!(reopened.replay_failure("SESSION").is_some());
    drop(reopened);
    let _ = std::fs::remove_file(path);
}

#[test]
fn native_audit_preserves_source_identity_idempotently_across_restart() {
    let path = database_path("canonical-source");
    let start = terminal(1, "RUN_STARTED", serde_json::Value::Null);
    let finish = terminal(2, "RUN_FINISHED", json!({"type":"success"}));
    let before = {
        let store = replay_store(&path);
        for item in [start.clone(), finish.clone(), start, finish] {
            store.append_protocol_event("SESSION", item).unwrap();
        }
        let audit = store.replay_agui_events("SESSION").unwrap();
        let protocol = store.replay_protocol_events("SESSION").unwrap();
        assert_eq!(audit.len(), 2);
        assert_eq!(protocol.len(), 2);
        for (audit, source) in audit.iter().zip(protocol.iter()) {
            assert_eq!(audit.id, protocol_event_identity(source).unwrap());
            assert_eq!(audit.metadata["producer"], "framework.agui");
            assert_eq!(audit.metadata["nativeRunId"], "NATIVE-RUN");
            assert_eq!(audit.metadata["executionId"], "EXECUTION");
            assert_eq!(
                audit.metadata["sourceSequence"],
                source.data["metadata"]["framework"]["sourceSequence"]
                    .as_u64()
                    .unwrap()
                    .to_string()
            );
        }
        assert_eq!(audit[0].event_name, "agent.run.started");
        assert_eq!(audit[1].event_name, "agent.run.completed");
        serde_json::to_value(audit).unwrap()
    };
    let reopened = replay_store(&path);
    assert_eq!(
        serde_json::to_value(reopened.replay_agui_events("SESSION").unwrap()).unwrap(),
        before
    );
    drop(reopened);
    let _ = std::fs::remove_file(path);
}

#[test]
fn native_audit_keeps_pauses_errors_and_presentation_faults_distinct_and_redacts_content() {
    let store = replay_store(&database_path("memory"));
    store.append_protocol_event("SESSION", terminal(1,"RUN_FINISHED",json!({"type":"interrupt","interrupts":[{"id":"APPROVAL","reason":"approval_required","payload":{"secret":"private approval content"}}]}))).unwrap();
    let mut failure = terminal(2, "RUN_ERROR", serde_json::Value::Null);
    failure.data["code"] = json!("AGENT_LIMIT_REACHED");
    failure.data["message"] = json!("private provider response");
    store.append_protocol_event("SESSION", failure).unwrap();
    let mut health = terminal(3, "RUN_ERROR", serde_json::Value::Null);
    health.data["code"] = json!("AGUI_BACKPRESSURE");
    store.append_protocol_event("SESSION", health).unwrap();
    store
        .append_protocol_event(
            "SESSION",
            terminal(4, "RUN_FINISHED", serde_json::Value::Null),
        )
        .unwrap();
    let audit = store.replay_agui_events("SESSION").unwrap();
    assert_eq!(audit.len(), 2);
    assert_eq!(audit[0].event_name, "agent.run.awaiting_input");
    assert_eq!(audit[0].outcome, "AWAITING_INPUT");
    assert_eq!(audit[1].event_name, "agent.run.failed");
    assert_eq!(audit[1].metadata["code"], "AGENT_LIMIT_REACHED");
    assert!(!audit[1].metadata.contains_key("nativeStatus"));
    let serialized = serde_json::to_string(&audit).unwrap();
    assert!(!serialized.contains("private approval content"));
    assert!(!serialized.contains("private provider response"));
    assert!(
        audit
            .iter()
            .all(|event| event.event_name != "agent.run.completed")
    );
}

#[test]
fn native_private_events_are_filtered_on_public_read_without_rewriting_storage() {
    let path = database_path("private-visibility");
    let content = json!([
        {"type":"text","text":"public answer"},
        {"type":"json","value":{"tronMessagePart":{"type":"reasoning","text":"OLD_RAW_SENTINEL"}}},
        {"type":"json","value":{"tronMessagePart":{"type":"reasoningSummary","text":"public summary"}}},
        {"type":"json","value":{"tronMessagePart":{"type":"context","kind":"chat_attachment","text":"INTERNAL_ATTACHMENT_SENTINEL"}}},
        {"type":"json","value":{"tronMessagePart":{"type":"compactionCheckpoint","summary":"INTERNAL_CHECKPOINT_SENTINEL"}}},
        {"type":"media","media":{"mime_type":"image/png","source":{"type":"base64","value":"PUBLIC_MEDIA"}}},
        {"type":"json","value":{"public":{"tronMessagePart":{"type":"reasoning","text":"PUBLIC_NESTED_JSON"}}}}
    ]);
    let mut custom = terminal(1, "CUSTOM", serde_json::Value::Null);
    custom.data["name"] = json!("framework.message");
    custom.data["value"] =
        json!({"messageId":"OLD-MESSAGE","content":content.clone(),"structured":null});
    let mut finish = terminal(2, "RUN_FINISHED", json!({"type":"success"}));
    finish.data["result"] =
        json!({"content":content,"status":"Completed","output":"public answer"});
    let original: Vec<_> = [custom, finish]
        .into_iter()
        .map(|event| {
            (
                protocol_event_identity(&event).unwrap(),
                event.sse_event.clone(),
                serde_json::to_string(&event).unwrap(),
            )
        })
        .collect();
    {
        let store = replay_store(&path);
        native_blocking(async {
            let host = tron_backend::application::agent::FrameworkHost::persistent(
                path.to_str().unwrap(),
                None,
            );
            let sessions = host.storage_framework().await.unwrap().sessions().unwrap();
            for (id, _, payload) in &original {
                let frame: AgentAguiProtocolEvent = serde_json::from_str(payload).unwrap();
                let mut event = eliteforge_agent_core::SessionEvent::new(
                    "SESSION",
                    "tron.agui.protocol",
                    json!({"streamKey":"SESSION","frame":frame}),
                );
                event.id = id.clone();
                sessions.store.append_event(event).await.unwrap();
            }
        });
        assert!(
            serde_json::to_string(&store.replay_protocol_events("SESSION").unwrap())
                .unwrap()
                .contains("OLD_RAW_SENTINEL")
        );
    }
    let store = Arc::new(replay_store(&path));
    let service = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(&EventBusConfig::default())),
        store.clone(),
    );
    let public = service.protocol_events("SESSION").unwrap();
    let serialized = serde_json::to_string(&public).unwrap();
    assert!(!serialized.contains("OLD_RAW_SENTINEL"));
    assert!(!serialized.contains("INTERNAL_ATTACHMENT_SENTINEL"));
    assert!(!serialized.contains("INTERNAL_CHECKPOINT_SENTINEL"));
    for value in [
        "public answer",
        "public summary",
        "PUBLIC_MEDIA",
        "PUBLIC_NESTED_JSON",
    ] {
        assert!(serialized.contains(value));
    }
    assert_eq!(
        public[0].data["value"]["content"].as_array().unwrap().len(),
        4
    );
    assert_eq!(
        public[1].data["result"]["content"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    for (event, (id, _, _)) in public.iter().zip(&original) {
        assert_eq!(protocol_event_identity(event).as_deref(), Some(id.as_str()));
    }
    native_blocking(async {
        let host = tron_backend::application::agent::FrameworkHost::persistent(
            path.to_str().unwrap(),
            None,
        );
        let events = host
            .storage_framework()
            .await
            .unwrap()
            .sessions()
            .unwrap()
            .store
            .list_events("SESSION", 0, 100)
            .await
            .unwrap();
        for (id, _, bytes) in &original {
            let persisted = events.iter().find(|event| &event.id == id).unwrap();
            assert_eq!(
                &serde_json::to_string(&persisted.payload["frame"]).unwrap(),
                bytes,
                "public read must not alter original stored history"
            );
        }
    });
    drop(service);
    drop(store);
    let _ = std::fs::remove_file(path);
}

fn native_blocking<T: Send>(future: impl std::future::Future<Output = T> + Send) -> T {
    std::thread::scope(|scope| {
        scope
            .spawn(|| tokio::runtime::Runtime::new().unwrap().block_on(future))
            .join()
            .unwrap()
    })
}
fn replay_store(path: &std::path::Path) -> SqliteAgentEventReplayStore {
    let host =
        tron_backend::application::agent::FrameworkHost::persistent(path.to_str().unwrap(), None);
    native_blocking(async {
        let sessions = host.storage_framework().await.unwrap().sessions().unwrap();
        if matches!(
            sessions.get("SESSION").await,
            Err(eliteforge_agent_core::StateError::NotFound)
        ) {
            let mut record = eliteforge_agent_core::SessionRecord::new("Native replay");
            record.id = "SESSION".into();
            sessions.create(record).await.unwrap();
        }
    });
    let store = SqliteAgentEventReplayStore::new().unwrap();
    store.bind_framework(&host).unwrap();
    store
}
