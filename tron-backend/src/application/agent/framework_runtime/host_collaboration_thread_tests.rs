use super::*;
use crate::domain::agent::{AgentCollaborationOwner, AgentExecutionScope};
use std::sync::atomic::{AtomicBool, Ordering};

fn record(id: &str) -> ThreadRecord {
    ThreadRecord {
        summary: fw::ThreadSummary {
            thread_id: id.into(),
            title: Some("Delegated work".into()),
            status: fw::ThreadStatus::Failed,
            latest_run_id: Some(fw::RunId::new_v4()),
        },
        workspace_code: String::new(),
        user_code: Some("profile".into()),
        agent_code: "coder".into(),
        owner: AgentCollaborationOwner {
            kind: "DUDE".into(),
            code: "main-dude".into(),
            origin_session_code: Some("dude-work-session".into()),
            task_code: Some("task-1".into()),
        },
        initiator_agent_code: Some("main-dude".into()),
        origin_session_code: "dude-work-session".into(),
        execution_scope: AgentExecutionScope::default(),
        completion: Some((fw::ThreadStatus::Failed, Err("needs attention".into()))),
        closed: false,
        feedback_pending: true,
        revision: 0,
        run_admitted_at_ms: fw::lifecycle_time_ms(),
        execution_sequence: 0,
        resumed_interaction_id: None,
    }
}
struct Sink {
    fail: AtomicBool,
    events: std::sync::Mutex<Vec<ThreadFeedback>>,
}
#[async_trait::async_trait]
impl ThreadFeedbackSink for Sink {
    async fn publish(&self, feedback: ThreadFeedback) -> Result<(), String> {
        if self.fail.swap(false, Ordering::AcqRel) {
            return Err("transient outage".into());
        }
        self.events.lock().unwrap().push(feedback);
        Ok(())
    }
}
#[tokio::test]
async fn durable_thread_outbox_survives_host_and_parent_exit_and_retries_once() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("threads.sqlite3");
    let host = FrameworkHost::persistent(&path, None);
    let mut first = record("thread-1");
    host.persist_thread(&mut first, true).await.unwrap();
    // A second run must not replace undelivered feedback from the first run.
    first.execution_sequence += 1;
    host.persist_thread(&mut first, false).await.unwrap();
    drop(host);
    let host = FrameworkHost::persistent(&path, None);
    let sink = Arc::new(Sink {
        fail: AtomicBool::new(true),
        events: Default::default(),
    });
    host.set_thread_feedback_sink(sink.clone()).unwrap();
    assert!(host.flush_thread_feedback().await.is_err());
    assert_eq!(host.flush_thread_feedback().await.unwrap(), 2);
    assert_eq!(host.flush_thread_feedback().await.unwrap(), 0);
    let events = sink.events.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_ne!(events[0].event_key, events[1].event_key);
    assert!(events.iter().all(|event| event.owner.code == "main-dude"
        && event.origin_session_code == "dude-work-session"
        && event.owner.task_code.as_deref() == Some("task-1")));
}
#[tokio::test]
async fn stale_thread_writer_cannot_overwrite_latest_state_and_close_keeps_history_binding() {
    let host = FrameworkHost::temporary(None).unwrap();
    let mut first = record("thread-1");
    first.feedback_pending = false;
    host.persist_thread(&mut first, true).await.unwrap();
    let mut stale = first.clone();
    first.closed = true;
    host.persist_thread(&mut first, false).await.unwrap();
    stale.summary.title = Some("stale".into());
    assert!(host.persist_thread(&mut stale, false).await.is_err());
    let stored = host.thread_record("thread-1").await.unwrap().unwrap();
    assert!(stored.closed);
    assert_eq!(stored.agent_code, "coder");
    assert_eq!(host.dude_threads("other-dude").await.unwrap().len(), 0);
    assert_eq!(host.dude_threads("main-dude").await.unwrap().len(), 1);
    assert!(host.dude_thread_catalog("main-dude").await.unwrap()[0].closed);
    let json = serde_json::to_string(&stored).unwrap();
    assert!(!json.contains("apiKey"));
    assert!(!json.contains("variables"));
}

#[test]
fn thread_access_requires_root_owner_and_scope_but_allows_other_sessions_of_same_dude() {
    let record = record("thread-1");
    let mut caller = record.owner.clone();
    caller.origin_session_code = Some("another-work-session".into());
    assert!(record.authorized_owner("", Some("profile"), &caller));
    caller.code = "other-dude".into();
    assert!(!record.authorized_owner("", Some("profile"), &caller));
    caller = record.owner.clone();
    caller.kind = "USER".into();
    assert!(!record.authorized_owner("", Some("profile"), &caller));
    assert!(!record.authorized_owner("workspace", Some("profile"), &record.owner));
    assert!(!record.authorized_owner("", Some("other-profile"), &record.owner));
}

struct CompletionModel(Arc<std::sync::atomic::AtomicUsize>);
#[async_trait::async_trait]
impl fw::ChatModel for CompletionModel {
    async fn generate(
        &self,
        _: &str,
        _: fw::ChatRequest,
        _: &dyn fw::ChatStream,
    ) -> Result<fw::ChatResponse, fw::ModelError> {
        self.0.fetch_add(1, Ordering::AcqRel);
        Ok(fw::ChatResponse {
            content: fw::Content::text("completed delegated work"),
            ..Default::default()
        })
    }
}
async fn recovery_agent(host: &FrameworkHost, calls: Arc<std::sync::atomic::AtomicUsize>) -> Agent {
    storage_agent(host, Arc::new(CompletionModel(calls)), vec![]).await
}
async fn storage_agent(
    host: &FrameworkHost,
    model: Arc<dyn fw::ChatModel>,
    tools: Vec<fw::ToolBinding>,
) -> Agent {
    let artifacts = host.0.path.with_extension("artifacts");
    std::fs::create_dir_all(&artifacts).unwrap();
    let agent = eliteforge_agent_test_support::TestBuilder::new()
        .default_models(false)
        .plugin(eliteforge_agent_framework::ModelRuntimePlugin::new(
            eliteforge_agent_framework::ModelRuntime::new(
                eliteforge_agent_framework::ModelRuntimeUpdate::new("", 0, vec![]),
            )
            .unwrap(),
        ))
        .plugin(fw::ModelConnectionPlugin::new(fw::ModelConnection::new(
            "fixture",
            model,
            vec![{
                let mut descriptor = fw::ModelDescriptor::new("complete");
                descriptor.supports_tools = Some(true);
                descriptor
            }],
        )))
        .plugin(fw::ToolProviderPlugin::new(
            "fixture.tools",
            fw::StaticToolProvider::new(tools),
        ))
        .plugin(
            eliteforge_agent_official_plugin_storage_local::LocalStoragePlugin::new(
                &host.0.path,
                artifacts,
            )
            .with_database(host.shared_database().await.unwrap()),
        )
        .build()
        .await
        .unwrap();
    host.0
        .framework
        .get_or_try_init(|| async { Ok::<_, fw::AgentError>(agent.framework()) })
        .await
        .unwrap();
    agent
}
#[tokio::test]
async fn restart_reconciles_real_native_run_completion_and_does_not_reexecute_model() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("recovery.sqlite3");
    let host = FrameworkHost::persistent(&path, None);
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let agent = recovery_agent(&host, calls.clone()).await;
    let mut thread = record("thread-native");
    thread.summary.status = fw::ThreadStatus::Running;
    thread.completion = None;
    thread.feedback_pending = false;
    host.persist_thread(&mut thread, true).await.unwrap();
    let mut request = fw::RunRequest::new("thread-native", "do work");
    request.run_id = thread.summary.latest_run_id;
    request.model = Some(fw::ModelRef::new("fixture", "complete"));
    let result = agent.run(request).await.unwrap();
    assert_eq!(result.output, "completed delegated work");
    // Simulate exit after the native result commits but before the host callback writes.
    drop(agent);
    drop(host);
    let host = FrameworkHost::persistent(&path, None);
    let _agent = recovery_agent(&host, calls.clone()).await;
    let sink = Arc::new(Sink {
        fail: AtomicBool::new(false),
        events: Default::default(),
    });
    host.set_thread_feedback_sink(sink.clone()).unwrap();
    assert_eq!(host.recover_collaboration_threads().await.unwrap(), 1);
    assert_eq!(host.recover_collaboration_threads().await.unwrap(), 0);
    assert_eq!(calls.load(Ordering::Acquire), 1);
    let stored = host.thread_record("thread-native").await.unwrap().unwrap();
    assert_eq!(stored.summary.status, fw::ThreadStatus::Completed);
    let events = sink.events.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].result.as_ref().unwrap().output,
        "completed delegated work"
    );
}

#[tokio::test]
async fn recovery_reports_abandoned_admission_without_replaying_external_actions() {
    let host = FrameworkHost::temporary(None).unwrap();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let _agent = recovery_agent(&host, calls.clone()).await;
    let mut thread = record("thread-abandoned");
    thread.summary.status = fw::ThreadStatus::Running;
    thread.completion = None;
    thread.feedback_pending = false;
    thread.run_admitted_at_ms = fw::lifecycle_time_ms().saturating_sub(31_000);
    host.persist_thread(&mut thread, true).await.unwrap();
    let sink = Arc::new(Sink {
        fail: AtomicBool::new(false),
        events: Default::default(),
    });
    host.set_thread_feedback_sink(sink.clone()).unwrap();
    assert_eq!(host.recover_collaboration_threads().await.unwrap(), 1);
    assert_eq!(calls.load(Ordering::Acquire), 0);
    let events = sink.events.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].status, fw::ThreadStatus::Failed);
    assert!(
        events[0]
            .error
            .as_deref()
            .unwrap()
            .contains("did not start")
    );
}

struct AskModel;
#[async_trait::async_trait]
impl fw::ChatModel for AskModel {
    async fn generate(
        &self,
        _: &str,
        request: fw::ChatRequest,
        _: &dyn fw::ChatStream,
    ) -> Result<fw::ChatResponse, fw::ModelError> {
        if request
            .messages
            .iter()
            .any(|message| message.role == fw::Role::Tool)
        {
            return Ok(fw::ChatResponse::text("answered"));
        }
        Ok(fw::ChatResponse::tool_calls(vec![fw::ToolCall::new(
            "ask",
            "ask",
            serde_json::json!({}),
        )]))
    }
}
struct AskTool;
#[async_trait::async_trait]
impl fw::Tool for AskTool {
    async fn call(
        &self,
        _: serde_json::Value,
        _: &fw::RunContext,
    ) -> Result<fw::ToolOutput, fw::ToolError> {
        Ok(fw::ToolOutput::text("choose").request_input(
            fw::InteractionRequest::input("choose", serde_json::json!({"type":"boolean"})).unwrap(),
        ))
    }
}
#[tokio::test]
async fn continuation_recovery_uses_answered_interaction_identity_when_timestamps_are_equal() {
    let host = FrameworkHost::temporary(None).unwrap();
    let agent = storage_agent(
        &host,
        Arc::new(AskModel),
        vec![fw::ToolBinding::new(
            fw::ToolDescriptor::new("ask", "ask", "choose", serde_json::json!({"type":"object"})),
            AskTool,
        )],
    )
    .await;
    let mut thread = record("same-millisecond");
    let mut request = fw::RunRequest::new("same-millisecond", "start");
    request.run_id = thread.summary.latest_run_id;
    request.model = Some(fw::ModelRef::new("fixture", "complete"));
    let awaiting = agent.run(request).await.unwrap();
    assert_eq!(awaiting.status, fw::RunStatus::AwaitingInput);
    let interaction = awaiting.interactions[0].id;
    let run_id = thread.summary.latest_run_id.unwrap();
    let native = host
        .storage_framework()
        .await
        .unwrap()
        .sessions()
        .unwrap()
        .store
        .get_run(run_id)
        .await
        .unwrap();
    thread.summary.status = fw::ThreadStatus::Running;
    thread.completion = None;
    thread.feedback_pending = false;
    thread.execution_sequence = 2;
    thread.resumed_interaction_id = Some(interaction);
    // Deterministically reproduce an answer admitted in the same millisecond as the previous result.
    thread.run_admitted_at_ms = native.finished_at_ms.unwrap();
    host.persist_thread(&mut thread, true).await.unwrap();
    let sink = Arc::new(Sink {
        fail: AtomicBool::new(false),
        events: Default::default(),
    });
    host.set_thread_feedback_sink(sink.clone()).unwrap();
    assert_eq!(
        host.recover_collaboration_threads().await.unwrap(),
        0,
        "old waiting result is not the admitted continuation result"
    );
    assert!(sink.events.lock().unwrap().is_empty());
    let mut resume = agent
        .answer("same-millisecond", interaction, serde_json::json!(true))
        .await
        .unwrap();
    resume.model = Some(fw::ModelRef::new("fixture", "complete"));
    assert_eq!(
        agent.run(resume).await.unwrap().status,
        fw::RunStatus::Completed
    );
    assert_eq!(host.recover_collaboration_threads().await.unwrap(), 1);
    assert_eq!(host.recover_collaboration_threads().await.unwrap(), 0);
    let events = sink.events.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].status, fw::ThreadStatus::Completed);
}

fn memory_context(record: &ThreadRecord) -> AgentRunContext {
    serde_json::from_value(serde_json::json!({
        "agentCode":record.agent_code,"runCode":fw::RunId::new_v4().to_string(),"conversationCode":record.summary.thread_id,"workspaceCode":record.workspace_code,"userCode":record.user_code,"collaborationOwner":record.owner,"locale":"en-US","agentKind":"REACT",
        "model":{"modelCode":"fixture","protocol":"CHAT_COMPLETIONS","baseUrl":"http://127.0.0.1:1/v1","apiKey":"synthetic","modelName":"fixture","maxTokens":2048,"contextWindowTokens":32000},
        "variables":[],"skills":[],"tools":[],"executionScope":record.execution_scope,"limits":{"maxSteps":12,"maxToolCalls":12,"maxParallelToolCalls":1},"messages":[],"observations":[],"startedAt":"fixture",
        "shortTermMemory":{"sessionCode":record.summary.thread_id,"inputCode":"input","inputKind":"USER_PROMPT","promptPrefix":[],"todoEnabled":false}
    })).unwrap()
}
// Exercise synchronous STM adapters on the same runtime flavor as production native execution.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn native_thread_stm_scope_and_family_come_from_durable_owner_without_user_chat_link() {
    use crate::application::short_term_memory::{ShortTermMemoryConfig, ShortTermMemoryService};
    let host = FrameworkHost::temporary(None).unwrap();
    let agent = recovery_agent(&host, Arc::new(std::sync::atomic::AtomicUsize::new(0))).await;
    let sessions = host.storage_framework().await.unwrap().sessions().unwrap();
    let mut current = record("memory-current");
    current.feedback_pending = false;
    host.persist_thread(&mut current, true).await.unwrap();
    for (id, owner, user, workspace, origin, task, summary) in [
        (
            "memory-current",
            "main-dude",
            "profile",
            "",
            "dude-work-session",
            "task-1",
            "current",
        ),
        (
            "memory-sibling",
            "main-dude",
            "profile",
            "",
            "dude-work-session",
            "task-1",
            "authorized-family",
        ),
        (
            "memory-other-dude",
            "other-dude",
            "profile",
            "",
            "dude-work-session",
            "task-1",
            "other-dude-secret",
        ),
        (
            "memory-other-user",
            "main-dude",
            "other-profile",
            "",
            "dude-work-session",
            "task-1",
            "other-user-secret",
        ),
        (
            "memory-other-workspace",
            "main-dude",
            "profile",
            "other-workspace",
            "dude-work-session",
            "task-1",
            "other-workspace-secret",
        ),
        (
            "memory-unrelated",
            "main-dude",
            "profile",
            "",
            "other-origin",
            "other-task",
            "unrelated-secret",
        ),
    ] {
        if id != "memory-current" {
            let mut related = record(id);
            related.owner.code = owner.into();
            related.user_code = Some(user.into());
            related.workspace_code = workspace.into();
            related.origin_session_code = origin.into();
            related.owner.task_code = Some(task.into());
            related.feedback_pending = false;
            host.persist_thread(&mut related, true).await.unwrap();
        }
        let mut request = fw::RunRequest::new(id, "seed native history");
        request.model = Some(fw::ModelRef::new("fixture", "complete"));
        agent.run(request).await.unwrap();
        sessions
            .write_context(
                id,
                "tron.session-summary-view",
                None,
                serde_json::json!({"summary":summary}),
            )
            .await
            .unwrap();
    }
    let service = ShortTermMemoryService::with_host(host.clone(), ShortTermMemoryConfig::default());
    let context = memory_context(&current);
    let prepared = service
        .prepare_model_context_with_tool_schema_estimate(&context, 0)
        .unwrap();
    assert_eq!(
        prepared.sources.unwrap().revision.scope,
        context.memory_scope("memory-current").unwrap()
    );
    let family = service
        .framework_data_source(&context, "session_family_context")
        .unwrap()
        .0;
    assert_eq!(family.len(), 1);
    assert!(family[0].text_content().contains("authorized-family"));
    for change in ["owner", "user", "workspace"] {
        let mut forged = context.clone();
        match change {
            "owner" => forged.collaboration_owner.as_mut().unwrap().code = "other-dude".into(),
            "user" => forged.user_code = Some("other-profile".into()),
            _ => forged.workspace_code = "other-workspace".into(),
        }
        assert!(
            service
                .framework_data_source(&forged, "session_family_context")
                .is_err(),
            "caller cannot forge {change} to rebind native Thread STM"
        );
    }
    let count = host
        .shared_database()
        .await
        .unwrap()
        .call(|connection| {
            connection.query_row(
                "SELECT count(*) FROM workspace_session_links WHERE session_id LIKE 'memory-%'",
                [],
                |row| row.get::<_, usize>(0),
            )
        })
        .await
        .unwrap();
    assert_eq!(
        count, 0,
        "native Thread STM never fabricates a product/user Chat link"
    );
}
