use super::*;
#[tokio::test]
async fn dropped_completion_sender_ends_with_explicit_unknown_product_boundary() {
    let (sender, mut receiver) = tokio::sync::mpsc::channel::<ChatSsePayload>(1);
    let (started, ready) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let _sender = sender;
        let _ = started.send(());
        std::future::pending::<()>().await;
    });
    ready.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let (done, incomplete) = poll_product_completion(&mut receiver, "SESSION", "RUN");
    assert!(incomplete);
    let done = done.expect("a dropped producer must end the stream");
    let unknown = product_completion_payload(None, done, true);
    assert_eq!(unknown.data["code"], "CHAT_STREAM_COMPLETION_UNAVAILABLE");
    assert_eq!(unknown.data["presentationIncomplete"], true);
    assert_eq!(
        unknown.data["productCompletionError"]["code"],
        "CHAT_STREAM_COMPLETION_UNAVAILABLE"
    );
}
#[test]
fn lost_product_ack_does_not_replace_an_authoritative_native_success_or_pause() {
    for outcome in [
        json!({"type":"success"}),
        json!({"type":"interrupt","interrupts":[{"id":"INPUT","reason":"input_required"}]}),
    ] {
        let native = crate::domain::agent::AgentAguiProtocolEvent {
            sse_event: "RUN_FINISHED".into(),
            data: json!({"type":"RUN_FINISHED","runId":"RUN","outcome":outcome,"metadata":{"tron":{"eventId":"native:EXEC:1:0"}}}),
        };
        let actual = product_completion_payload(
            Some(native),
            completion_channel_closed_payload("SESSION", "RUN"),
            true,
        );
        assert_eq!(actual.event_name, "RUN_FINISHED");
        assert_eq!(actual.data["outcome"], outcome);
        assert_eq!(
            actual.data["productCompletionError"]["code"],
            "CHAT_STREAM_COMPLETION_UNAVAILABLE"
        );
    }
}

#[tokio::test]
async fn stalled_replay_reader_has_one_bounded_acknowledgment_and_does_not_block_completion() {
    use crate::{
        application::agent::{AgentAguiReplayStore, AgentAguiStreamService},
        domain::agent::{AgentAguiEvent, AgentAguiProtocolEvent},
        shared::event_bus::AguiEventSink,
    };
    use std::sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    };
    struct StalledReader {
        release: Arc<(Mutex<bool>, Condvar)>,
        incomplete: AtomicBool,
    }
    impl AgentAguiReplayStore for StalledReader {
        fn append_protocol_event(&self, _: &str, _: AgentAguiProtocolEvent) -> Result<(), String> {
            Ok(())
        }
        fn replay_agui_events(&self, _: &str) -> Result<Vec<AgentAguiEvent>, String> {
            Ok(vec![])
        }
        fn replay_protocol_events(&self, _: &str) -> Result<Vec<AgentAguiProtocolEvent>, String> {
            let (lock, ready) = &*self.release;
            let mut released = lock.lock().unwrap();
            while !*released {
                released = ready.wait(released).unwrap();
            }
            Ok(vec![])
        }
        fn record_delivery_failure(&self, _: &str, _: &str) {
            self.incomplete.store(true, Ordering::SeqCst);
        }
    }
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let store = Arc::new(StalledReader {
        release: release.clone(),
        incomplete: AtomicBool::new(false),
    });
    let service = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(
            &crate::shared::event_bus::EventBusConfig::default(),
        )),
        store.clone(),
    );
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        read_protocol_with_deadline(service, "SESSION".into(), None, Duration::from_millis(20)),
    )
    .await;
    // Always release the detached blocking reader before assertions/Runtime drop.
    *release.0.lock().unwrap() = true;
    release.1.notify_all();
    let (events, cursor, stalled) = result.expect("HTTP replay acknowledgment must be bounded");
    assert!(events.is_err());
    assert!(cursor.is_none());
    assert!(stalled);
    assert!(store.incomplete.load(Ordering::SeqCst));
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    drop(sender);
    assert!(
        poll_product_completion(&mut receiver, "SESSION", "RUN")
            .0
            .is_some()
    );
}

#[test]
fn native_relation_routes_multiple_children_without_rekeying_or_unrelated_session_frames() {
    use crate::domain::agent::AgentAguiProtocolEvent;
    let child = |code: &str, native: &str, status: Value| AgentAguiProtocolEvent {
        sse_event: "RUN_FINISHED".into(),
        data: json!({"type":"RUN_FINISHED","threadId":"SESSION","runId":code,"outcome":status,"metadata":{"framework":{"runId":native},"tron":{"eventId":format!("native:{native}:1:0")}}}),
    };
    let paused = child(
        "PRODUCT-CHILD-2",
        "NATIVE-CHILD-2",
        json!({"type":"interrupt","interrupts":[{"id":"APPROVAL","reason":"approval_required","responseSchema":{"type":"boolean"}}]}),
    );
    let events = vec![
        child(
            "PRODUCT-CHILD-1",
            "NATIVE-CHILD-1",
            json!({"type":"success"}),
        ),
        paused.clone(),
        AgentAguiProtocolEvent {
            sse_event: "SUBAGENT_STARTED".into(),
            data: json!({"type":"SUBAGENT_STARTED","runId":"goal:ROOT","subagentRunId":"NATIVE-CHILD-1","metadata":{"framework":{"runId":"ROOT"},"tron":{"parentProductRunCode":"INITIAL"}}}),
        },
        AgentAguiProtocolEvent {
            sse_event: "SUBAGENT_STARTED".into(),
            data: json!({"type":"SUBAGENT_STARTED","runId":"goal:ROOT","subagentRunId":"NATIVE-CHILD-2","metadata":{"framework":{"runId":"ROOT"},"tron":{"parentProductRunCode":"INITIAL"}}}),
        },
    ];
    let mut children = std::collections::BTreeSet::new();
    resolve_native_child_routes(&events, "INITIAL", &mut children);
    assert!(
        events
            .iter()
            .all(|event| protocol_event_belongs_to_invocation_tree(
                &event.data,
                "INITIAL",
                &children
            ))
    );
    assert!(!protocol_event_belongs_to_invocation_tree(
        &child("UNRELATED", "OTHER", json!({"type":"success"})).data,
        "INITIAL",
        &children
    ));
    assert_eq!(events[0].data["runId"], "PRODUCT-CHILD-1");
    assert_eq!(paused.data["runId"], "PRODUCT-CHILD-2");
    let done = ChatSsePayload::agui(
        "RUN_FINISHED",
        json!({"type":"RUN_FINISHED","runId":"PRODUCT-CHILD-2","result":{"run":{"runCode":"PRODUCT-CHILD-2","status":"AWAITING_USER_INPUT"}}}),
    );
    let final_event = product_completion_payload(Some(paused.clone()), done, false);
    assert_eq!(final_event.data["runId"], "PRODUCT-CHILD-2");
    assert_eq!(final_event.data["outcome"], paused.data["outcome"]);
}

#[test]
fn lost_product_ack_preserves_authoritative_native_failure_code_and_identity() {
    let native = crate::domain::agent::AgentAguiProtocolEvent {
        sse_event: "RUN_ERROR".into(),
        data: json!({"type":"RUN_ERROR","runId":"RUN","code":"MODEL_PROVIDER_FAILED","metadata":{"framework":{"runId":"NATIVE","executionId":"EXEC","sourceSequence":3},"tron":{"eventId":"native:EXEC:3:0"}}}),
    };
    let output = product_completion_payload(
        Some(native.clone()),
        completion_channel_closed_payload("SESSION", "RUN"),
        true,
    );
    assert_eq!(output.data["code"], "MODEL_PROVIDER_FAILED");
    assert_eq!(output.data["metadata"], native.data["metadata"]);
    assert_eq!(
        output.data["productCompletionError"]["code"],
        "CHAT_STREAM_COMPLETION_UNAVAILABLE"
    );
    assert_eq!(output.data["presentationIncomplete"], true);
}

#[test]
fn presentation_error_frames_do_not_replace_native_execution_terminals() {
    use crate::domain::agent::AgentAguiProtocolEvent;
    let frame = |kind: &str, data: serde_json::Value| AgentAguiProtocolEvent {
        sse_event: kind.into(),
        data,
    };
    assert!(native_protocol_execution_terminal(&frame(
        "RUN_FINISHED",
        json!({"outcome":{"type":"success"}})
    )));
    assert!(native_protocol_execution_terminal(&frame(
        "RUN_FINISHED",
        json!({"outcome":{"type":"interrupt"}})
    )));
    assert!(native_protocol_execution_terminal(&frame(
        "RUN_ERROR",
        json!({"code":"MODEL_PROVIDER_FAILED","productCompletionError":{"code":"CHAT_STREAM_COMPLETION_UNAVAILABLE"}})
    )));
    assert!(!native_protocol_execution_terminal(&frame(
        "RUN_ERROR",
        json!({"code":"AGUI_BACKPRESSURE"})
    )));
    assert!(!native_protocol_execution_terminal(&frame(
        "RUN_ERROR",
        json!({"code":"CHAT_STREAM_COMPLETION_UNAVAILABLE"})
    )));
    assert!(!native_protocol_execution_terminal(&frame(
        "RUN_FINISHED",
        json!({})
    )));
}
