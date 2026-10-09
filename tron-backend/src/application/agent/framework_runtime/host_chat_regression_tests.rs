use super::*;
use crate::application::agent::{AgentAguiStreamService, ReactAgentBuilder};
use crate::application::short_term_memory::{ShortTermMemoryConfig, ShortTermMemoryService};
use crate::domain::short_term_memory::ShortTermMemoryError;

struct SlowSummaryModel {
    entered: Arc<Notify>,
    release: Arc<Notify>,
}
impl ModelClient for SlowSummaryModel {
    fn generate<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        self.generate_once(config, request)
    }
    fn generate_once<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            if request.messages.iter().any(|message| {
                message
                    .text_content()
                    .contains("Summarize historical conversation")
            }) {
                self.entered.notify_one();
                self.release.notified().await;
            }
            StorageAcceptanceModel.generate_once(config, request).await
        })
    }
    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async {})
    }
    fn close_observed<'a>(
        &'a self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<eliteforge_agent_plugin_core::ShutdownReport, String>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async { Ok(Default::default()) })
    }
}

#[tokio::test]
async fn background_summary_allows_next_chat_turn_and_rejects_a_stale_snapshot() {
    let host = FrameworkHost::temporary(None).unwrap();
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let model = Arc::new(SlowSummaryModel {
        entered: entered.clone(),
        release: release.clone(),
    });
    host.bind_model_client(model.clone()).unwrap();
    let memory = ShortTermMemoryService::with_host(host.clone(), ShortTermMemoryConfig::default());
    let harness = ReactAgentBuilder::new(
        model.clone(),
        Arc::new(ToolRegistry::default()),
        crate::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let mut context = test_context(0);
    context.run_code = fw::RunId::new_v4().to_string();
    context.conversation_code = None;
    context.stream_key = Some(context.run_code.clone());
    let session = context.run_code.clone();
    within(host.run(harness.clone(), context.clone(), None, None))
        .await
        .unwrap();

    let summary_memory = memory.clone();
    let summary_session = session.clone();
    let summary_model = model.clone();
    let config = context.model.clone();
    let summary = tokio::spawn(async move {
        summary_memory
            .summarize(&summary_session, &config, summary_model, None, None)
            .await
    });
    within(entered.notified()).await;
    assert!(!summary.is_finished());

    context.run_code = fw::RunId::new_v4().to_string();
    context.conversation_code = Some(session.clone());
    context.stream_key = Some(session.clone());
    context.messages = vec![AgentMessage::user("simplify the answer")];
    let second = within(host.run(harness, context.clone(), None, None))
        .await
        .unwrap();
    assert_eq!(second.final_output, "native reply");
    assert!(
        !summary.is_finished(),
        "the next turn must not wait for summary completion"
    );

    release.notify_one();
    assert!(matches!(
        within(summary).await.unwrap(),
        Err(ShortTermMemoryError::CompactionConflict)
    ));
    assert!(
        memory.latest_compaction(&session).unwrap().is_none(),
        "a stale summary must not be published"
    );

    release.notify_one();
    let fresh = within(memory.summarize(&session, &context.model, model, None, None))
        .await
        .unwrap();
    assert_eq!(fresh["coveredMessages"], 4);
    let history = host
        .framework()
        .await
        .unwrap()
        .sessions()
        .unwrap()
        .store
        .list_runs(&session)
        .await
        .unwrap();
    assert!(
        history
            .iter()
            .filter(|run| run.kind == "SUMMARY")
            .all(|run| run.execution_scope == "tron.session-summary")
    );
    assert!(
        history
            .iter()
            .filter(|run| run.kind == "REACT")
            .all(|run| run.status == fw::ManagedRunStatus::Completed)
    );
    assert!(host.close().await.is_success());
}

struct ThinkingModel {
    kind: ModelStreamEventKind,
    entered: Arc<Notify>,
    release: Arc<Notify>,
}
impl ModelClient for ThinkingModel {
    fn generate<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        StorageAcceptanceModel.generate_once(config, request)
    }
    fn generate_stream<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        _: &'a ModelRequest,
        sink: ModelStreamSink,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            for delta in ["", " \n", "依据文档", "与检索结果整理回答。"] {
                sink(ModelStreamEvent {
                    kind: self.kind,
                    delta: delta.into(),
                });
            }
            self.entered.notify_one();
            self.release.notified().await;
            sink(ModelStreamEvent::text_delta("native reply"));
            Ok(ModelResponse::final_output("native reply"))
        })
    }
    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async {})
    }
    fn close_observed<'a>(
        &'a self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<eliteforge_agent_plugin_core::ShutdownReport, String>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async { Ok(Default::default()) })
    }
}

#[tokio::test]
async fn provider_thinking_and_summary_reach_live_native_events_and_durable_replay() {
    use crate::adapters::outbound::persistence::SqliteAgentEventReplayStore;
    use crate::shared::event_bus::{AguiEventSink, EventBusConfig};

    for kind in [
        ModelStreamEventKind::Reasoning,
        ModelStreamEventKind::ReasoningSummary,
    ] {
        let host = FrameworkHost::temporary(None).unwrap();
        let entered = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let model = Arc::new(ThinkingModel {
            kind,
            entered: entered.clone(),
            release: release.clone(),
        });
        host.bind_model_client(model.clone()).unwrap();
        let path = host.0.path.clone();
        let replay =
            Arc::new(SqliteAgentEventReplayStore::connect(path.to_str().unwrap()).unwrap());
        let service = AgentAguiStreamService::with_replay_store(
            Arc::new(AguiEventSink::new(&EventBusConfig::default())),
            replay,
        );
        host.bind_presentation(service.clone()).unwrap();
        let harness = ReactAgentBuilder::new(
            model,
            Arc::new(ToolRegistry::default()),
            crate::shared::event_bus::EventBusHandle::disabled(),
        )
        .with_framework_host(Some(host.clone()))
        .build()
        .unwrap();
        let mut context = test_context(0);
        context.run_code = fw::RunId::new_v4().to_string();
        context.conversation_code = None;
        context.stream_key = Some(context.run_code.clone());
        let session = context.run_code.clone();
        let running_host = host.clone();
        // Native AG-UI output must work without the optional product callback.
        let running =
            tokio::spawn(async move { running_host.run(harness, context, None, None).await });
        within(entered.notified()).await;
        within(async {
            loop {
                let events = service.protocol_events(&session).unwrap();
                if events
                    .iter()
                    .filter(|event| event.sse_event == "REASONING_MESSAGE_CONTENT")
                    .count()
                    == 2
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await;
        assert!(
            !running.is_finished(),
            "reasoning must be visible while the model is running"
        );
        release.notify_one();
        assert_eq!(
            within(running).await.unwrap().unwrap().final_output,
            "native reply"
        );
        let events = service.protocol_events(&session).unwrap();
        let content = events
            .iter()
            .filter(|event| event.sse_event == "REASONING_MESSAGE_CONTENT")
            .map(|event| event.data["delta"].as_str().unwrap())
            .collect::<String>();
        assert_eq!(content, "依据文档与检索结果整理回答。");
        for event_type in [
            "REASONING_START",
            "REASONING_MESSAGE_START",
            "REASONING_MESSAGE_END",
            "REASONING_END",
        ] {
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event.sse_event == event_type)
                    .count(),
                1
            );
        }
        assert!(!service.events(&session).unwrap().replay_incomplete);
        let reopened = SqliteAgentEventReplayStore::connect(path.to_str().unwrap()).unwrap();
        use crate::application::agent::AgentAguiReplayStore;
        assert_eq!(reopened.replay_protocol_events(&session).unwrap(), events);
        assert!(host.close().await.is_success());
    }
}
