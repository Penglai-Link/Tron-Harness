//! Addressable in-memory lifecycle for synchronous and background child runs.
//! Paused children remain registered so child_answer resumes their checkpoint.
//!
//! Detached children keep the delegation semantics of spawn (lineage, depth,
//! subagent records, live presentation) while owning an independent
//! cancellation token and deadline: the parent run ending, failing or being
//! cancelled never stops them. Unlike threads the registry is not durable;
//! a completed result is delivered through `child_wait`, and completion is
//! notified to the parent session: Dude-owned children go through the durable
//! feedback outbox that wakes the Dude; agent-owned children steer the still
//! running parent run directly.
use super::*;
use eliteforge_agent_framework::RunControl;
use std::sync::RwLock;
use tokio::sync::watch;

const MAX_DETACHED_CHILDREN: usize = 128;

pub(in crate::application::agent::framework_runtime::host) type Children =
    RwLock<BTreeMap<String, Arc<DetachedChild>>>;

type ChildCompletion = (fw::ChildRunStatus, Result<fw::ChildRunResult, String>);

pub(in crate::application::agent::framework_runtime::host) struct DetachedChild {
    context: AgentRunContext,
    agent: Agent,
    transition: Mutex<()>,
    state: std::sync::Mutex<ChildState>,
}

struct ChildState {
    summary: fw::DetachedChildSummary,
    completion: watch::Receiver<Option<ChildCompletion>>,
    control: RunControl,
    owner: crate::domain::agent::AgentCollaborationOwner,
    workspace_code: String,
    user_code: Option<String>,
    origin_session_code: String,
    parent_run_code: String,
    execution_sequence: u64,
    /// True once `child_wait` delivered this run's final result.
    delivered: bool,
}

fn thread_status(status: &fw::ChildRunStatus) -> fw::ThreadStatus {
    match status {
        fw::ChildRunStatus::Running => fw::ThreadStatus::Running,
        fw::ChildRunStatus::Completed => fw::ThreadStatus::Completed,
        fw::ChildRunStatus::AwaitingInput => fw::ThreadStatus::AwaitingInput,
        fw::ChildRunStatus::StepLimit => fw::ThreadStatus::StepLimit,
        fw::ChildRunStatus::ToolCallLimit => fw::ThreadStatus::ToolCallLimit,
        fw::ChildRunStatus::Failed => fw::ThreadStatus::Failed,
        fw::ChildRunStatus::Cancelled => fw::ThreadStatus::Cancelled,
    }
}

impl Runtime {
    fn child_summary(child: &DetachedChild) -> fw::DetachedChildSummary {
        child.state.lock().unwrap().summary.clone()
    }
    async fn child(&self, id: &str) -> Result<Arc<DetachedChild>, fw::AgentError> {
        let host = self.host()?;
        let child = host
            .0
            .collaboration_children
            .read()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| failure("child run not found"))?;
        let authorized = {
            let state = child.state.lock().unwrap();
            state.workspace_code == self.context.workspace_code
                && state.user_code == self.context.user_code
                && state.owner.kind
                    == self
                        .context
                        .collaboration_owner
                        .as_ref()
                        .map(|o| o.kind.as_str())
                        .unwrap_or("USER")
                && state.owner.code
                    == self
                        .context
                        .collaboration_owner
                        .as_ref()
                        .map(|o| o.code.clone())
                        .unwrap_or_else(|| {
                            self.context
                                .user_code
                                .clone()
                                .unwrap_or_else(|| "system".into())
                        })
        };
        if !authorized {
            return Err(failure("child run not found"));
        }
        let agent_code = child
            .state
            .lock()
            .unwrap()
            .summary
            .agent_code
            .clone()
            .ok_or_else(|| failure("child target missing"))?;
        self.authorize(&agent_code)?;
        Ok(child)
    }
    pub(super) async fn spawn_detached_child(
        &self,
        request: fw::RunRequest,
        parent: &fw::RunContext,
    ) -> Result<fw::DetachedChildSummary, fw::AgentError> {
        self.spawn_registered_child(request, parent, true).await
    }
    pub(super) async fn spawn_registered_child(
        &self,
        request: fw::RunRequest,
        parent: &fw::RunContext,
        detached: bool,
    ) -> Result<fw::DetachedChildSummary, fw::AgentError> {
        let session = format!("tool-child-{}", fw::RunId::new_v4());
        let (agent, mut request, prepared) = self
            .prepare_new(
                request,
                session.clone(),
                parent,
                if detached {
                    ExecutionLimits::AgentDefinitionDetached
                } else {
                    ExecutionLimits::AgentDefinition
                },
            )
            .await?;
        let host = self.host()?;
        let owner = parent
            .extensions
            .get::<super::super::subagents::SubagentOwner>()
            .map(|owner| owner.0.clone())
            .unwrap_or_else(|| parent.session_id.clone());
        request
            .extensions
            .insert(super::super::subagents::SubagentOwner(owner.clone()));
        let code = prepared
            .agent_code
            .clone()
            .ok_or_else(|| failure("child target missing"))?;
        let target = self
            .directory()?
            .into_iter()
            .find(|entry| entry.agent_code == code);
        let name = target
            .as_ref()
            .map(|entry| entry.name.clone())
            .unwrap_or_else(|| code.clone());
        let avatar_id = target.map(|entry| entry.avatar_id);
        let task = request.input.text();
        let admission = host.0.config_lock.lock().await;
        host.ensure_open().map_err(failure)?;
        self.authorize(&code)?;
        let (child_handle, stream) = if detached {
            agent.start_child_detached_with_output_owned::<eliteforge_agent_official_plugin_agui::AgUiStream>(request, parent).await?
        } else {
            agent
                .start_child_with_output_owned::<eliteforge_agent_official_plugin_agui::AgUiStream>(
                    request, parent,
                )
                .await?
        };
        drop(admission);
        let run_id = child_handle.run_id();
        let control = child_handle.control();
        host.record_subagent(
            &owner,
            super::super::SubagentRecord {
                run_id,
                session_id: session.clone(),
                parent_run_id: parent.run_id,
                agent_code: code.clone(),
                agent_name: name,
                avatar_id,
                task: task.clone(),
                user_code: prepared.user_code.clone(),
            },
        )
        .await?;
        let (sender, completion) = watch::channel(None);
        let summary = fw::DetachedChildSummary {
            child_run_id: run_id,
            session_id: session.clone(),
            agent_code: Some(code),
            status: fw::ChildRunStatus::Running,
        };
        let state = ChildState {
            summary: summary.clone(),
            completion,
            control,
            owner: super::threads::owner(&self.context),
            workspace_code: prepared.workspace_code.clone(),
            user_code: prepared.user_code.clone(),
            origin_session_code: self
                .context
                .collaboration_owner
                .as_ref()
                .and_then(|owner| owner.origin_session_code.clone())
                .unwrap_or_else(|| parent.session_id.clone()),
            parent_run_code: self.context.run_code.clone(),
            execution_sequence: 1,
            delivered: false,
        };
        let child = Arc::new(DetachedChild {
            context: prepared.clone(),
            agent,
            transition: Mutex::new(()),
            state: std::sync::Mutex::new(state),
        });
        {
            let mut children = host.0.collaboration_children.write().unwrap();
            if children.len() >= MAX_DETACHED_CHILDREN {
                // Delivered terminal runs keep only a small snapshot; release
                // them before refusing new work.
                children.retain(|_, child| {
                    let state = child.state.lock().unwrap();
                    !(state.delivered
                        && !matches!(
                            state.summary.status,
                            fw::ChildRunStatus::Running | fw::ChildRunStatus::AwaitingInput
                        ))
                });
            }
            if children.len() >= MAX_DETACHED_CHILDREN {
                return Err(failure(
                    "detached child capacity exceeded; collect finished children first",
                ));
            }
            children.insert(run_id.to_string(), child.clone());
        }
        let presentation = host.presentation_service().map_err(failure)?;
        let notify_host = host.clone();
        let trace_id = {
            use opentelemetry::trace::TraceContextExt;
            opentelemetry::Context::current()
                .span()
                .span_context()
                .trace_id()
                .to_string()
        };
        crate::app::telemetry::spawn_traced(async move {
            // The handle owns the child until completion; the registry only
            // mirrors its state so parents can wait, steer, answer or cancel.
            let (result, ()) = tokio::join!(
                child_handle.wait(),
                super::super::super::presentation::consume_native_agui(
                    stream,
                    &prepared,
                    presentation,
                    &trace_id,
                )
            );
            let (status, stored) = match result {
                Ok(result) => {
                    let status = fw::ChildRunStatus::from(&result.status);
                    (status, Ok(fw::ChildRunResult { run_id, result }))
                }
                Err(error) => {
                    let status = if matches!(error.primary(), fw::AgentError::Cancelled) {
                        fw::ChildRunStatus::Cancelled
                    } else {
                        fw::ChildRunStatus::Failed
                    };
                    (status, Err(error.to_string()))
                }
            };
            let notification = {
                let mut state = child.state.lock().unwrap();
                state.summary.status = status.clone();
                let notification = ChildNotification {
                    owner: state.owner.clone(),
                    session_id: state.summary.session_id.clone(),
                    origin_session_code: state.origin_session_code.clone(),
                    parent_run_code: state.parent_run_code.clone(),
                    status: status.clone(),
                    run_id,
                    sequence: state.execution_sequence,
                    result: stored.clone(),
                };
                sender.send_replace(Some((status, stored)));
                notification
            };
            notify_host.deliver_child_notification(notification).await;
        });
        Ok(summary)
    }
    pub(super) async fn wait_detached_child(
        &self,
        child_run_id: &str,
    ) -> Result<fw::ChildRunResult, fw::AgentError> {
        let child = self.child(child_run_id).await?;
        let mut completion = child.state.lock().unwrap().completion.clone();
        let finished = loop {
            if let Some(finished) = completion.borrow_and_update().clone() {
                break finished;
            }
            completion
                .changed()
                .await
                .map_err(|_| failure("child execution owner terminated"))?;
        };
        // The result now belongs to the caller; the record stays addressable
        // for read/send/answer until capacity pressure releases it.
        child.state.lock().unwrap().delivered = true;
        finished.1.map_err(fw::AgentError::Run)
    }
    pub(super) async fn send_detached_child(
        &self,
        child_run_id: &str,
        message: &str,
    ) -> Result<fw::DetachedChildSummary, fw::AgentError> {
        let child = self.child(child_run_id).await?;
        let state = child.state.lock().unwrap();
        if state.summary.status != fw::ChildRunStatus::Running {
            return Err(failure(
                "child run is not running; steering requires an active execution",
            ));
        }
        state.control.steer(message.to_owned())?;
        Ok(state.summary.clone())
    }
    pub(super) async fn cancel_detached_child(
        &self,
        child_run_id: &str,
    ) -> Result<fw::DetachedChildSummary, fw::AgentError> {
        let child = self.child(child_run_id).await?;
        let state = child.state.lock().unwrap();
        state.control.cancel();
        Ok(state.summary.clone())
    }
    pub(super) async fn read_detached_child(
        &self,
        child_run_id: &str,
        limit: usize,
    ) -> Result<fw::ChildRunView, fw::AgentError> {
        let child = self.child(child_run_id).await?;
        let session = child.state.lock().unwrap().summary.session_id.clone();
        // Reading a live conversation races its own writes; require a settled
        // run and steer parents towards child_wait or the completion notice.
        if child.state.lock().unwrap().summary.status == fw::ChildRunStatus::Running {
            return Err(failure(
                "child run is still running; wait for it before reading",
            ));
        }
        let mut conversation = child
            .agent
            .conversation(&fw::ConversationKey::new(session))
            .await?;
        if conversation.messages.len() > limit {
            let start = conversation.messages.len() - limit;
            conversation.messages = conversation.messages.split_off(start);
        }
        Ok(fw::ChildRunView {
            summary: Self::child_summary(&child),
            conversation,
        })
    }
    pub(super) async fn list_detached_children(
        &self,
    ) -> Result<Vec<fw::DetachedChildSummary>, fw::AgentError> {
        let host = self.host()?;
        let authorized: std::collections::HashSet<_> = self
            .directory()?
            .into_iter()
            .map(|entry| entry.agent_code)
            .collect();
        let owner = super::threads::owner(&self.context);
        Ok(host
            .0
            .collaboration_children
            .read()
            .unwrap()
            .values()
            .filter(|child| {
                let state = child.state.lock().unwrap();
                state.workspace_code == self.context.workspace_code
                    && state.user_code == self.context.user_code
                    && state.owner.kind == owner.kind
                    && state.owner.code == owner.code
                    && state
                        .summary
                        .agent_code
                        .as_deref()
                        .is_some_and(|code| authorized.contains(code))
            })
            .map(|child| Self::child_summary(child))
            .collect())
    }
    pub(super) async fn answer_detached_child(
        &self,
        child_run_id: &str,
        interaction: fw::RunId,
        value: serde_json::Value,
        parent: &fw::RunContext,
    ) -> Result<fw::DetachedChildSummary, fw::AgentError> {
        let child = self.child(child_run_id).await?;
        let _transition = child.transition.lock().await;
        {
            let state = child.state.lock().unwrap();
            if state.summary.status == fw::ChildRunStatus::Running {
                return Err(fw::AgentError::SessionBusy(
                    state.summary.session_id.clone(),
                ));
            }
            if state.summary.status != fw::ChildRunStatus::AwaitingInput {
                return Err(failure("child run is not awaiting input"));
            }
            let current = state.completion.borrow();
            if !current.as_ref().is_some_and(|(_, result)| {
                result.as_ref().is_ok_and(|result| {
                    result
                        .result
                        .interactions
                        .iter()
                        .any(|request| request.id == interaction)
                })
            }) {
                return Err(failure(
                    "interaction is not pending on the current child execution",
                ));
            }
        }
        let session = child.state.lock().unwrap().summary.session_id.clone();
        let request = child
            .agent
            .answer_with_provenance(
                &session,
                interaction,
                value,
                Some(self.answer_provenance(parent, interaction)),
            )
            .await?;
        let (_, request, _) = self
            .prepare_existing(
                request,
                session,
                &child.context,
                parent,
                ExecutionLimits::Request,
            )
            .await?;
        // Keep the transition owned until the resumed execution is registered.
        // A competing answer cannot consume the same pause before admission.
        {
            let mut state = child.state.lock().unwrap();
            state.parent_run_code = self.context.run_code.clone();
        }
        self.launch_child_run(child.clone(), request).await
    }
    /// Restart one detached child for a resumed execution. Resumed runs are
    /// native root executions of the child session, mirroring thread answers.
    pub(super) async fn launch_child_run(
        &self,
        child: Arc<DetachedChild>,
        request: fw::RunRequest,
    ) -> Result<fw::DetachedChildSummary, fw::AgentError> {
        let host = self.host()?;
        let _admission = host.0.config_lock.lock().await;
        host.ensure_open().map_err(failure)?;
        let handle = child.agent.start(request)?;
        let run_id = handle.run_id;
        let control = child.agent.control(&handle)?;
        let (sender, completion) = watch::channel(None);
        let summary = {
            let mut state = child.state.lock().unwrap();
            state.summary.status = fw::ChildRunStatus::Running;
            state.delivered = false;
            state.control = control;
            state.completion = completion;
            state.execution_sequence = state.execution_sequence.saturating_add(1);
            state.summary.clone()
        };
        let host_cell = host.0.clone();
        let context = child.context.clone();
        crate::app::telemetry::spawn_traced(async move {
            let result = child.agent.wait(handle).await;
            let (status, stored) = match result {
                Ok(result) => {
                    let status = fw::ChildRunStatus::from(&result.status);
                    (status, Ok(fw::ChildRunResult { run_id, result }))
                }
                Err(error) => {
                    let status = if matches!(error.primary(), fw::AgentError::Cancelled) {
                        fw::ChildRunStatus::Cancelled
                    } else {
                        fw::ChildRunStatus::Failed
                    };
                    (status, Err(error.to_string()))
                }
            };
            let notification = {
                let mut state = child.state.lock().unwrap();
                state.summary.status = status.clone();
                let notification = ChildNotification {
                    owner: state.owner.clone(),
                    session_id: state.summary.session_id.clone(),
                    origin_session_code: state.origin_session_code.clone(),
                    parent_run_code: state.parent_run_code.clone(),
                    status: status.clone(),
                    run_id,
                    sequence: state.execution_sequence,
                    result: stored.clone(),
                };
                sender.send_replace(Some((status, stored)));
                notification
            };
            FrameworkHost(host_cell)
                .deliver_child_notification(notification)
                .await;
        });
        let _ = context;
        Ok(summary)
    }
}

struct ChildNotification {
    owner: crate::domain::agent::AgentCollaborationOwner,
    session_id: String,
    origin_session_code: String,
    parent_run_code: String,
    status: fw::ChildRunStatus,
    run_id: fw::RunId,
    sequence: u64,
    result: Result<fw::ChildRunResult, String>,
}

impl FrameworkHost {
    /// Notify the parent session that one detached child finished.
    ///
    /// Dude-owned children use the durable feedback outbox so an idle Dude is
    /// woken through its event system. Everything else steers the parent run
    /// directly while it is still executing; if the parent is gone the
    /// notification is dropped and the model recovers through child_wait.
    async fn deliver_child_notification(&self, notification: ChildNotification) {
        if notification.owner.kind == "DUDE" {
            let feedback = ThreadFeedback {
                event_key: format!(
                    "child:{}:{}:finished",
                    notification.run_id, notification.sequence
                ),
                owner: notification.owner,
                session_id: notification.session_id,
                origin_session_code: notification.origin_session_code,
                initiator_agent_code: None,
                status: thread_status(&notification.status),
                result: notification
                    .result
                    .as_ref()
                    .ok()
                    .map(|result| fw::ThreadResult::from_run(result.run_id, &result.result)),
                error: notification.result.err(),
                child_run_id: Some(notification.run_id),
            };
            if let Err(error) = self.enqueue_child_feedback(&feedback).await {
                tracing::error!(%error, "persist detached child feedback failed");
                return;
            }
            if let Err(error) = self.flush_thread_feedback().await {
                tracing::warn!(%error, "detached child feedback remains pending");
            }
            return;
        }
        let outcome = match &notification.result {
            Ok(result) if result.result.status == fw::RunStatus::AwaitingInput => {
                format!(
                    "等待调用方回答。用 child_wait 获取完整交互，再用 child_answer 回答。interactions={}",
                    serde_json::to_string(&result.result.interactions).unwrap_or_default()
                )
            }
            Ok(result) => format!("{}", result.result.output),
            Err(error) => error.clone(),
        };
        let message = format!(
            "后台子执行已结束（childRunId={}，status={}，agent={}）。输出摘要：{}。需要完整结果时用 child_wait 获取。",
            notification.run_id,
            serde_json::to_value(&notification.status)
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .unwrap_or_default(),
            notification.session_id,
            outcome
        );
        self.notify_native_run(&notification.parent_run_code, &message);
    }
    pub(super) async fn enqueue_child_feedback(
        &self,
        feedback: &ThreadFeedback,
    ) -> Result<(), fw::AgentError> {
        let payload = serde_json::to_string(feedback).map_err(failure)?;
        let event_key = feedback.event_key.clone();
        let session_id = feedback.session_id.clone();
        self.shared_database()
            .await?
            .call(move |connection| {
                connection
                    .execute(
                        "INSERT OR IGNORE INTO collaboration_thread_feedback(event_key,session_id,payload_json) VALUES (?1,?2,?3)",
                        rusqlite::params![event_key, session_id, payload],
                    )
                    .map_err(|e| fw::StateError::Backend(e.to_string()))?;
                Ok::<_, fw::StateError>(())
            })
            .await
            .map_err(failure)?;
        Ok(())
    }
    /// Host teardown stops every detached child: nothing may outlive the
    /// process that owns its in-memory execution.
    pub(in crate::application::agent::framework_runtime) fn cancel_detached_children(&self) {
        let children: Vec<_> = self
            .0
            .collaboration_children
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect();
        for child in children {
            let state = child.state.lock().unwrap();
            if state.summary.status == fw::ChildRunStatus::Running {
                state.control.cancel();
            }
        }
    }
}
