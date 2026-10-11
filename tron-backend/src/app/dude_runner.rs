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
        let s = &self.state;
        AgentInvocationService::new(
            s.workspaces.clone(),
            s.apps.clone(),
            s.variables.clone(),
            s.skills.clone(),
            s.agent_management.clone(),
            s.agent_runs.clone(),
            s.mcp.clone(),
            s.event_bus.clone(),
            s.agent_model_client.clone(),
            s.agent_tool_registry.clone(),
            s.runtime_http_client.clone(),
        )
        .with_framework_host(s.agent_framework.clone())
        .with_active_runs(s.active_runs.clone())
        .with_short_term_memory(s.short_term_memory.clone())
        .with_prompt_service(s.prompts.clone())
        .with_platform_scope()
    }
}
#[async_trait::async_trait]
impl DudeExecutor for PlatformDudeExecutor {
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
        self.state
            .chats
            .ensure_dude_session(&session, None)
            .map_err(|_| DudeError::Storage)?;
        let router = crate::application::model_router::AgentModelRouter::new(
            self.state.models.clone(),
            self.state.model_providers.clone(),
        )
        .with_agent_management(self.state.agent_management.clone());
        let (model, route) = router
            .resolve_initial("", &session.session_code, &event.dude_code, None)
            .map_err(DudeError::Runtime)?;
        let prompt=event.payload["content"].as_str().map(str::to_owned).unwrap_or_else(||format!("处理 {source:?} 事件。仅在有实际工作时行动；无变化的心跳保持安静。事件数据：{payload}",source=event.source,payload=event.payload));
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
                None,
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
pub(super) fn start(state: AppState) {
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
        self.0
            .notify(crate::application::dude::NotificationInput {
                dedup_key: format!("{}:{}:{}", n.kind, n.related_code, n.message),
                dude_code: Some(n.dude_code.clone()),
                title: n.kind.into(),
                body: n.message.clone(),
                category: "Skill".into(),
                target: serde_json::json!({"candidateCode":n.related_code}),
            })
            .map_err(|_| {
                crate::domain::dude_memory::DudeMemoryError::Storage(
                    "Dude notification persistence failed".into(),
                )
            })?;
        if n.kind == "SkillRevisionRequested" {
            self.0.publish_event(crate::application::dude::DudeEventInput{dude_code:n.dude_code,source:DudeEventSource::ThreadFeedback,dedup_key:format!("skill-revision:{}:{}",n.related_code,n.message),session_code:None,task_code:None,workspace_code:None,payload:serde_json::json!({"candidateCode":n.related_code,"content":n.message}),available_at:chrono::Utc::now().timestamp_millis()}).map_err(|_|crate::domain::dude_memory::DudeMemoryError::Storage("Dude notification persistence failed".into()))?;
        }
        Ok(())
    }
}
