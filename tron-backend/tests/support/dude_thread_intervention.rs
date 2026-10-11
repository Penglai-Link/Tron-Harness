//! Exercises production Thread tools and human intervention against the real SDK engine.
use super::*;
use std::{
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::Duration,
};
use tron_backend::{
    application::agent::{AgentInvocationService, ThreadFeedback, ThreadFeedbackSink},
    domain::agent::AgentCollaborationOwner,
};

#[derive(Default)]
struct HumanScript {
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
    child_started: AtomicBool,
    questions: AtomicUsize,
    thread_id: Mutex<Option<String>>,
}
impl ModelClient for HumanScript {
    fn generate<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        _: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async { Err("native execution required".into()) })
    }
    fn generate_native_once<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        request: &'a fw::ChatRequest,
        _: &'a ModelRequest,
        _: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            let results: Vec<_> = request
                .messages
                .iter()
                .filter_map(|m| m.tool_result.as_ref().map(|_| m.content.text()))
                .collect();
            if request
                .messages
                .iter()
                .any(|m| m.content.text() == "root:dude-native")
            {
                if results.is_empty() {
                    return Ok(ModelResponse::tool_call(
                        "thread_create",
                        json!({"agentCode":"reviewer","task":"child:needs-human","title":"Dude delegated work"}),
                    ));
                }
                let created: Value = serde_json::from_str(&results[0])
                    .map_err(|e| format!("{e}: {}", results[0]))?;
                let id = created["thread_id"]
                    .as_str()
                    .ok_or_else(|| format!("thread did not start: {created}"))?;
                *self.thread_id.lock().unwrap() = Some(id.into());
                return Ok(ModelResponse::final_output("parent-finished-before-child"));
            }
            if request
                .messages
                .iter()
                .rev()
                .find(|m| m.role == fw::Role::User)
                .is_some_and(|m| m.content.text() == "human:followup")
            {
                return Ok(ModelResponse::final_output("human-followup-completed"));
            }
            if results.is_empty() {
                if !self.child_started.swap(true, Ordering::AcqRel) {
                    self.entered.notify_one();
                    self.release.notified().await;
                }
                self.questions.fetch_add(1, Ordering::AcqRel);
                assert!(request.tools.iter().any(|t| t.call_name == "chat_ask_user"));
                assert!(
                    request
                        .messages
                        .iter()
                        .filter(|message| message.role == fw::Role::User)
                        .flat_map(|message| &message.content.0)
                        .any(|block| match block {
                            fw::ContentBlock::Json { value } => {
                                let part = &value["tronMessagePart"];
                                part["type"] == "context"
                                    && part["kind"] == "session_family_context"
                                    && part["text"]
                                        .as_str()
                                        .is_some_and(|text| text.contains("trusted-dude-family-note"))
                            }
                            _ => false,
                        }),
                    "native Thread must load its own authorized STM family source"
                );
                return Ok(ModelResponse::tool_call(
                    "chat_ask_user",
                    json!({"intro":"Choose deployment target","questions":[{"question":"Which target?","options":[{"label":"Staging"},{"label":"Production"}]}]}),
                ));
            }
            Ok(ModelResponse::final_output("answered-work-completed"))
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
#[derive(Default)]
struct Feedback {
    events: Mutex<Vec<ThreadFeedback>>,
    arrived: tokio::sync::Notify,
}
#[async_trait::async_trait]
impl ThreadFeedbackSink for Feedback {
    async fn publish(&self, event: ThreadFeedback) -> Result<(), String> {
        self.events.lock().unwrap().push(event);
        self.arrived.notify_one();
        Ok(())
    }
}
impl Feedback {
    async fn until(&self, count: usize) {
        tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                let notified = self.arrived.notified();
                if self.events.lock().unwrap().len() >= count {
                    break;
                }
                notified.await;
            }
        })
        .await
        .expect("Thread proactively publishes feedback without polling");
    }
}
fn intervention_service(f: &Fixture, model: Arc<HumanScript>) -> AgentInvocationService {
    let router = tron_backend::application::model_router::AgentModelRouter::new(
        f.state.models.clone(),
        f.state.model_providers.clone(),
    )
    .with_agent_management(f.state.agent_management.clone());
    AgentInvocationService::new(
        f.state.workspaces.clone(),
        f.state.apps.clone(),
        f.state.variables.clone(),
        f.state.skills.clone(),
        f.state.agent_management.clone(),
        f.state.agent_runs.clone(),
        f.state.mcp.clone(),
        EventBusHandle::disabled(),
        model,
        f.state.agent_tool_registry.clone(),
        f.state.runtime_http_client.clone(),
    )
    .with_framework_host(f.state.agent_framework.clone())
    .with_short_term_memory(f.state.short_term_memory.clone())
    .with_prompt_service(f.state.prompts.clone())
    .with_active_runs(f.state.active_runs.clone())
    .with_auto_model_route(None, Some(router))
}
// Match the production runtime: synchronous STM reads must yield while native parent/child work owns async storage gates.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dude_thread_survives_parent_exit_and_human_answer_and_message_keep_native_ownership() {
    let f = Fixture::new();
    reviewer(&f);
    let script = Arc::new(HumanScript::default());
    let sink = Arc::new(Feedback::default());
    let host = f.state.agent_framework.clone();
    host.set_thread_feedback_sink(sink.clone()).unwrap();
    let router = tron_backend::application::model_router::AgentModelRouter::new(
        f.state.models.clone(),
        f.state.model_providers.clone(),
    )
    .with_agent_management(f.state.agent_management.clone());
    let agent = ReactAgentBuilder::new(
        script.clone(),
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .with_short_term_memory(Some(f.state.short_term_memory.clone()))
    .with_auto_model_router(Some(router))
    .build()
    .unwrap();
    f.state
        .chats
        .ensure_dude_session(
            &tron_backend::domain::dude::DudeSession {
                session_code: "original-dude-work-session".into(),
                dude_code: "main-agent".into(),
                purpose: tron_backend::domain::dude::DudeSessionPurpose::Task,
                correlation_key: "dude-task".into(),
                title: "Dude work".into(),
                workspace_code: Some(WORKSPACE.into()),
                created_at: 0,
                updated_at: 0,
            },
            Some(USER.into()),
        )
        .unwrap();
    host.storage_framework()
        .await
        .unwrap()
        .sessions()
        .unwrap()
        .write_context(
            "original-dude-work-session",
            "tron.session-summary-view",
            None,
            json!({"summary":"trusted-dude-family-note"}),
        )
        .await
        .unwrap();
    let mut parent = context("root:dude-native");
    parent.conversation_code = Some("original-dude-work-session".into());
    parent.collaboration_owner = Some(AgentCollaborationOwner {
        kind: "DUDE".into(),
        code: "main-agent".into(),
        origin_session_code: Some("original-dude-work-session".into()),
        task_code: Some("dude-task".into()),
    });
    let parent = tokio::time::timeout(Duration::from_secs(30), agent.run(parent))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(parent.final_output, "parent-finished-before-child");
    let id = script.thread_id.lock().unwrap().clone().unwrap();
    tokio::time::timeout(Duration::from_secs(30), script.entered.notified())
        .await
        .unwrap();
    assert!(
        sink.events.lock().unwrap().is_empty(),
        "child remains blocked after the initiating Run finishes"
    );
    script.release.notify_one();
    sink.until(1).await;
    let first = sink.events.lock().unwrap()[0].clone();
    assert_eq!(first.status, fw::ThreadStatus::AwaitingInput);
    assert_eq!(first.owner.code, "main-agent");
    assert_eq!(first.owner.task_code.as_deref(), Some("dude-task"));
    assert_eq!(first.origin_session_code, "original-dude-work-session");
    let interaction = &first.result.as_ref().unwrap().interactions[0];
    let question = interaction.presentation.as_ref().unwrap()["questions"][0]["id"]
        .as_str()
        .unwrap();
    let answer = json!({"answers":{(question):{"text":"Staging"}}});
    let service = intervention_service(&f, script.clone());
    assert!(
        host.dude_thread_history("other-dude", &id, Default::default())
            .await
            .is_err()
    );
    assert!(
        host.dude_thread_answer(
            "other-dude",
            &id,
            interaction.id,
            answer.clone(),
            USER,
            "en-US",
            &service
        )
        .await
        .is_err()
    );
    assert!(
        host.dude_thread_answer(
            "main-agent",
            &id,
            interaction.id,
            answer.clone(),
            "OTHER-USER",
            "en-US",
            &service
        )
        .await
        .is_err()
    );
    let resumed = host
        .dude_thread_answer(
            "main-agent",
            &id,
            interaction.id,
            answer,
            USER,
            "en-US",
            &service,
        )
        .await
        .unwrap();
    assert_eq!(
        resumed.latest_run_id,
        Some(first.result.as_ref().unwrap().run_id),
        "interaction answers resume the original Run"
    );
    sink.until(2).await;
    assert_eq!(
        sink.events.lock().unwrap()[1].status,
        fw::ThreadStatus::Completed
    );
    assert_eq!(
        script.questions.load(Ordering::Acquire),
        1,
        "native continuation must not replay the question tool"
    );
    assert!(
        host.dude_thread_send(
            "main-agent",
            &id,
            "human:followup",
            "OTHER-USER",
            "en-US",
            &service
        )
        .await
        .is_err()
    );
    let sent = host
        .dude_thread_send("main-agent", &id, "human:followup", USER, "en-US", &service)
        .await
        .unwrap();
    assert_ne!(
        sent.latest_run_id, resumed.latest_run_id,
        "new messages start another Run in the existing Thread"
    );
    sink.until(3).await;
    assert_eq!(host.flush_thread_feedback().await.unwrap(), 0);
    let events = sink.events.lock().unwrap().clone();
    assert_eq!(events.len(), 3);
    let keys: std::collections::HashSet<_> = events.iter().map(|event| &event.event_key).collect();
    assert_eq!(
        keys.len(),
        3,
        "AwaitingInput and Completed on the same Run each deliver exactly once"
    );
    assert!(events.iter().all(|event| event.owner.code == "main-agent"
        && event.owner.task_code.as_deref() == Some("dude-task")));
    let history = host
        .dude_thread_history("main-agent", &id, Default::default())
        .await
        .unwrap();
    let response = serde_json::to_value(&history.records.interactions[0]).unwrap();
    assert_eq!(response["responseProvenance"]["sender"]["kind"], "HUMAN");
    assert_eq!(response["responseProvenance"]["sender"]["id"], USER);
    let input = history
        .records
        .messages
        .iter()
        .find(|message| {
            message.content
                == vec![fw::HistoryContentBlock::Text {
                    text: "child:needs-human".into(),
                }]
        })
        .unwrap();
    assert_eq!(input.provenance.as_ref().unwrap().sender.kind, "DUDE");
    let human = history
        .records
        .messages
        .iter()
        .find(|message| {
            message.content
                == vec![fw::HistoryContentBlock::Text {
                    text: "human:followup".into(),
                }]
        })
        .unwrap();
    assert_eq!(human.provenance.as_ref().unwrap().sender.kind, "HUMAN");
    assert_eq!(
        human.provenance.as_ref().unwrap().sender.id.as_deref(),
        Some(USER)
    );
    assert!(
        history
            .records
            .messages
            .iter()
            .filter(|message| message.role == fw::Role::Assistant)
            .all(|message| message.provenance.as_ref().is_some_and(
                |p| p.sender.kind == "AGENT" && p.sender.id.as_deref() == Some("reviewer")
            ))
    );
    let groups = f
        .state
        .chats
        .list_session_groups(tron_backend::application::chat::ChatSessionQuery {
            workspace_code: Some(WORKSPACE.into()),
        })
        .unwrap();
    assert!(
        groups
            .iter()
            .flat_map(|group| &group.sessions)
            .all(|session| session.session_code != id
                && session.session_code != "original-dude-work-session"),
        "neither Dude parent nor native Thread appears in ordinary user Chats"
    );
    let catalog = host.dude_thread_catalog("main-agent").await.unwrap();
    assert_eq!(catalog.len(), 1);
    assert_eq!(catalog[0].task_code.as_deref(), Some("dude-task"));
    assert_eq!(catalog[0].agent_code, "reviewer");
    assert!(host.close().await.is_success());
}
