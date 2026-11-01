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
    session_id: Mutex<Option<String>>,
}

#[derive(Default)]
struct ChildAnswerScript {
    answered: AtomicBool,
}

#[derive(Default)]
struct ThreadAnswerScript {
    received: AtomicBool,
}

impl ModelClient for ThreadAnswerScript {
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
            let child = request
                .messages
                .iter()
                .any(|message| message.content.text() == "thread:ask-parent");
            let tool_messages: Vec<_> = request
                .messages
                .iter()
                .filter(|message| message.tool_result.is_some())
                .collect();
            if child {
                if tool_messages.is_empty() {
                    return Ok(to_gateway_response(ModelResponse::tool_call(
                        "ask_caller",
                        json!({"question":"Which target?"}),
                    )));
                }
                assert!(
                    serde_json::to_string(&request.messages)
                        .unwrap()
                        .contains("acceptance-target"),
                    "thread must receive the answer: {:?}",
                    request
                        .messages
                        .iter()
                        .filter(|message| message.role != fw::Role::System)
                        .collect::<Vec<_>>()
                );
                self.received.store(true, Ordering::Release);
                return Ok(ModelResponse::final_output("thread received answer"));
            }
            let results: Vec<Value> = tool_messages
                .iter()
                .map(|message| {
                    serde_json::from_str(&common::single_gateway_result(&message.content.text()))
                        .unwrap()
                })
                .collect();
            if results.is_empty() {
                return Ok(to_gateway_response(ModelResponse::tool_call(
                    "thread_create",
                    json!({"agentCode":"reviewer","task":"thread:ask-parent","title":"Interactive acceptance"}),
                )));
            }
            let session = results
                .iter()
                .find_map(|result| result["session_id"].as_str())
                .expect("thread ID");
            let latest = results.last().unwrap();
            if latest["output"] == "thread received answer" {
                return Ok(ModelResponse::final_output("Parent answered the thread"));
            }
            if let Some(interaction) = latest["interactions"]
                .as_array()
                .and_then(|items| items.first())
            {
                return Ok(to_gateway_response(ModelResponse::tool_call(
                    "thread_answer",
                    json!({"session_id":session,"interaction_id":interaction["id"],"answer":"acceptance-target"}),
                )));
            }
            Ok(to_gateway_response(ModelResponse::tool_call(
                "thread_wait",
                json!({"session_id":session}),
            )))
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
struct ChildSteeringScript {
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
    started: AtomicBool,
    received: AtomicBool,
}

impl ModelClient for ChildSteeringScript {
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
            let child = request
                .messages
                .iter()
                .any(|message| message.content.text() == "child:steering-test");
            if child {
                if request
                    .messages
                    .iter()
                    .any(|message| message.content.text().contains("steering: revised target"))
                {
                    self.received.store(true, Ordering::Release);
                    return Ok(ModelResponse::final_output("child received steering"));
                }
                self.started.store(true, Ordering::Release);
                self.entered.notify_one();
                self.release.notified().await;
                return Ok(ModelResponse::tool_call(
                    "tool_search",
                    json!({"items":[{"domainCode":"harness_workspace"}]}),
                ));
            }
            let results: Vec<Value> = request
                .messages
                .iter()
                .filter(|message| message.tool_result.is_some())
                .map(|message| {
                    serde_json::from_str(&common::single_gateway_result(&message.content.text()))
                        .unwrap()
                })
                .collect();
            if results.is_empty() {
                return Ok(to_gateway_response(ModelResponse::tool_call(
                    "agent_spawn",
                    json!({"agentCode":"reviewer","task":"child:steering-test","background":true}),
                )));
            }
            let child_id = results
                .iter()
                .find_map(|result| result["child_run_id"].as_str())
                .expect("child ID");
            if results.last().unwrap()["output"] == "child received steering" {
                return Ok(ModelResponse::final_output("Parent steered the child"));
            }
            if results.len() == 1 {
                if !self.started.load(Ordering::Acquire) {
                    self.entered.notified().await;
                }
                return Ok(to_gateway_response(ModelResponse::tool_call(
                    "child_send",
                    json!({"child_run_id":child_id,"message":"steering: revised target"}),
                )));
            }
            self.release.notify_one();
            Ok(to_gateway_response(ModelResponse::tool_call(
                "child_wait",
                json!({"child_run_id":child_id}),
            )))
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
impl ModelClient for ChildAnswerScript {
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
            let tool_messages: Vec<_> = request
                .messages
                .iter()
                .filter(|message| message.tool_result.is_some())
                .collect();
            if request
                .messages
                .iter()
                .any(|message| message.content.text() == "child:ask-dude")
            {
                return Ok(if tool_messages.is_empty() {
                    to_gateway_response(ModelResponse::tool_call(
                        "ask_caller",
                        json!({"question":"Which target?"}),
                    ))
                } else {
                    assert!(
                        tool_messages.iter().all(|message| !message
                            .tool_result
                            .as_ref()
                            .unwrap()
                            .is_error),
                        "caller interaction failed"
                    );
                    ModelResponse::final_output("child:answered")
                });
            }
            let results: Vec<Value> = tool_messages
                .iter()
                .map(|message| {
                    let text = common::single_gateway_result(&message.content.text());
                    serde_json::from_str(&text).unwrap_or_else(|error| {
                        panic!("invalid parent tool result {text}: {error}")
                    })
                })
                .collect();
            if results.is_empty() {
                return Ok(to_gateway_response(ModelResponse::tool_call(
                    "agent_spawn",
                    json!({"agentCode":"reviewer","task":"child:ask-dude","background":true}),
                )));
            }
            let child = results
                .iter()
                .find_map(|value| value["child_run_id"].as_str())
                .expect("child ID");
            let latest = results.last().unwrap();
            if latest["output"] == "child:answered" {
                return Ok(ModelResponse::final_output("Dude answered the child"));
            }
            if let Some(interaction) = latest["interactions"]
                .as_array()
                .and_then(|items| items.first())
            {
                self.answered.store(true, Ordering::Release);
                return Ok(to_gateway_response(ModelResponse::tool_call(
                    "child_answer",
                    json!({"child_run_id":child,"interaction_id":interaction["id"],"answer":"staging"}),
                )));
            }
            Ok(to_gateway_response(ModelResponse::tool_call(
                "child_wait",
                json!({"child_run_id":child}),
            )))
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
                .filter_map(|m| {
                    m.tool_result
                        .as_ref()
                        .map(|_| common::single_gateway_result(&m.content.text()))
                })
                .collect();
            if request
                .messages
                .iter()
                .any(|m| m.content.text() == "root:dude-native")
            {
                if results.is_empty() {
                    // Gateway mode: every wrapped tool travels as a tool_execute
                    // envelope so the model-visible schema stays the stable pair.
                    return Ok(ModelResponse::tool_call(
                        "tool_execute",
                        json!({
                            "tool":"thread_create",
                            "arguments":{"items":[{"agentCode":"reviewer","task":"child:needs-human","title":"Dude delegated work"}]}
                        }),
                    ));
                }
                let created: Value = serde_json::from_str(&results[0])
                    .map_err(|e| format!("{e}: {}", results[0]))?;
                let id = created["session_id"]
                    .as_str()
                    .ok_or_else(|| format!("thread did not start: {created}"))?;
                *self.session_id.lock().unwrap() = Some(id.into());
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
                let model_tools = request
                    .tools
                    .iter()
                    .map(|t| t.call_name.as_str())
                    .collect::<Vec<_>>();
                assert_eq!(
                    model_tools,
                    vec!["tool_execute", "tool_search"],
                    "gateway must expose only the stable pair"
                );
                return Ok(ModelResponse::tool_call(
                    "tool_execute",
                    json!({
                        "tool":"ask_caller",
                        "arguments":{"question":"Which deployment target?"}
                    }),
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
    run_dude_thread_intervention(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn platform_dude_without_workspace_delegates_and_resumes_a_thread() {
    run_dude_thread_intervention(true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn platform_dude_can_cancel_a_child_and_cannot_message_one_that_has_finished() {
    let f = Fixture::new();
    reviewer(&f);
    let host = f.state.agent_framework.clone();
    let agent = ReactAgentBuilder::new(
        Arc::new(Script::default()),
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    for (index, prompt, expected) in [
        (0, "root:detached-cancel", "cancel"),
        (1, "root:detached-steer", "is not running"),
    ] {
        let session = format!("platform-dude-child-{index}");
        f.state
            .chats
            .ensure_dude_session(
                &tron_backend::domain::dude::DudeSession {
                    session_code: session.clone(),
                    dude_code: "main-dude".into(),
                    purpose: tron_backend::domain::dude::DudeSessionPurpose::Task,
                    correlation_key: format!("platform-child-{index}"),
                    title: "Dude child test".into(),
                    workspace_code: None,
                    created_at: 0,
                    updated_at: 0,
                },
                Some(USER.into()),
            )
            .unwrap();
        let mut parent = context(prompt);
        parent.workspace_code.clear();
        parent.agent_code = Some("main-dude".into());
        parent.conversation_code = Some(session.clone());
        parent.collaboration_owner = Some(AgentCollaborationOwner {
            kind: "DUDE".into(),
            code: "main-dude".into(),
            origin_session_code: Some(session),
            task_code: None,
        });
        let result = tokio::time::timeout(Duration::from_secs(60), agent.run(parent))
            .await
            .unwrap()
            .unwrap();
        assert!(
            result.final_output.to_lowercase().contains(expected),
            "{}",
            result.final_output
        );
    }
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn platform_dude_answers_a_background_child_without_a_workspace() {
    let f = Fixture::new();
    reviewer(&f);
    let host = f.state.agent_framework.clone();
    let script = Arc::new(ChildAnswerScript::default());
    let agent = ReactAgentBuilder::new(
        script.clone(),
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let session = "platform-dude-answer-child";
    f.state
        .chats
        .ensure_dude_session(
            &tron_backend::domain::dude::DudeSession {
                session_code: session.into(),
                dude_code: "main-dude".into(),
                purpose: tron_backend::domain::dude::DudeSessionPurpose::Task,
                correlation_key: "answer-child".into(),
                title: "Dude child answer".into(),
                workspace_code: None,
                created_at: 0,
                updated_at: 0,
            },
            Some(USER.into()),
        )
        .unwrap();
    let mut parent = context("root:dude-answer");
    parent.workspace_code.clear();
    parent.agent_code = Some("main-dude".into());
    parent.conversation_code = Some(session.into());
    parent.collaboration_owner = Some(AgentCollaborationOwner {
        kind: "DUDE".into(),
        code: "main-dude".into(),
        origin_session_code: Some(session.into()),
        task_code: None,
    });
    let result = tokio::time::timeout(Duration::from_secs(60), agent.run(parent))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.final_output, "Dude answered the child");
    assert!(script.answered.load(Ordering::Acquire));
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ordinary_agent_answers_a_delegated_thread() {
    run_thread_answer(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn platform_dude_answers_a_delegated_thread_without_a_workspace() {
    run_thread_answer(true).await;
}

async fn run_thread_answer(dude: bool) {
    let f = Fixture::new();
    reviewer(&f);
    let host = f.state.agent_framework.clone();
    let script = Arc::new(ThreadAnswerScript::default());
    let agent = ReactAgentBuilder::new(
        script.clone(),
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let mut parent = context("root:thread-answer");
    if dude {
        let session = "platform-dude-thread-answer";
        f.state
            .chats
            .ensure_dude_session(
                &tron_backend::domain::dude::DudeSession {
                    session_code: session.into(),
                    dude_code: "main-dude".into(),
                    purpose: tron_backend::domain::dude::DudeSessionPurpose::Task,
                    correlation_key: "thread-answer".into(),
                    title: "Dude thread answer".into(),
                    workspace_code: None,
                    created_at: 0,
                    updated_at: 0,
                },
                Some(USER.into()),
            )
            .unwrap();
        parent.workspace_code.clear();
        parent.agent_code = Some("main-dude".into());
        parent.conversation_code = Some(session.into());
        parent.collaboration_owner = Some(AgentCollaborationOwner {
            kind: "DUDE".into(),
            code: "main-dude".into(),
            origin_session_code: Some(session.into()),
            task_code: None,
        });
    }
    let result = tokio::time::timeout(Duration::from_secs(60), agent.run(parent))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.final_output, "Parent answered the thread");
    assert!(script.received.load(Ordering::Acquire));
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ordinary_agent_steers_a_running_child() {
    let f = Fixture::new();
    reviewer(&f);
    let host = f.state.agent_framework.clone();
    let script = Arc::new(ChildSteeringScript::default());
    let agent = ReactAgentBuilder::new(
        script.clone(),
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let result = tokio::time::timeout(
        Duration::from_secs(60),
        agent.run(context("root:ordinary-steering")),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(result.final_output, "Parent steered the child");
    assert!(
        script.received.load(Ordering::Acquire),
        "child must receive the steering message"
    );
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn platform_dude_steers_a_running_child_without_a_workspace() {
    let f = Fixture::new();
    reviewer(&f);
    let host = f.state.agent_framework.clone();
    let script = Arc::new(ChildSteeringScript::default());
    let agent = ReactAgentBuilder::new(
        script.clone(),
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let session = "platform-dude-steer-child";
    f.state
        .chats
        .ensure_dude_session(
            &tron_backend::domain::dude::DudeSession {
                session_code: session.into(),
                dude_code: "main-dude".into(),
                purpose: tron_backend::domain::dude::DudeSessionPurpose::Task,
                correlation_key: "steer-child".into(),
                title: "Dude child steering".into(),
                workspace_code: None,
                created_at: 0,
                updated_at: 0,
            },
            Some(USER.into()),
        )
        .unwrap();
    let mut parent = context("root:dude-steering");
    parent.workspace_code.clear();
    parent.agent_code = Some("main-dude".into());
    parent.conversation_code = Some(session.into());
    parent.collaboration_owner = Some(AgentCollaborationOwner {
        kind: "DUDE".into(),
        code: "main-dude".into(),
        origin_session_code: Some(session.into()),
        task_code: None,
    });
    let result = tokio::time::timeout(Duration::from_secs(60), agent.run(parent))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.final_output, "Parent steered the child");
    assert!(
        script.received.load(Ordering::Acquire),
        "child must receive the steering message"
    );
    assert!(host.close().await.is_success());
}

async fn run_dude_thread_intervention(platform_scope: bool) {
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
                dude_code: "main-dude".into(),
                purpose: tron_backend::domain::dude::DudeSessionPurpose::Task,
                correlation_key: "dude-task".into(),
                title: "Dude work".into(),
                workspace_code: if platform_scope {
                    None
                } else {
                    Some(WORKSPACE.into())
                },
                created_at: 0,
                updated_at: 0,
            },
            Some(USER.into()),
        )
        .unwrap();
    let mut parent = context("root:dude-native");
    if platform_scope {
        parent.workspace_code.clear();
    }
    parent.agent_code = Some("main-dude".into());
    parent.conversation_code = Some("original-dude-work-session".into());
    parent.collaboration_owner = Some(AgentCollaborationOwner {
        kind: "DUDE".into(),
        code: "main-dude".into(),
        origin_session_code: Some("original-dude-work-session".into()),
        task_code: Some("dude-task".into()),
    });
    let parent = tokio::time::timeout(Duration::from_secs(30), agent.run(parent))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(parent.final_output, "parent-finished-before-child");
    let id = script.session_id.lock().unwrap().clone().unwrap();
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
    assert_eq!(first.session_id, id);
    let native = host
        .storage_framework()
        .await
        .unwrap()
        .sessions()
        .unwrap()
        .get(&id)
        .await
        .unwrap();
    assert_eq!(
        native.id, id,
        "delegation feedback addresses the same native session"
    );
    assert_eq!(first.owner.code, "main-dude");
    assert_eq!(first.owner.task_code.as_deref(), Some("dude-task"));
    assert_eq!(first.origin_session_code, "original-dude-work-session");
    let interaction = &first.result.as_ref().unwrap().interactions[0];
    assert_eq!(
        interaction.presentation.as_ref().unwrap()["kind"],
        "ask_caller"
    );
    let answer = json!("Staging");
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
            "main-dude",
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
            "main-dude",
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
            "main-dude",
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
        .dude_thread_send("main-dude", &id, "human:followup", USER, "en-US", &service)
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
    assert!(events.iter().all(|event| event.owner.code == "main-dude"
        && event.owner.task_code.as_deref() == Some("dude-task")));
    let history = host
        .dude_thread_history("main-dude", &id, Default::default())
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
    let catalog = host.dude_thread_catalog("main-dude").await.unwrap();
    assert_eq!(catalog.len(), 1);
    assert_eq!(catalog[0].task_code.as_deref(), Some("dude-task"));
    assert_eq!(catalog[0].agent_code, "reviewer");
    assert!(host.close().await.is_success());
    let telemetry = rusqlite::Connection::open(
        std::path::Path::new(&f.state.config.database_url).with_extension("telemetry.sqlite3"),
    )
    .unwrap();
    let spans = telemetry
        .prepare("SELECT data FROM ef_telemetry_spans")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .map(|row| serde_json::from_str::<fw::TelemetrySpan>(&row.unwrap()).unwrap())
        .collect::<Vec<_>>();
    let create = spans
        .iter()
        .find(|span| {
            span.name == "agent.tool"
                && span
                    .attributes
                    .get("gen_ai.tool.name")
                    .is_some_and(|name| name == "thread_create")
        })
        .expect("real Dude dispatch produces a tool span");
    let thread_run = spans
        .iter()
        .find(|span| {
            span.name == "agent.run"
                && span.trace_id == create.trace_id
                && span.parent_span_id.as_deref() == Some(&create.span_id)
        })
        .expect("detached Thread inherits the actual dispatching tool span");
    assert!(
        spans
            .iter()
            .any(|span| span.name == "agent.model" && span.trace_id == thread_run.trace_id),
        "Thread reasoning remains queryable in the same persisted trace after parent exit"
    );
}
