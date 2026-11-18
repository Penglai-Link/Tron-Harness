use crate::{
    app::AppState,
    application::{
        agent::AgentInvocationService,
        assistant::{AssistantExecutionOutcome, AssistantExecutor},
    },
    domain::{agent::AgentCollaborationOwner, assistant::*},
};
use std::sync::Arc;
use tron_i18n::t;

fn memory_execution_error(
    error: crate::domain::assistant_memory::AssistantMemoryError,
) -> AssistantError {
    match error {
        crate::domain::assistant_memory::AssistantMemoryError::ExecutionBusy => {
            AssistantError::ExecutionBusy
        }
        error => AssistantError::Runtime(error.to_string()),
    }
}

pub(super) struct PlatformAssistantExecutor {
    pub state: AppState,
}
impl PlatformAssistantExecutor {
    fn agent(&self) -> AgentInvocationService {
        self.state.assistant_invocation_service()
    }
}
#[async_trait::async_trait]
impl AssistantExecutor for PlatformAssistantExecutor {
    fn is_ready(&self) -> Result<bool, AssistantError> {
        self.state
            .profiles
            .active()
            .map(|profile| profile.is_some())
            .map_err(|error| AssistantError::Runtime(error.to_string()))
    }

    async fn recover(
        &self,
        event: &AssistantEvent,
    ) -> Result<Option<AssistantExecutionOutcome>, AssistantError> {
        let framework = self
            .state
            .agent_framework
            .storage_framework()
            .await
            .map_err(|error| AssistantError::Runtime(error.to_string()))?;
        let store = framework
            .sessions()
            .map_err(|error| AssistantError::Runtime(error.to_string()))?
            .store;
        let run_id = eliteforge_agent_core::RunId::parse_str(&event.event_id)
            .map_err(|_| AssistantError::Invalid)?;
        match store.get_run(run_id).await {
            Ok(run) => {
                if let Some(result) = run.result {
                    if matches!(
                        result.status,
                        eliteforge_agent_core::RunStatus::Completed
                            | eliteforge_agent_core::RunStatus::AwaitingInput
                    ) {
                        return Ok(Some(AssistantExecutionOutcome {
                            summary: result.output,
                            awaiting_input: result.status
                                == eliteforge_agent_core::RunStatus::AwaitingInput,
                        }));
                    }
                }
                Err(AssistantError::Runtime("PersonAI execution was interrupted. Inspect its tools and external results before continuing the task.".into()))
            }
            Err(eliteforge_agent_core::StateError::NotFound) => Ok(None),
            Err(error) => Err(AssistantError::Runtime(error.to_string())),
        }
    }
    async fn execute(
        &self,
        event: AssistantEvent,
        session: AssistantSession,
    ) -> Result<AssistantExecutionOutcome, AssistantError> {
        use opentelemetry::{
            Context, KeyValue,
            trace::{FutureExt, Status, TraceContextExt},
        };
        use tracing::Instrument;

        let telemetry = self
            .state
            .agent_framework
            .telemetry()
            .await
            .map_err(|error| {
                AssistantError::Runtime(format!("PersonAI tracing unavailable: {error:?}"))
            })?;
        // A durable wakeup is a new operation, even if dispatch was initiated by
        // an HTTP request or another PersonAI. Never inherit the worker's context.
        let guard = telemetry.begin_operation(
            "tron.assistant.execute",
            &Context::new(),
            vec![
                KeyValue::new("tron.assistant.code", event.assistant_code.clone()),
                KeyValue::new("tron.assistant.event.id", event.event_id.clone()),
                KeyValue::new("tron.assistant.event.source", format!("{:?}", event.source)),
                KeyValue::new("tron.assistant.event.attempt", i64::from(event.attempts)),
                KeyValue::new("tron.session.code", session.session_code.clone()),
                KeyValue::new("tron.agent.code", event.assistant_code.clone()),
                KeyValue::new(
                    "tron.workspace.code",
                    session.workspace_code.clone().unwrap_or_default(),
                ),
                KeyValue::new(
                    "tron.task.code",
                    event.task_code.clone().unwrap_or_default(),
                ),
            ],
        );
        let context = guard.0.clone();
        let ids = context.span().span_context().clone();
        let log_span = tracing::info_span!("assistant.execute", trace_id=%ids.trace_id(), span_id=%ids.span_id(),
            assistant_code=%event.assistant_code, event_id=%event.event_id);
        let result = async {
            tracing::info!("PersonAI execution started");
            let result = self.execute_inner(event, session).await;
            match &result {
                Err(AssistantError::ExecutionBusy) => tracing::info!("PersonAI execution deferred"),
                Err(error) => {
                    crate::app::telemetry::record_error(&context, "assistant", error);
                    context
                        .span()
                        .set_status(Status::error("PersonAI execution failed"));
                    tracing::error!("PersonAI execution failed");
                }
                Ok(_) => tracing::info!("PersonAI execution finished"),
            }
            result
        }
        .instrument(log_span)
        .with_context(context.clone())
        .await;
        guard.complete(match &result {
            Ok(outcome) if outcome.awaiting_input => "waiting",
            Ok(_) => "completed",
            Err(AssistantError::ExecutionBusy) => "deferred",
            Err(_) => "error",
        });
        result
    }
}

impl PlatformAssistantExecutor {
    async fn execute_inner(
        &self,
        event: AssistantEvent,
        session: AssistantSession,
    ) -> Result<AssistantExecutionOutcome, AssistantError> {
        let assistant = self.state.assistants.get(&event.assistant_code)?;
        if assistant.kind == AssistantKind::Memory
            && event.payload.get("businessNotice").is_none()
            && (event.payload["resumeMemoryCandidates"] == true
                || matches!(
                    event.source,
                    AssistantEventSource::Heartbeat | AssistantEventSource::MemoryBatch
                ))
        {
            let profile = self
                .state
                .profiles
                .current_context()
                .map_err(|e| AssistantError::Runtime(e.to_string()))?;
            let user = profile.person_ai_user_id;
            let router = crate::application::model_router::AgentModelRouter::new(
                self.state.models.clone(),
                self.state.model_providers.clone(),
            )
            .with_agent_management(self.state.agent_management.clone());
            let runner = crate::application::assistant_memory::AssistantMemoryRunner::new(
                self.state.assistant_memory.clone(),
                self.state.memory.clone(),
                self.state.agent_framework.clone(),
                self.agent(),
                router,
                Arc::new(MemorySessionFactory {
                    assistants: self.state.assistants.clone(),
                    chats: self.state.chats.clone(),
                }),
            );
            runner
                .resume_candidates(&event.assistant_code, &user, assistant.memory_auto_approve)
                .await
                .map_err(memory_execution_error)?;
            let Some(batch) = runner
                .prepare_wakeup(&event.assistant_code, &user)
                .await
                .map_err(|e| AssistantError::Runtime(e.to_string()))?
            else {
                return Ok(AssistantExecutionOutcome {
                    summary: String::new(),
                    awaiting_input: false,
                });
            };
            let batch_session = self.state.assistants.resolve_session(
                &event.assistant_code,
                AssistantSessionPurpose::MemoryBatch,
                &batch.batch_code,
                None,
            )?;
            self.state
                .assistants
                .attach_session(&event, &batch_session.session_code)?;
            self.state
                .chats
                .ensure_assistant_session(&batch_session, Some(user.clone()))
                .map_err(|_| AssistantError::Storage)?;
            let batch = runner
                .execute_batch(
                    &user,
                    &batch.batch_code,
                    &batch_session.session_code,
                    assistant.memory_auto_approve,
                )
                .await
                .map_err(memory_execution_error)?;
            return Ok(AssistantExecutionOutcome {
                summary: batch.summary.unwrap_or_default(),
                awaiting_input: false,
            });
        }
        if event.source == AssistantEventSource::TaskReminder {
            if let Some(code) = &event.task_code {
                let version = event.payload["scheduleVersion"].as_i64().unwrap_or(0);
                if !self
                    .state
                    .tasks
                    .assistant_task_reminder_is_current(code, version)
                    .map_err(|_| AssistantError::Storage)?
                {
                    // The event is persisted with its result summary and shown in the
                    // PersonAI activity list; use the locale the run would use.
                    let locale = crate::shared::locale::SupportedLocale::from_raw(
                        event.payload["locale"].as_str().unwrap_or(""),
                    )
                    .as_str();
                    return Ok(AssistantExecutionOutcome {
                        summary: t!(
                            "agent_runtime.assistant.stale_task_reminder_skipped",
                            locale = locale
                        )
                        .into_owned(),
                        awaiting_input: false,
                    });
                }
                self.state
                    .tasks
                    .bind_assistant_work_session(code, &session.session_code)
                    .map_err(|_| AssistantError::Storage)?;
            }
        }
        let run_user = self
            .state
            .profiles
            .current_context()
            .map_err(|e| AssistantError::Runtime(e.to_string()))?
            .person_ai_user_id;
        let owner = self
            .state
            .chats
            .get_session(&session.session_code)
            .ok()
            .and_then(|s| s.user_code)
            .or_else(|| {
                event
                    .payload
                    .get("userCode")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
            });
        if owner.as_ref().is_some_and(|owner| owner != &run_user) {
            return Err(AssistantError::ExecutionBusy);
        }
        self.state
            .chats
            .ensure_assistant_session(&session, Some(run_user.clone()))
            .map_err(|_| AssistantError::Storage)?;
        let router = crate::application::model_router::AgentModelRouter::new(
            self.state.models.clone(),
            self.state.model_providers.clone(),
        )
        .with_agent_management(self.state.agent_management.clone());
        let (model, route) = router
            .resolve_initial("", &session.session_code, &event.assistant_code, None)
            .map_err(AssistantError::Runtime)?;
        // The transcript records the input, while live event data and fixed
        // operating instructions belong to their configured contributors.
        let prompt = if event.source == AssistantEventSource::UserMessage {
            event.payload["content"].as_str().map(str::to_owned)
        } else {
            None
        }
        .unwrap_or_else(|| format!("{:?}", event.source));
        let agent = self
            .agent()
            .with_runtime_input(crate::application::assistant::soul::runtime_input_kind(event.source), serde_json::json!({"source":format!("{:?}",event.source),"eventId":event.event_id,"ownerAssistantCode":event.assistant_code,"taskCode":event.task_code,"payload":event.payload,"onboarding":event.dedup_key.starts_with("soul-awakening:") && event.payload["onboarding"] == true}))
            .with_auto_model_route(Some(route), Some(router))
            .with_collaboration_owner(AgentCollaborationOwner {
                kind: "ASSISTANT".into(),
                code: event.assistant_code.clone(),
                origin_session_code: Some(session.session_code.clone()),
                task_code: event.task_code.clone(),
            });
        let result = self
            .state
            .chats
            .send_message(
                &session.session_code,
                crate::application::chat::SendChatMessageCommand {
                    annotations: Vec::new(),
                    requested_skills: Vec::new(),
                    a2ui_action_id: None,
                    a2ui_metadata: None,
                    run_code: Some(event.event_id.clone()),
                    content: Some(prompt),
                    workspace_code: None,
                    app_code: None,
                    task_code: event.task_code.clone(),
                    agent_code: Some(event.assistant_code.clone()),
                    model_selection: None,
                    model_code: Some(model.model_code.clone()),
                    system_model_id: None,
                    reasoning_effort: None,
                    page_context: None,
                    limits: None,
                    tools: vec![],
                    attachments: vec![],
                    client_capabilities: Some(crate::domain::chat::ChatClientCapabilities {
                        user_input_requests: true,
                        a2ui: event
                            .payload
                            .get("a2ui")
                            .filter(|v| !v.is_null())
                            .cloned()
                            .and_then(|v| serde_json::from_value(v).ok()),
                        artifacts: None,
                    }),
                    resume: vec![],
                },
                model,
                Some(run_user),
                Some(
                    crate::shared::locale::SupportedLocale::from_raw(
                        event.payload["locale"].as_str().unwrap_or("zh-CN"),
                    )
                    .as_str()
                    .into(),
                ),
                None,
                agent,
            )
            .await
            .map_err(|e| match e {
                crate::application::chat::ChatSendError::RunAlreadyActive { .. } => {
                    AssistantError::ExecutionBusy
                }
                error => AssistantError::Runtime(format!("{error:?}")),
            })?;
        let result = result.run;
        if !matches!(result.status, "COMPLETED" | "AWAITING_USER_INPUT") {
            return Err(AssistantError::Runtime(format!(
                "PersonAI execution ended with {}: {}",
                result.status, result.final_output
            )));
        }
        Ok(AssistantExecutionOutcome {
            summary: result.final_output,
            awaiting_input: result.status == "AWAITING_USER_INPUT",
        })
    }
}
pub(super) struct ThreadFeedbackBridge(pub crate::application::assistant::AssistantService);
#[async_trait::async_trait]
impl crate::application::agent::ThreadFeedbackSink for ThreadFeedbackBridge {
    async fn publish(
        &self,
        feedback: crate::application::agent::ThreadFeedback,
    ) -> Result<(), String> {
        if feedback.owner.kind != "ASSISTANT" {
            return Ok(());
        }
        let input = crate::application::assistant::AssistantEventInput {
            assistant_code: feedback.owner.code.clone(),
            source: AssistantEventSource::ThreadFeedback,
            dedup_key: feedback.event_key.clone(),
            session_code: self
                .0
                .session(&feedback.origin_session_code)
                .ok()
                .flatten()
                .filter(|s| {
                    s.assistant_code == feedback.owner.code
                        && s.purpose != AssistantSessionPurpose::Interactive
                })
                .map(|s| s.session_code),
            task_code: feedback.owner.task_code.clone(),
            workspace_code: None,
            payload: serde_json::json!({"sessionCode":feedback.session_id,"originSessionCode":feedback.origin_session_code,"status":feedback.status,"result":feedback.result,"error":feedback.error,"childRunId":feedback.child_run_id,"content":if feedback.child_run_id.is_some(){format!("后台子执行 {} 已完成（会话 {}）。基于结果继续所属工作，必要时提出需要用户处理的决定。",feedback.child_run_id.unwrap(),feedback.session_id)}else{format!("处理 Thread {} 的反馈。基于结果继续所属工作，必要时提出需要用户处理的决定。",feedback.session_id)}}),
            available_at: chrono::Utc::now().timestamp_millis(),
        };
        self.0.publish_event(input).map_err(|e| e.to_string())?;
        Ok(())
    }
}
pub(super) fn start(state: AppState) {
    let task_retention = state.tasks.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 60));
        loop {
            interval.tick().await;
            let tasks = task_retention.clone();
            match tokio::task::spawn_blocking(move || {
                tasks.prune_finished_assistant_tasks(chrono::Utc::now().timestamp_millis())
            })
            .await
            {
                Ok(Ok(count)) if count > 0 => {
                    tracing::info!(count, "Expired PersonAI tasks removed")
                }
                Ok(Ok(_)) => {}
                Ok(Err(error)) => tracing::error!(?error, "PersonAI task retention failed"),
                Err(error) => tracing::error!(%error, "PersonAI task retention worker failed"),
            }
        }
    });
    let retention_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let state = retention_state.clone();
            match tokio::task::spawn_blocking(move || {
                let candidates = state.assistants.expired_heartbeat_sessions()?;
                for code in candidates {
                    // Archiving retires the in-memory run before the transactional
                    // delete checks for queued events and live Framework leases.
                    let chat = match state.chats.get_session(&code) {
                        Ok(chat) => chat,
                        Err(crate::domain::chat::ChatError::SessionNotFound) => {
                            state.assistants.delete_empty_heartbeat_session(&code)?;
                            continue;
                        }
                        Err(error) => {
                            tracing::warn!(session_code=%code, %error, "Heartbeat history lookup failed");
                            continue;
                        }
                    };
                    let was_archived = chat.status == crate::domain::chat::ChatSessionStatus::Archived;
                    if state.chats.archive_session(&code).is_ok() {
                        if let Err(error) = state.chats.delete_expired_heartbeat_session(&code) {
                            if !was_archived { let _ = state.chats.restore_session(&code); }
                            tracing::warn!(session_code=%code, %error, "Heartbeat history retirement deferred");
                        }
                    }
                }
                state.assistants.prune_heartbeat_events()?;
                Ok::<(), AssistantError>(())
            }).await {
                Ok(Ok(())) => {},
                Ok(Err(error)) => tracing::error!(%error, "Heartbeat history retirement failed"),
                Err(error) => tracing::error!(%error, "Heartbeat history retirement worker failed"),
            }
        }
    });
    let heartbeat_assistants = state.assistants.clone();
    state
        .automations
        .register_builtin_interval(
            crate::application::automation::BuiltinScheduleTarget::AssistantHeartbeat,
            std::time::Duration::from_secs(2),
            Arc::new(move || {
                if let Err(error) =
                    heartbeat_assistants.schedule_heartbeats(chrono::Utc::now().timestamp_millis())
                {
                    tracing::error!("PersonAI heartbeat scheduling: {error}");
                }
            }),
        )
        .expect("PersonAI heartbeat scheduler should initialize");

    let _ = state
        .agent_framework
        .set_thread_feedback_sink(Arc::new(ThreadFeedbackBridge(state.assistants.clone())));
    let dispatcher = crate::application::assistant::AssistantDispatcher::new(
        state.assistants.clone(),
        Arc::new(PlatformAssistantExecutor {
            state: state.clone(),
        }),
    );
    dispatcher.start();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            if let Err(error) = state.agent_framework.recover_collaboration_threads().await {
                tracing::error!("PersonAI Thread recovery: {error}");
            }
            let now = chrono::Utc::now().timestamp_millis();
            match state.tasks.due_assistant_tasks(now) {
                Ok(tasks) => {
                    for task in tasks {
                        let Some(owner) = task.owner_assistant_code.clone() else {
                            continue;
                        };
                        if !state.assistants.get(&owner).is_ok_and(|d| d.enabled) {
                            continue;
                        }
                        if state
                            .tasks
                            .claim_assistant_task(&task.task_code, task.schedule_version, now)
                            .unwrap_or(false)
                        {
                            if let Ok(claimed) = state.tasks.get_task(&task.task_code) {
                                let _=state.assistants.publish_event(crate::application::assistant::AssistantEventInput{assistant_code:owner,source:AssistantEventSource::TaskReminder,dedup_key:format!("task:{}:{}",claimed.task_code,claimed.schedule_version),session_code:claimed.work_session_code,task_code:Some(claimed.task_code.clone()),workspace_code:claimed.workspace_code,payload:serde_json::json!({"scheduleVersion":claimed.schedule_version,"content":format!("推进任务：{}\n{}",claimed.title,claimed.description.unwrap_or_default()),"dueAt":claimed.due_at}),available_at:now});
                            }
                        }
                    }
                }
                Err(e) => tracing::error!("PersonAI task scheduling: {e:?}"),
            }
        }
    });
}

pub(super) struct MemoryNotifications(pub crate::application::assistant::AssistantService);
impl crate::application::assistant_memory::AssistantMemoryNotifications for MemoryNotifications {
    fn publish(
        &self,
        n: crate::application::assistant_memory::MemoryNotice,
    ) -> Result<(), crate::domain::assistant_memory::AssistantMemoryError> {
        // Keep revision execution separate from the assessment/notification decision.
        if n.kind == "SkillRevisionRequested" {
            self.0.publish_event(crate::application::assistant::AssistantEventInput {
                assistant_code: n.assistant_code.clone(), source: AssistantEventSource::ThreadFeedback,
                dedup_key: format!("skill-revision:{}:{}", n.related_code, n.revision),
                session_code: None, task_code: None, workspace_code: None,
                payload: serde_json::json!({"resumeMemoryCandidates":true,"candidateCode":n.related_code,"content":n.message}),
                available_at: chrono::Utc::now().timestamp_millis(),
            }).map_err(|error| crate::domain::assistant_memory::AssistantMemoryError::Storage(error.to_string()))?;
        }
        // Candidate lifecycle changes are inputs for the owning PersonAI's assessment,
        // not a second path that bypasses the platform notification tool.
        self.0.publish_event(crate::application::assistant::AssistantEventInput {
            assistant_code: n.assistant_code,
            source: AssistantEventSource::ThreadFeedback,
            dedup_key: format!("skill-assessment:{}:{}:{}", n.related_code, n.revision, n.kind),
            session_code: None, task_code: None, workspace_code: None,
            payload: serde_json::json!({"businessNotice":{"candidateCode":n.related_code,"revision":n.revision,"kind":n.kind},"content":format!("技能事项发生变化：{}。{}。请检查候选项实际状态，关联或创建你负责的事项，评估进展、是否需要用户处理，以及已有待确认项是否解决。按事项变化协议决定是否通知，不要将每次内部状态变化都通知用户。",n.kind,n.message)}),
            available_at: chrono::Utc::now().timestamp_millis(),
        }).map_err(|error| crate::domain::assistant_memory::AssistantMemoryError::Storage(error.to_string()))?;
        Ok(())
    }
}

struct MemorySessionFactory {
    assistants: crate::application::assistant::AssistantService,
    chats: crate::application::chat::ChatService,
}
#[async_trait::async_trait]
impl crate::application::assistant_memory::AssistantMemorySessionFactory for MemorySessionFactory {
    async fn author_session(
        &self,
        assistant: &str,
        user: &str,
        batch: &str,
        skill: &str,
    ) -> Result<String, crate::domain::assistant_memory::AssistantMemoryError> {
        let session = self
            .assistants
            .resolve_session(
                assistant,
                AssistantSessionPurpose::SkillAuthoring,
                &format!("{batch}:{skill}"),
                None,
            )
            .map_err(|e| {
                crate::domain::assistant_memory::AssistantMemoryError::Storage(e.to_string())
            })?;
        self.chats
            .ensure_assistant_session(&session, Some(user.into()))
            .map_err(|e| {
                crate::domain::assistant_memory::AssistantMemoryError::Storage(e.to_string())
            })?;
        Ok(session.session_code)
    }
}
