use crate::{
    app::AppState,
    application::{
        agent::AgentInvocationService,
        dude::{DudeExecutionOutcome, DudeExecutor},
    },
    domain::{agent::AgentCollaborationOwner, dude::*},
};
use std::sync::Arc;
pub(super) struct PlatformDudeExecutor {
    pub state: AppState,
}
impl PlatformDudeExecutor {
    fn agent(&self) -> AgentInvocationService {
        self.state.dude_invocation_service()
    }
}
#[async_trait::async_trait]
impl DudeExecutor for PlatformDudeExecutor {
    fn is_ready(&self) -> Result<bool, DudeError> {
        self.state
            .profiles
            .active()
            .map(|profile| profile.is_some())
            .map_err(|error| DudeError::Runtime(error.to_string()))
    }

    async fn recover(&self, event: &DudeEvent) -> Result<Option<DudeExecutionOutcome>, DudeError> {
        let framework = self
            .state
            .agent_framework
            .storage_framework()
            .await
            .map_err(|error| DudeError::Runtime(error.to_string()))?;
        let store = framework
            .sessions()
            .map_err(|error| DudeError::Runtime(error.to_string()))?
            .store;
        let run_id = eliteforge_agent_core::RunId::parse_str(&event.event_id)
            .map_err(|_| DudeError::Invalid)?;
        match store.get_run(run_id).await {
            Ok(run) => {
                if let Some(result) = run.result {
                    if matches!(
                        result.status,
                        eliteforge_agent_core::RunStatus::Completed
                            | eliteforge_agent_core::RunStatus::AwaitingInput
                    ) {
                        return Ok(Some(DudeExecutionOutcome {
                            summary: result.output,
                            awaiting_input: result.status
                                == eliteforge_agent_core::RunStatus::AwaitingInput,
                            notify: event.source != DudeEventSource::Heartbeat,
                        }));
                    }
                }
                Err(DudeError::Runtime("Dude execution was interrupted. Inspect its tools and external results before continuing the task.".into()))
            }
            Err(eliteforge_agent_core::StateError::NotFound) => Ok(None),
            Err(error) => Err(DudeError::Runtime(error.to_string())),
        }
    }
    async fn execute(
        &self,
        event: DudeEvent,
        session: DudeSession,
    ) -> Result<DudeExecutionOutcome, DudeError> {
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
            .map_err(|error| DudeError::Runtime(format!("Dude tracing unavailable: {error:?}")))?;
        // A durable wakeup is a new operation, even if dispatch was initiated by
        // an HTTP request or another Dude. Never inherit the worker's context.
        let guard = telemetry.begin_operation(
            "tron.dude.execute",
            &Context::new(),
            vec![
                KeyValue::new("tron.dude.code", event.dude_code.clone()),
                KeyValue::new("tron.dude.event.id", event.event_id.clone()),
                KeyValue::new("tron.dude.event.source", format!("{:?}", event.source)),
                KeyValue::new("tron.dude.event.attempt", i64::from(event.attempts)),
                KeyValue::new("tron.session.code", session.session_code.clone()),
                KeyValue::new("tron.agent.code", event.dude_code.clone()),
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
        let log_span = tracing::info_span!("dude.execute", trace_id=%ids.trace_id(), span_id=%ids.span_id(),
            dude_code=%event.dude_code, event_id=%event.event_id);
        let result = async {
            tracing::info!("Dude execution started");
            let result = self.execute_inner(event, session).await;
            if result.is_err() {
                context
                    .span()
                    .set_status(Status::error("Dude execution failed"));
                tracing::error!("Dude execution failed");
            } else {
                tracing::info!("Dude execution finished");
            }
            result
        }
        .instrument(log_span)
        .with_context(context.clone())
        .await;
        guard.complete(match &result {
            Ok(outcome) if outcome.awaiting_input => "waiting",
            Ok(_) => "completed",
            Err(_) => "error",
        });
        result
    }
}

impl PlatformDudeExecutor {
    async fn execute_inner(
        &self,
        event: DudeEvent,
        session: DudeSession,
    ) -> Result<DudeExecutionOutcome, DudeError> {
        let dude = self.state.dudes.get(&event.dude_code)?;
        if dude.kind == DudeKind::Memory
            && matches!(
                event.source,
                DudeEventSource::Heartbeat
                    | DudeEventSource::MemoryBatch
                    | DudeEventSource::ThreadFeedback
            )
        {
            let profile = self
                .state
                .profiles
                .current_context()
                .map_err(|e| DudeError::Runtime(e.to_string()))?;
            let user = profile.person_ai_user_id;
            let router = crate::application::model_router::AgentModelRouter::new(
                self.state.models.clone(),
                self.state.model_providers.clone(),
            )
            .with_agent_management(self.state.agent_management.clone());
            let runner = crate::application::dude_memory::DudeMemoryRunner::new(
                self.state.dude_memory.clone(),
                self.state.memory.clone(),
                self.state.agent_framework.clone(),
                self.agent(),
                router,
                Arc::new(MemorySessionFactory {
                    dudes: self.state.dudes.clone(),
                    chats: self.state.chats.clone(),
                }),
            );
            runner
                .resume_candidates(&event.dude_code, &user, dude.memory_auto_approve)
                .await
                .map_err(|e| DudeError::Runtime(e.to_string()))?;
            let Some(batch) = runner
                .prepare_wakeup(&event.dude_code, &user)
                .await
                .map_err(|e| DudeError::Runtime(e.to_string()))?
            else {
                return Ok(DudeExecutionOutcome {
                    summary: String::new(),
                    awaiting_input: false,
                    notify: false,
                });
            };
            let batch_session = self.state.dudes.resolve_session(
                &event.dude_code,
                DudeSessionPurpose::MemoryBatch,
                &batch.batch_code,
                None,
            )?;
            self.state
                .dudes
                .attach_session(&event, &batch_session.session_code)?;
            self.state
                .chats
                .ensure_dude_session(&batch_session, Some(user.clone()))
                .map_err(|_| DudeError::Storage)?;
            let batch = runner
                .execute_batch(
                    &user,
                    &batch.batch_code,
                    &batch_session.session_code,
                    dude.memory_auto_approve,
                )
                .await
                .map_err(|e| DudeError::Runtime(e.to_string()))?;
            return Ok(DudeExecutionOutcome {
                summary: batch.summary.unwrap_or_default(),
                awaiting_input: false,
                notify: false,
            });
        }
        if event.source == DudeEventSource::TaskReminder {
            if let Some(code) = &event.task_code {
                let version = event.payload["scheduleVersion"].as_i64().unwrap_or(0);
                if !self
                    .state
                    .tasks
                    .dude_task_reminder_is_current(code, version)
                    .map_err(|_| DudeError::Storage)?
                {
                    return Ok(DudeExecutionOutcome {
                        summary: "任务已调整，旧提醒已跳过".into(),
                        awaiting_input: false,
                        notify: false,
                    });
                }
                self.state
                    .tasks
                    .bind_dude_work_session(code, &session.session_code)
                    .map_err(|_| DudeError::Storage)?;
            }
        }
        let run_user = self
            .state
            .profiles
            .current_context()
            .map_err(|e| DudeError::Runtime(e.to_string()))?
            .person_ai_user_id;
        self.state
            .chats
            .ensure_dude_session(&session, Some(run_user.clone()))
            .map_err(|_| DudeError::Storage)?;
        let router = crate::application::model_router::AgentModelRouter::new(
            self.state.models.clone(),
            self.state.model_providers.clone(),
        )
        .with_agent_management(self.state.agent_management.clone());
        let (model, route) = router
            .resolve_initial(
                session.workspace_code.as_deref().unwrap_or(""),
                &session.session_code,
                &event.dude_code,
                None,
            )
            .map_err(DudeError::Runtime)?;
        let mut prompt=event.payload["content"].as_str().map(str::to_owned).unwrap_or_else(||format!("处理 {source:?} 事件。仅在有实际工作时行动；无变化的心跳保持安静。事件数据：{payload}",source=event.source,payload=event.payload));
        if event.source != DudeEventSource::UserMessage {
            prompt.push_str(&format!("\n事件数据：{}", event.payload));
        }
        if event.source == DudeEventSource::ThreadFeedback {
            prompt.push_str("\n处理 Thread 反馈时，先读取 result.interactions。若已有上下文足以回答，调用 thread_answer，使用实际 session_id、interaction_id 和符合 response_schema 的 answer；不要给等待回答的 Thread 发送普通消息。若需要用户补充事实或作决定，调用 dude_notify 通知用户并指向此 Thread，保留等待状态。任务完成后更新对应 task 状态。没有实际进展时保持安静。");
        }
        if event.source == DudeEventSource::Heartbeat {
            prompt.push_str("\n按你的角色检查需要主动推进的事项。已由任务调度触发的工作在对应 Task 会话运行；不要在心跳会话重复执行这些任务。仅在发现实际变化、需要决策或取得有用结果时通过 dude_notify 通知；无变化保持安静。");
        }
        let agent = self
            .agent()
            .with_runtime_input(format!("DUDE_{:?}", event.source).to_uppercase(), serde_json::json!({"source":format!("{:?}",event.source),"eventId":event.event_id,"ownerDudeCode":event.dude_code,"taskCode":event.task_code}))
            .with_auto_model_route(Some(route), Some(router))
            .with_collaboration_owner(AgentCollaborationOwner {
                kind: "DUDE".into(),
                code: event.dude_code.clone(),
                origin_session_code: Some(session.session_code.clone()),
                task_code: event.task_code.clone(),
            });
        let result = self
            .state
            .chats
            .send_message(
                &session.session_code,
                crate::application::chat::SendChatMessageCommand {
                    a2ui_action_id: None,
                    a2ui_metadata: None,
                    run_code: Some(event.event_id.clone()),
                    content: Some(prompt),
                    workspace_code: None,
                    app_code: None,
                    task_code: event.task_code.clone(),
                    agent_code: Some(event.dude_code.clone()),
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
                        a2ui: None,
                    }),
                    resume: vec![],
                },
                model,
                Some(run_user),
                Some("zh-CN".into()),
                None,
                agent,
            )
            .await
            .map_err(|e| DudeError::Runtime(format!("{e:?}")))?;
        let result = result.run;
        Ok(DudeExecutionOutcome {
            summary: result.final_output,
            awaiting_input: result.status == "AWAITING_USER_INPUT",
            notify: event.source != DudeEventSource::Heartbeat,
        })
    }
}
pub(super) struct ThreadFeedbackBridge(pub crate::application::dude::DudeService);
#[async_trait::async_trait]
impl crate::application::agent::ThreadFeedbackSink for ThreadFeedbackBridge {
    async fn publish(
        &self,
        feedback: crate::application::agent::ThreadFeedback,
    ) -> Result<(), String> {
        if feedback.owner.kind != "DUDE" {
            return Ok(());
        }
        let input = crate::application::dude::DudeEventInput {
            dude_code: feedback.owner.code.clone(),
            source: DudeEventSource::ThreadFeedback,
            dedup_key: feedback.event_key.clone(),
            session_code: self
                .0
                .session(&feedback.origin_session_code)
                .ok()
                .flatten()
                .filter(|s| {
                    s.dude_code == feedback.owner.code
                        && s.purpose != DudeSessionPurpose::Interactive
                })
                .map(|s| s.session_code),
            task_code: feedback.owner.task_code.clone(),
            workspace_code: None,
            payload: serde_json::json!({"sessionCode":feedback.session_id,"originSessionCode":feedback.origin_session_code,"status":feedback.status,"result":feedback.result,"error":feedback.error,"content":format!("处理 Thread {} 的反馈。基于结果继续所属工作，必要时提出需要用户处理的决定。",feedback.session_id)}),
            available_at: chrono::Utc::now().timestamp_millis(),
        };
        self.0.publish_event(input).map_err(|e| e.to_string())?;
        Ok(())
    }
}
pub(super) fn start(state: AppState) {
    let heartbeat_dudes = state.dudes.clone();
    state
        .automations
        .register_builtin_interval(
            crate::application::automation::BuiltinScheduleTarget::DudeHeartbeat,
            std::time::Duration::from_secs(2),
            Arc::new(move || {
                if let Err(error) =
                    heartbeat_dudes.schedule_heartbeats(chrono::Utc::now().timestamp_millis())
                {
                    tracing::error!("Dude heartbeat scheduling: {error}");
                }
            }),
        )
        .expect("Dude heartbeat scheduler should initialize");

    let _ = state
        .agent_framework
        .set_thread_feedback_sink(Arc::new(ThreadFeedbackBridge(state.dudes.clone())));
    let dispatcher = crate::application::dude::DudeDispatcher::new(
        state.dudes.clone(),
        Arc::new(PlatformDudeExecutor {
            state: state.clone(),
        }),
    );
    dispatcher.start();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            if let Err(error) = state.agent_framework.recover_collaboration_threads().await {
                tracing::error!("Dude Thread recovery: {error}");
            }
            let now = chrono::Utc::now().timestamp_millis();
            match state.tasks.due_dude_tasks(now) {
                Ok(tasks) => {
                    for task in tasks {
                        let Some(owner) = task.owner_dude_code.clone() else {
                            continue;
                        };
                        if !state.dudes.get(&owner).is_ok_and(|d| d.enabled) {
                            continue;
                        }
                        if state
                            .tasks
                            .claim_dude_task(&task.task_code, task.schedule_version, now)
                            .unwrap_or(false)
                        {
                            if let Ok(claimed) = state.tasks.get_task(&task.task_code) {
                                let _=state.dudes.publish_event(crate::application::dude::DudeEventInput{dude_code:owner,source:DudeEventSource::TaskReminder,dedup_key:format!("task:{}:{}",claimed.task_code,claimed.schedule_version),session_code:claimed.work_session_code,task_code:Some(claimed.task_code.clone()),workspace_code:claimed.workspace_code,payload:serde_json::json!({"scheduleVersion":claimed.schedule_version,"content":format!("推进任务：{}\n{}",claimed.title,claimed.description.unwrap_or_default()),"dueAt":claimed.due_at}),available_at:now});
                            }
                        }
                    }
                }
                Err(e) => tracing::error!("Dude task scheduling: {e:?}"),
            }
        }
    });
}

pub(super) struct MemoryNotifications(pub crate::application::dude::DudeService);
impl crate::application::dude_memory::DudeMemoryNotifications for MemoryNotifications {
    fn publish(
        &self,
        n: crate::application::dude_memory::MemoryNotice,
    ) -> Result<(), crate::domain::dude_memory::DudeMemoryError> {
        let retire: &[&str] = match n.kind {
            "SkillPendingApproval" | "SkillInstalled" | "SkillRejected" => &[
                "SkillDependencyMissing",
                "SkillValidationFailed",
                "SkillPendingApproval",
                "SkillRevisionRequested",
                "SkillInstallFailed",
            ],
            "SkillValidationFailed" => &[
                "SkillDependencyMissing",
                "SkillPendingApproval",
                "SkillRevisionRequested",
                "SkillValidationFailed",
            ],
            "SkillRevisionRequested" => &[
                "SkillPendingApproval",
                "SkillValidationFailed",
                "SkillDependencyMissing",
            ],
            "SkillInstallFailed" => &[
                "SkillPendingApproval",
                "SkillRevisionRequested",
                "SkillInstallFailed",
            ],
            "SkillIndexRecovered" => &["SkillIndexFailed"],
            _ => &[],
        };
        self.0
            .notify_replacing(
                crate::application::dude::NotificationInput {
                    dedup_key: format!("{}:{}:{}", n.related_code, n.revision, n.kind),
                    dude_code: Some(n.dude_code.clone()),
                    title: n.kind.into(),
                    body: n.message.clone(),
                    category: "Skill".into(),
                    target: serde_json::json!({"candidateCode":n.related_code}),
                },
                retire,
            )
            .map_err(|_| {
                crate::domain::dude_memory::DudeMemoryError::Storage(
                    "Dude notification persistence failed".into(),
                )
            })?;
        if n.kind == "SkillRevisionRequested" {
            self.0.publish_event(crate::application::dude::DudeEventInput{dude_code:n.dude_code,source:DudeEventSource::ThreadFeedback,dedup_key:format!("skill-revision:{}:{}",n.related_code,n.revision),session_code:None,task_code:None,workspace_code:None,payload:serde_json::json!({"candidateCode":n.related_code,"content":n.message}),available_at:chrono::Utc::now().timestamp_millis()}).map_err(|_|crate::domain::dude_memory::DudeMemoryError::Storage("Dude notification persistence failed".into()))?;
        }
        Ok(())
    }
}

struct MemorySessionFactory {
    dudes: crate::application::dude::DudeService,
    chats: crate::application::chat::ChatService,
}
#[async_trait::async_trait]
impl crate::application::dude_memory::DudeMemorySessionFactory for MemorySessionFactory {
    async fn author_session(
        &self,
        dude: &str,
        user: &str,
        batch: &str,
        skill: &str,
    ) -> Result<String, crate::domain::dude_memory::DudeMemoryError> {
        let session = self
            .dudes
            .resolve_session(
                dude,
                DudeSessionPurpose::SkillAuthoring,
                &format!("{batch}:{skill}"),
                None,
            )
            .map_err(|e| crate::domain::dude_memory::DudeMemoryError::Storage(e.to_string()))?;
        self.chats
            .ensure_dude_session(&session, Some(user.into()))
            .map_err(|e| crate::domain::dude_memory::DudeMemoryError::Storage(e.to_string()))?;
        Ok(session.session_code)
    }
}
