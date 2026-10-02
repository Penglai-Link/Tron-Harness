use super::*;
use crate::{
    application::agent::{AgentAguiReplayStore, AgentAguiStreamError},
    domain::agent::{AgentAguiEvent, AgentRunLimits},
    shared::event_bus::{AguiEventSink, EventBusConfig},
};
use async_trait::async_trait;
use eliteforge_agent_core as fw;
use eliteforge_agent_framework::AgentBuilder;
use eliteforge_agent_official_plugin_agui::{AgUiOptions, AgUiPlugin};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

fn native_builder(storage: &tempfile::TempDir) -> AgentBuilder {
    let artifacts = storage.path().join("artifacts");
    if !artifacts.exists() {
        let mut directory = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            directory.mode(0o700);
        }
        directory.create(&artifacts).unwrap();
    }
    AgentBuilder::new().plugin(
        eliteforge_agent_official_plugin_storage_local::LocalStoragePlugin::new(
            storage.path().join("native.sqlite"),
            artifacts,
        ),
    )
}

fn context() -> AgentRunContext {
    serde_json::from_value(json!({
        "runCode":"PRODUCT-RUN", "conversationCode":"SESSION", "workspaceCode":"WORKSPACE", "userCode":"USER",
        "locale":"en-US", "requestId":null, "streamKey":"SESSION", "agentKind":"REACT", "systemAgentSessionId":null,
        "model":{"modelCode":"test", "protocol":"CHAT_COMPLETIONS", "baseUrl":"http://localhost", "apiKey":"", "modelName":"test"},
        "variables":[], "skills":[], "tools":[], "executionScope":{}, "limits":serde_json::to_value(AgentRunLimits::default()).unwrap(),
        "messages":[], "observations":[], "shortTermMemory":null, "startedAt":"2026-10-02T00:00:00Z"
    })).unwrap()
}
fn raw(sequence: u64) -> Value {
    json!({"type":"TEXT_MESSAGE_CONTENT","messageId":"MESSAGE","runId":"EXECUTION","delta":"ha","timestamp":1,
        "metadata":{"framework":{"runId":"LOGICAL","executionId":"EXECUTION","sourceSequence":sequence}}})
}
#[test]
fn identical_same_millisecond_chunks_are_distinct_source_events() {
    let first = project_native_event(raw(1), &context(), 0).unwrap();
    let second = project_native_event(raw(2), &context(), 0).unwrap();
    assert_ne!(
        crate::application::agent::protocol_event_identity(&first),
        crate::application::agent::protocol_event_identity(&second)
    );
    assert_eq!(
        first.data.pointer("/metadata/framework/runId"),
        Some(&json!("LOGICAL"))
    );
    assert_eq!(
        first.data.pointer("/metadata/framework/executionId"),
        Some(&json!("EXECUTION"))
    );
    assert_eq!(
        first.data.pointer("/metadata/tron/nativeRunId"),
        Some(&json!("EXECUTION"))
    );
    assert_eq!(first.data["runId"], "PRODUCT-RUN");
}
#[test]
fn one_source_event_expanding_to_multiple_frames_retains_each_ordinal() {
    let first = project_native_event(raw(1), &context(), 0).unwrap();
    let second = project_native_event(raw(1), &context(), 1).unwrap();
    assert_ne!(
        crate::application::agent::protocol_event_identity(&first),
        crate::application::agent::protocol_event_identity(&second)
    );
}
struct FailingReplay {
    attempts: AtomicUsize,
    incomplete: AtomicBool,
}
impl AgentAguiReplayStore for FailingReplay {
    fn append_protocol_event(&self, _: &str, _: AgentAguiProtocolEvent) -> Result<(), String> {
        self.attempts.fetch_add(1, Ordering::Relaxed);
        Err("disk unavailable".into())
    }
    fn replay_agui_events(&self, _: &str) -> Result<Vec<AgentAguiEvent>, String> {
        Err("read unavailable".into())
    }
    fn replay_protocol_events(&self, _: &str) -> Result<Vec<AgentAguiProtocolEvent>, String> {
        Err("read unavailable".into())
    }
    fn record_delivery_failure(&self, _: &str, _: &str) {
        self.incomplete.store(true, Ordering::Relaxed);
    }
    fn replay_failure(&self, _: &str) -> Option<String> {
        self.incomplete
            .load(Ordering::Relaxed)
            .then(|| "disk unavailable".into())
    }
}
struct TextLoop;
#[async_trait]
impl fw::AgentLoop for TextLoop {
    async fn run(
        &self,
        _: &fw::RunRequest,
        run: &fw::RunContext,
        _: &fw::AgentServices,
    ) -> Result<fw::RunResult, fw::AgentError> {
        run.emit_output(fw::OutputEvent::MessageStarted {
            message_id: "M".into(),
        });
        for _ in 0..2 {
            run.emit_output(fw::OutputEvent::TextDelta {
                message_id: "M".into(),
                delta: "ha".into(),
            });
        }
        run.emit_output(fw::OutputEvent::MessageFinished {
            message_id: "M".into(),
            content: "haha".into(),
            structured: None,
            tool_calls: vec![],
        });
        Ok(fw::RunResult {
            output: "haha".into(),
            content: "haha".into(),
            structured: None,
            interactions: vec![],
            status: fw::RunStatus::Completed,
            usage: fw::Usage::default(),
            steps: 1,
            tool_calls: 0,
            external_session: None,
        })
    }
}
#[tokio::test]
async fn persistent_write_failure_still_drains_native_stream_and_preserves_success() {
    let storage = tempfile::tempdir().unwrap();
    let agent = native_builder(&storage)
        .default_models(false)
        .plugin(AgUiPlugin::default())
        .plugin(fw::AgentLoopPlugin::new(TextLoop))
        .build()
        .await
        .unwrap();
    let (handle, stream) = agent
        .start_with_output::<AgUiStream>(fw::RunRequest::new("SESSION", "test"))
        .unwrap();
    let store = Arc::new(FailingReplay {
        attempts: AtomicUsize::new(0),
        incomplete: AtomicBool::new(false),
    });
    let service = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(&EventBusConfig::default())),
        store.clone(),
    );
    let context = context();
    let (result, ()) = tokio::join!(
        agent.wait(handle),
        consume_native_agui(stream, &context, Some(service.clone()))
    );
    assert_eq!(result.unwrap().status, fw::RunStatus::Completed);
    assert!(
        store.attempts.load(Ordering::Relaxed) >= 4,
        "failure must not stop consuming later native frames"
    );
    assert!(matches!(
        service.ensure_complete("SESSION"),
        Err(AgentAguiStreamError::ReplayIncomplete(_))
    ));
    agent.close().await;
}
#[tokio::test]
async fn native_output_overflow_reports_incomplete_without_changing_execution() {
    let storage = tempfile::tempdir().unwrap();
    let plugin = AgUiPlugin::new(AgUiOptions {
        capacity: 1,
        ..AgUiOptions::default()
    })
    .unwrap();
    let agent = native_builder(&storage)
        .default_models(false)
        .plugin(plugin)
        .plugin(fw::AgentLoopPlugin::new(TextLoop))
        .build()
        .await
        .unwrap();
    let (handle, stream) = agent
        .start_with_output::<AgUiStream>(fw::RunRequest::new("SESSION", "test"))
        .unwrap();
    // Intentionally finish the executor before draining the bounded presentation channel.
    assert_eq!(
        agent.wait(handle).await.unwrap().status,
        fw::RunStatus::Completed
    );
    let store = Arc::new(
        crate::adapters::outbound::persistence::SqliteAgentEventReplayStore::connect(":memory:")
            .unwrap(),
    );
    let service = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(&EventBusConfig::default())),
        store.clone(),
    );
    consume_native_agui(stream, &context(), Some(service.clone())).await;
    assert!(matches!(
        service.ensure_complete("SESSION"),
        Err(AgentAguiStreamError::ReplayIncomplete(_))
    ));
    let frames = store.replay_protocol_events("SESSION").unwrap();
    assert!(
        frames.iter().any(|frame| frame.sse_event == "RUN_ERROR"
            && frame
                .data
                .get("code")
                .and_then(Value::as_str)
                .is_some_and(|code| code.starts_with("AGUI_"))),
        "native output overflow must be observable independently of storage failure"
    );
    agent.close().await;
}

struct StalledReplay {
    attempts: AtomicUsize,
    release: std::sync::Mutex<bool>,
    wake: std::sync::Condvar,
    incomplete: AtomicBool,
}
impl AgentAguiReplayStore for StalledReplay {
    fn append_protocol_event(&self, _: &str, _: AgentAguiProtocolEvent) -> Result<(), String> {
        self.attempts.fetch_add(1, Ordering::Relaxed);
        let mut release = self.release.lock().unwrap();
        while !*release {
            release = self.wake.wait(release).unwrap();
        }
        Ok(())
    }
    fn replay_agui_events(&self, _: &str) -> Result<Vec<AgentAguiEvent>, String> {
        Ok(vec![])
    }
    fn replay_protocol_events(&self, _: &str) -> Result<Vec<AgentAguiProtocolEvent>, String> {
        Ok(vec![])
    }
    fn record_delivery_failure(&self, _: &str, _: &str) {
        self.incomplete.store(true, Ordering::Relaxed);
    }
}
#[tokio::test]
async fn stalled_replay_write_does_not_hold_execution_or_spawn_unbounded_writes() {
    let storage = tempfile::tempdir().unwrap();
    let agent = native_builder(&storage)
        .default_models(false)
        .plugin(AgUiPlugin::default())
        .plugin(fw::AgentLoopPlugin::new(TextLoop))
        .build()
        .await
        .unwrap();
    let (handle, stream) = agent
        .start_with_output::<AgUiStream>(fw::RunRequest::new("SESSION", "test"))
        .unwrap();
    let store = Arc::new(StalledReplay {
        attempts: AtomicUsize::new(0),
        release: std::sync::Mutex::new(false),
        wake: std::sync::Condvar::new(),
        incomplete: AtomicBool::new(false),
    });
    let service = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(&EventBusConfig::default())),
        store.clone(),
    );
    let context = context();
    let done = tokio::time::timeout(std::time::Duration::from_secs(1), async {
        tokio::join!(
            agent.wait(handle),
            consume_native_agui_with_deadline(
                stream,
                &context,
                Some(service),
                std::time::Duration::from_millis(20)
            )
        )
    })
    .await;
    // Always release the test's blocking write even when the assertion fails.
    *store.release.lock().unwrap() = true;
    store.wake.notify_all();
    let (result, ()) = done.expect("presentation drain must have a finite acknowledgment deadline");
    assert_eq!(result.unwrap().status, fw::RunStatus::Completed);
    assert_eq!(store.attempts.load(Ordering::Relaxed), 1);
    assert!(store.incomplete.load(Ordering::Relaxed));
    agent.close().await;
}

struct PausedChildLoop;
#[async_trait]
impl fw::AgentLoop for PausedChildLoop {
    async fn run(
        &self,
        _: &fw::RunRequest,
        _: &fw::RunContext,
        _: &fw::AgentServices,
    ) -> Result<fw::RunResult, fw::AgentError> {
        Ok(fw::RunResult {
            output: "".into(),
            content: "".into(),
            structured: None,
            interactions: vec![fw::InteractionRequest::approval(
                fw::RunId::new_v4(),
                "Allow the next operation?",
            )],
            status: fw::RunStatus::AwaitingInput,
            usage: fw::Usage::default(),
            steps: 1,
            tool_calls: 0,
            external_session: None,
        })
    }
}
struct NativeChildrenLoop {
    children: Vec<eliteforge_agent_framework::Agent>,
    presentation: AgentAguiStreamService,
}
#[async_trait]
impl fw::AgentLoop for NativeChildrenLoop {
    async fn run(
        &self,
        _: &fw::RunRequest,
        parent: &fw::RunContext,
        _: &fw::AgentServices,
    ) -> Result<fw::RunResult, fw::AgentError> {
        let mut result = None;
        for (index, child) in self.children.iter().enumerate() {
            let (handle, output) = child
                .start_child_with_output_owned::<AgUiStream>(
                    fw::RunRequest::new(format!("CHILD-{index}"), "test"),
                    parent,
                )
                .await?;
            let mut product = context();
            product.run_code = format!("PRODUCT-CHILD-{index}");
            let (actual, ()) = tokio::join!(
                handle.wait(),
                consume_native_agui(output, &product, Some(self.presentation.clone()))
            );
            result = Some(actual?);
        }
        Ok(result.unwrap())
    }
}
#[tokio::test]
async fn real_native_multi_child_stream_preserves_distinct_source_ids_and_paused_child_terminal() {
    let storage = tempfile::tempdir().unwrap();
    use crate::adapters::outbound::persistence::SqliteAgentEventReplayStore;
    let store = Arc::new(SqliteAgentEventReplayStore::connect(":memory:").unwrap());
    let presentation = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(
            &crate::shared::event_bus::EventBusConfig::default(),
        )),
        store,
    );
    let first = native_builder(&storage)
        .default_models(false)
        .plugin(AgUiPlugin::default())
        .plugin(fw::AgentLoopPlugin::new(TextLoop))
        .build()
        .await
        .unwrap();
    let second = native_builder(&storage)
        .default_models(false)
        .plugin(AgUiPlugin::default())
        .plugin(fw::AgentLoopPlugin::new(PausedChildLoop))
        .build()
        .await
        .unwrap();
    let parent = native_builder(&storage)
        .default_models(false)
        .plugin(AgUiPlugin::default())
        .plugin(fw::AgentLoopPlugin::new(NativeChildrenLoop {
            children: vec![first.clone(), second.clone()],
            presentation: presentation.clone(),
        }))
        .build()
        .await
        .unwrap();
    let (handle, output) = parent
        .start_with_output::<AgUiStream>(fw::RunRequest::new("GOAL-NATIVE", "test"))
        .unwrap();
    let (result, ()) = tokio::join!(
        parent.wait(handle),
        consume_native_agui_group(output, "SESSION", "INITIAL", Some(presentation.clone()))
    );
    assert_eq!(result.unwrap().status, fw::RunStatus::AwaitingInput);
    let frames = presentation.protocol_events("SESSION").unwrap();
    let starts = frames
        .iter()
        .filter(|event| event.sse_event == "SUBAGENT_STARTED")
        .collect::<Vec<_>>();
    assert_eq!(starts.len(), 2);
    assert_ne!(
        starts[0].data["subagentRunId"],
        starts[1].data["subagentRunId"]
    );
    assert!(starts.iter().all(
        |event| event.data["runId"].as_str().unwrap().starts_with("goal:")
            && event.data.pointer("/metadata/tron/parentProductRunCode") == Some(&json!("INITIAL"))
    ));
    for (index, start) in starts.iter().enumerate() {
        let terminal = frames
            .iter()
            .find(|event| {
                event.sse_event == "RUN_FINISHED"
                    && event.data["runId"] == format!("PRODUCT-CHILD-{index}")
            })
            .unwrap();
        assert_eq!(
            terminal.data.pointer("/metadata/framework/runId"),
            Some(&start.data["subagentRunId"])
        );
        if index == 1 {
            assert_eq!(terminal.data["outcome"]["type"], "interrupt");
            assert_eq!(
                terminal.data["outcome"]["interrupts"][0]["responseSchema"]["type"],
                "boolean"
            );
        }
    }
    parent.close().await;
    first.close().await;
    second.close().await;
}

struct PrivatePresentationLoop;
#[async_trait]
impl fw::AgentLoop for PrivatePresentationLoop {
    async fn run(
        &self,
        _: &fw::RunRequest,
        run: &fw::RunContext,
        _: &fw::AgentServices,
    ) -> Result<fw::RunResult, fw::AgentError> {
        let content = fw::Content(vec![
            fw::ContentBlock::Text {
                text: "public answer".into(),
            },
            fw::ContentBlock::Json {
                value: json!({"tronMessagePart":{"type":"reasoning","text":"PRIVATE_RAW_SENTINEL"}}),
            },
            fw::ContentBlock::Json {
                value: json!({"tronMessagePart":{"type":"reasoningSummary","text":"public summary"}}),
            },
            fw::ContentBlock::Media {
                media: fw::MediaContent {
                    mime_type: "image/png".into(),
                    source: fw::MediaSource::Base64("PUBLIC_MEDIA".into()),
                    name: Some("image".into()),
                    estimated_tokens: Some(1),
                },
            },
            fw::ContentBlock::Json {
                value: json!({"public":{"tronMessagePart":{"type":"reasoning","text":"PUBLIC_NESTED_JSON"}}}),
            },
        ]);
        run.emit_output(fw::OutputEvent::MessageStarted {
            message_id: "PRIVATE-MESSAGE".into(),
        });
        run.emit_output(fw::OutputEvent::MessageFinished {
            message_id: "PRIVATE-MESSAGE".into(),
            content: content.clone(),
            structured: None,
            tool_calls: vec![],
        });
        Ok(fw::RunResult {
            output: "public answer".into(),
            content,
            structured: None,
            interactions: vec![],
            status: fw::RunStatus::Completed,
            usage: fw::Usage::default(),
            steps: 1,
            tool_calls: 0,
            external_session: None,
        })
    }
}
#[tokio::test]
async fn native_public_replay_hides_only_private_product_reasoning_and_preserves_original_content()
{
    use crate::adapters::outbound::persistence::SqliteAgentEventReplayStore;
    let storage = tempfile::tempdir().unwrap();
    let agent = native_builder(&storage)
        .plugin(AgUiPlugin::default())
        .plugin(fw::AgentLoopPlugin::new(PrivatePresentationLoop))
        .build()
        .await
        .unwrap();
    let (handle, stream) = agent
        .start_with_output::<AgUiStream>(fw::RunRequest::new("SESSION", "test"))
        .unwrap();
    let path = storage.path().join("public-replay.sqlite");
    let replay = Arc::new(SqliteAgentEventReplayStore::connect(path.to_str().unwrap()).unwrap());
    let service = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(&EventBusConfig::default())),
        replay.clone(),
    );
    let context = context();
    let (result, ()) = tokio::join!(
        agent.wait(handle),
        consume_native_agui(stream, &context, Some(service.clone()))
    );
    let result = result.unwrap();
    assert_eq!(result.status, fw::RunStatus::Completed);
    assert_eq!(result.content.0.len(), 5);
    assert!(
        serde_json::to_string(&result.content)
            .unwrap()
            .contains("PRIVATE_RAW_SENTINEL")
    );
    let events = service.protocol_events("SESSION").unwrap();
    let message = events
        .iter()
        .find(|event| event.data["name"] == "framework.message")
        .unwrap();
    let terminal = events
        .iter()
        .find(|event| event.sse_event == "RUN_FINISHED")
        .unwrap();
    assert_eq!(
        message.data["value"]["content"].as_array().unwrap().len(),
        4
    );
    assert_eq!(
        terminal.data["result"]["content"].as_array().unwrap().len(),
        4
    );
    let public = serde_json::to_string(&events).unwrap();
    assert!(!public.contains("PRIVATE_RAW_SENTINEL"));
    for value in [
        "public answer",
        "public summary",
        "PUBLIC_MEDIA",
        "PUBLIC_NESTED_JSON",
    ] {
        assert!(public.contains(value));
    }
    for event in &events {
        assert!(
            crate::application::agent::protocol_event_identity(event)
                .unwrap()
                .starts_with("native:")
        );
        assert!(
            event
                .data
                .pointer("/metadata/framework/executionId")
                .is_some()
        );
        assert!(
            event
                .data
                .pointer("/metadata/framework/sourceSequence")
                .is_some()
        );
    }
    assert!(!service.events("SESSION").unwrap().replay_incomplete);
    drop(service);
    drop(replay);
    let reopened = SqliteAgentEventReplayStore::connect(path.to_str().unwrap()).unwrap();
    assert_eq!(reopened.replay_protocol_events("SESSION").unwrap(), events);
    agent.close().await;
}
