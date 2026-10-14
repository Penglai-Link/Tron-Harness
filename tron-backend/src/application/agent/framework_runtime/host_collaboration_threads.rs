//! Runtime bindings over durable Thread identities. Native sessions use shared storage.
use super::*;
use std::sync::RwLock;
use tokio::sync::watch;
#[path = "host_collaboration_thread_store.rs"]
mod store;
use store::{ThreadRecord, owner};

#[path = "host_collaboration_thread_feedback.rs"]
mod feedback;
#[path = "host_collaboration_thread_intervention.rs"]
mod intervention;
pub use feedback::{DudeThreadCatalogItem, ThreadFeedback, ThreadFeedbackSink};

pub(in crate::application::agent::framework_runtime::host) type Threads =
    RwLock<BTreeMap<String, Arc<Thread>>>;
type Completion = (fw::ThreadStatus, Result<fw::ThreadResult, String>);
pub(in crate::application::agent::framework_runtime::host) struct Thread {
    context: AgentRunContext,
    agent: Agent,
    transition: Mutex<()>,
    state: std::sync::Mutex<ThreadState>,
}
struct ThreadState {
    summary: fw::ThreadSummary,
    completion: watch::Receiver<Option<Completion>>,
    origin_execution_id: fw::RunId,
    record: ThreadRecord,
}

impl Runtime {
    async fn thread(&self, id: &str) -> Result<Arc<Thread>, fw::AgentError> {
        let host = self.host()?;
        let cached = host
            .0
            .collaboration_threads
            .read()
            .unwrap()
            .get(id)
            .cloned();
        if let Some(thread) = cached {
            let authorized = {
                let state = thread.state.lock().unwrap();
                !state.record.closed && state.record.authorized(&self.context)
            };
            if !authorized {
                return Err(failure("thread not found"));
            }
            self.authorize(
                thread
                    .context
                    .agent_code
                    .as_deref()
                    .ok_or_else(|| failure("thread target missing"))?,
            )?;
            return Ok(thread);
        }
        let record = host
            .thread_record(id)
            .await?
            .filter(|record| !record.closed && record.authorized(&self.context))
            .ok_or_else(|| failure("thread not found"))?;
        self.authorize(&record.agent_code)?;
        let mut context = self.context.clone();
        context.agent_code = Some(record.agent_code.clone());
        context.conversation_code = Some(id.into());
        context.stream_key = Some(id.into());
        context.execution_scope = record.execution_scope.clone();
        context.collaboration_owner = Some(record.owner.clone());
        let agent = host
            .collaboration_agent(
                &record.workspace_code,
                &record.agent_code,
                self.builder.agent_management(),
            )
            .await?;
        let (_, completion) = watch::channel(record.completion.clone());
        let thread = Arc::new(Thread {
            context,
            agent,
            transition: Mutex::new(()),
            state: std::sync::Mutex::new(ThreadState {
                summary: record.summary.clone(),
                completion,
                origin_execution_id: fw::RunId::nil(),
                record,
            }),
        });
        let thread = host
            .0
            .collaboration_threads
            .write()
            .unwrap()
            .entry(id.into())
            .or_insert_with(|| thread.clone())
            .clone();
        Ok(thread)
    }
    async fn ensure_registered(
        &self,
        id: &str,
        expected: &Arc<Thread>,
    ) -> Result<(), fw::AgentError> {
        if !Arc::ptr_eq(&self.thread(id).await?, expected) {
            return Err(failure("thread not found"));
        }
        Ok(())
    }
    async fn launch(
        &self,
        thread: Arc<Thread>,
        request: fw::RunRequest,
        parent: Option<&fw::RunContext>,
        cause: fw::ThreadRunCause,
        interaction_id: Option<fw::RunId>,
    ) -> Result<fw::ThreadSummary, fw::AgentError> {
        let host = self.host()?;
        let _admission = host.0.config_lock.lock().await;
        host.ensure_open().map_err(failure)?;
        self.authorize(
            thread
                .context
                .agent_code
                .as_deref()
                .ok_or_else(|| failure("thread target missing"))?,
        )?;
        let run_id = request.run_id.unwrap_or_else(fw::RunId::new_v4);
        let mut request = request;
        request.run_id = Some(run_id);
        let mut record = thread.state.lock().unwrap().record.clone();
        record.execution_sequence = record
            .execution_sequence
            .checked_add(1)
            .ok_or_else(|| failure("Thread execution sequence overflow"))?;
        record.run_admitted_at_ms = fw::lifecycle_time_ms();
        record.resumed_interaction_id = interaction_id;
        record.summary.status = fw::ThreadStatus::Running;
        record.summary.latest_run_id = Some(run_id);
        record.completion = None;
        record.feedback_pending = false;
        host.persist_thread(&mut record, false).await?;
        thread.state.lock().unwrap().record = record;
        let handle = match thread.agent.start(request) {
            Ok(handle) => handle,
            Err(error) => {
                let mut record = thread.state.lock().unwrap().record.clone();
                record.summary.status = fw::ThreadStatus::Failed;
                record.completion = Some((fw::ThreadStatus::Failed, Err(error.to_string())));
                record.feedback_pending = record.owner.kind == "DUDE";
                host.persist_thread(&mut record, false).await?;
                let mut state = thread.state.lock().unwrap();
                state.summary = record.summary.clone();
                let (_, completion) = watch::channel(record.completion.clone());
                state.completion = completion;
                state.record = record;
                return Err(error);
            }
        };
        let (sender, receiver) = watch::channel(None);
        let summary = {
            let mut state = thread.state.lock().unwrap();
            state.summary.status = fw::ThreadStatus::Running;
            state.summary.latest_run_id = Some(run_id);
            state.completion = receiver;
            state.origin_execution_id =
                parent.map_or_else(fw::RunId::nil, fw::RunContext::execution_id);
            state.summary.clone()
        };
        let origin = parent.cloned();
        let returned = summary.clone();
        let feedback_host = host.clone();
        let execution_sequence = thread.state.lock().unwrap().record.execution_sequence;
        // Move the RunHandle into its background owner before another await.
        // Cancelling the initiating tool/Run must not cancel this independent Thread.
        crate::app::telemetry::spawn_traced(async move {
            if let Some(origin) = &origin {
                origin
                    .emit(fw::AgentEvent::Collaboration(Box::new(
                        fw::CollaborationEvent::ThreadRunStarted {
                            session_id: summary.session_id.clone(),
                            run_id,
                            cause,
                            interaction_id,
                        },
                    )))
                    .await;
            }
            let result = thread
                .agent
                .wait(handle)
                .await
                .map(|result| fw::ThreadResult::from_run(run_id, &result));
            let status = match &result {
                Ok(result) => fw::ThreadStatus::from(&result.status),
                Err(error) if matches!(error.primary(), fw::AgentError::Cancelled) => {
                    fw::ThreadStatus::Cancelled
                }
                Err(_) => fw::ThreadStatus::Failed,
            };
            let _transition = thread.transition.lock().await;
            let mut record = thread.state.lock().unwrap().record.clone();
            if record.execution_sequence == execution_sequence
                && record.summary.latest_run_id == Some(run_id)
            {
                record.summary.status = status.clone();
                record.completion = Some((
                    status.clone(),
                    result
                        .as_ref()
                        .map(Clone::clone)
                        .map_err(ToString::to_string),
                ));
                record.feedback_pending = record.owner.kind == "DUDE";
                // Completion and pending feedback are one durable write. Sink failure is retried.
                if let Err(error) = feedback_host.persist_thread(&mut record, false).await {
                    tracing::error!(session_id = %record.summary.session_id, %error, "persist Thread completion failed; native run remains authoritative");
                } else {
                    let mut state = thread.state.lock().unwrap();
                    state.summary.status = status.clone();
                    state.record = record;
                }
            }
            if let Err(error) = feedback_host.flush_thread_feedback().await {
                tracing::warn!(%error, "Thread feedback remains pending");
            }
            if let Some(origin) = &origin {
                origin
                    .emit(fw::AgentEvent::Collaboration(Box::new(
                        fw::CollaborationEvent::ThreadRunFinished {
                            session_id: summary.session_id,
                            run_id,
                            status: status.clone(),
                            interactions: result
                                .as_ref()
                                .map(|r| r.interactions.clone())
                                .unwrap_or_default(),
                            error: result.as_ref().err().map(ToString::to_string),
                        },
                    )))
                    .await;
            }
            sender.send_replace(Some((status, result.map_err(|error| error.to_string()))));
        });
        Ok(returned)
    }
}

#[async_trait::async_trait]
impl AgentCollaborationRuntime for Runtime {
    async fn agent_details(&self, code: &str, parent: &fw::RunContext) -> Result<serde_json::Value, fw::AgentError> {
        self.read_agent_details(code, parent).await
    }

    async fn spawn_child(
        &self,
        request: fw::RunRequest,
        parent: &fw::RunContext,
    ) -> Result<fw::ChildRunResult, fw::AgentError> {
        let session = format!("tool-child-{}", fw::RunId::new_v4());
        let (agent, mut request, prepared) = self
            .prepare_new(
                request,
                session.clone(),
                parent,
                ExecutionLimits::AgentDefinition,
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
        self.authorize(
            prepared
                .agent_code
                .as_deref()
                .ok_or_else(|| failure("child target missing"))?,
        )?;
        let (child, stream) = agent
            .start_child_with_output_owned::<eliteforge_agent_official_plugin_agui::AgUiStream>(
                request, parent,
            )
            .await?;
        drop(admission);
        let run_id = child.run_id();
        host.record_subagent(
            &owner,
            super::super::SubagentRecord {
                run_id,
                session_id: session.clone(),
                parent_run_id: parent.run_id,
                agent_code: code,
                agent_name: name,
                avatar_id,
                task,
                user_code: prepared.user_code.clone(),
            },
        )
        .await?;
        let presentation = host.presentation_service().map_err(failure)?;
        let (result, ()) = tokio::join!(
            child.wait(),
            super::super::super::presentation::consume_native_agui(stream, &prepared, presentation)
        );
        let mut result = result?;
        if result.status == fw::RunStatus::AwaitingInput
            && prepared
                .collaboration_owner
                .as_ref()
                .is_some_and(|owner| owner.kind == "DUDE")
        {
            // A native approval/interaction may pause even a bounded child. Promote
            // that actual native session to a durable Thread, never fabricate a rerun.
            let mut record = ThreadRecord {
                summary: fw::ThreadSummary {
                    session_id: session.clone(),
                    title: None,
                    status: fw::ThreadStatus::AwaitingInput,
                    latest_run_id: Some(run_id),
                },
                workspace_code: prepared.workspace_code.clone(),
                user_code: prepared.user_code.clone(),
                agent_code: prepared
                    .agent_code
                    .clone()
                    .ok_or_else(|| failure("child target missing"))?,
                owner: store::owner(&prepared),
                initiator_agent_code: self.context.agent_code.clone(),
                origin_session_code: prepared
                    .collaboration_owner
                    .as_ref()
                    .and_then(|owner| owner.origin_session_code.clone())
                    .unwrap_or_else(|| parent.session_id.clone()),
                execution_scope: prepared.execution_scope.clone(),
                completion: Some((
                    fw::ThreadStatus::AwaitingInput,
                    Ok(fw::ThreadResult::from_run(run_id, &result)),
                )),
                closed: false,
                feedback_pending: true,
                revision: 0,
                run_admitted_at_ms: fw::lifecycle_time_ms(),
                execution_sequence: 1,
                resumed_interaction_id: None,
            };
            host.persist_thread(&mut record, true).await?;
            if let Err(error) = host.flush_thread_feedback().await {
                tracing::warn!(%error, "Spawn interaction feedback remains pending");
            }
            result.output.push_str(&format!("\nThis child is awaiting input in durable Thread {session}. Use thread_read/thread_answer with this session_id and the actual interaction ID; do not start a replacement execution."));
        }
        Ok(fw::ChildRunResult { run_id, result })
    }
    async fn create_thread(
        &self,
        request: fw::RunRequest,
        title: Option<String>,
        parent: &fw::RunContext,
    ) -> Result<fw::ThreadSummary, fw::AgentError> {
        if title
            .as_ref()
            .is_some_and(|s| s.trim().is_empty() || s.len() > 200)
        {
            return Err(failure("invalid thread title"));
        }
        let id = fw::RunId::new_v4().to_string();
        let (agent, request, prepared) = self
            .prepare_new(request, id.clone(), parent, ExecutionLimits::Request)
            .await?;
        let (_, completion) = watch::channel(None);
        let mut record = ThreadRecord {
            summary: fw::ThreadSummary {
                session_id: id.clone(),
                title: title.clone(),
                status: fw::ThreadStatus::Running,
                latest_run_id: request.run_id,
            },
            workspace_code: prepared.workspace_code.clone(),
            user_code: prepared.user_code.clone(),
            agent_code: prepared
                .agent_code
                .clone()
                .ok_or_else(|| failure("thread target missing"))?,
            owner: owner(&self.context),
            initiator_agent_code: self.context.agent_code.clone(),
            origin_session_code: self
                .context
                .collaboration_owner
                .as_ref()
                .and_then(|owner| owner.origin_session_code.clone())
                .unwrap_or_else(|| parent.session_id.clone()),
            execution_scope: prepared.execution_scope.clone(),
            completion: None,
            closed: false,
            feedback_pending: false,
            revision: 0,
            run_admitted_at_ms: fw::lifecycle_time_ms(),
            execution_sequence: 0,
            resumed_interaction_id: None,
        };
        let host = self.host()?;
        let admission = host.0.config_lock.lock().await;
        if host.0.collaboration_threads.read().unwrap().len() >= 128 {
            return Err(failure(
                "thread capacity exceeded; close completed threads first",
            ));
        }
        host.persist_thread(&mut record, true).await?;
        let thread = Arc::new(Thread {
            context: prepared.clone(),
            agent,
            transition: Mutex::new(()),
            state: std::sync::Mutex::new(ThreadState {
                summary: fw::ThreadSummary {
                    session_id: id.clone(),
                    title,
                    status: fw::ThreadStatus::Running,
                    latest_run_id: request.run_id,
                },
                completion,
                origin_execution_id: parent.execution_id(),
                record,
            }),
        });
        host.0
            .collaboration_threads
            .write()
            .unwrap()
            .insert(id.clone(), thread.clone());
        drop(admission);
        let _transition = thread.transition.lock().await;
        let created = thread.state.lock().unwrap().summary.clone();
        parent
            .emit(fw::AgentEvent::Collaboration(Box::new(
                fw::CollaborationEvent::ThreadCreated { summary: created },
            )))
            .await;
        match self
            .launch(
                thread.clone(),
                request,
                Some(parent),
                fw::ThreadRunCause::Created,
                None,
            )
            .await
        {
            Ok(summary) => Ok(summary),
            Err(error) => {
                host.0.collaboration_threads.write().unwrap().remove(&id);
                Err(error)
            }
        }
    }
    async fn send_thread(
        &self,
        id: &str,
        request: fw::RunRequest,
        parent: &fw::RunContext,
    ) -> Result<fw::ThreadSummary, fw::AgentError> {
        let thread = self.thread(id).await?;
        let _transition = thread.transition.lock().await;
        self.ensure_registered(id, &thread).await?;
        {
            let state = thread.state.lock().unwrap();
            if state.summary.status == fw::ThreadStatus::Running {
                return Err(fw::AgentError::SessionBusy(id.into()));
            }
            if state.summary.status == fw::ThreadStatus::AwaitingInput {
                return Err(failure("thread is awaiting input; use thread_answer"));
            }
        }
        if request.agent_code.is_some() {
            return Err(failure("thread target cannot be changed"));
        }
        let (_, request, _) = self
            .prepare_existing(
                request,
                id.into(),
                &thread.context,
                parent,
                ExecutionLimits::Request,
            )
            .await?;
        self.launch(
            thread.clone(),
            request,
            Some(parent),
            fw::ThreadRunCause::Message,
            None,
        )
        .await
    }
    async fn answer_thread(
        &self,
        id: &str,
        interaction: fw::RunId,
        value: serde_json::Value,
        parent: &fw::RunContext,
    ) -> Result<fw::ThreadSummary, fw::AgentError> {
        let thread = self.thread(id).await?;
        let _transition = thread.transition.lock().await;
        self.ensure_registered(id, &thread).await?;
        if thread.state.lock().unwrap().summary.status != fw::ThreadStatus::AwaitingInput {
            return Err(failure("thread is not awaiting input"));
        }
        let request = thread.agent.answer(id, interaction, value).await?;
        let (_, request, _) = self
            .prepare_existing(
                request,
                id.into(),
                &thread.context,
                parent,
                ExecutionLimits::Request,
            )
            .await?;
        self.launch(
            thread.clone(),
            request,
            Some(parent),
            fw::ThreadRunCause::InteractionResponse,
            Some(interaction),
        )
        .await
    }
    async fn wait_thread(
        &self,
        id: &str,
        run_id: Option<fw::RunId>,
        parent: &fw::RunContext,
    ) -> Result<fw::ThreadResult, fw::AgentError> {
        let thread = self.thread(id).await?;
        let (mut completion, origin_execution_id) = {
            let state = thread.state.lock().unwrap();
            if run_id.is_some_and(|id| Some(id) != state.summary.latest_run_id) {
                return Err(failure("requested run is not the latest thread run"));
            }
            (state.completion.clone(), state.origin_execution_id)
        };
        loop {
            let completed = completion.borrow_and_update().clone();
            if let Some((status, result)) = completed {
                self.thread(id).await?;
                if parent.execution_id() != origin_execution_id {
                    let run_id = run_id
                        .or_else(|| result.as_ref().ok().map(|r| r.run_id))
                        .or_else(|| thread.state.lock().unwrap().summary.latest_run_id)
                        .ok_or_else(|| failure("thread has no run"))?;
                    parent
                        .emit(fw::AgentEvent::Collaboration(Box::new(
                            fw::CollaborationEvent::ThreadRunFinished {
                                session_id: id.into(),
                                run_id,
                                status,
                                interactions: result
                                    .as_ref()
                                    .map(|r| r.interactions.clone())
                                    .unwrap_or_default(),
                                error: result.as_ref().err().cloned(),
                            },
                        )))
                        .await;
                }
                return result.map_err(fw::AgentError::Run);
            }
            completion
                .changed()
                .await
                .map_err(|_| failure("thread execution owner terminated"))?;
        }
    }
    async fn read_thread(&self, id: &str, limit: usize) -> Result<fw::ThreadView, fw::AgentError> {
        let thread = self.thread(id).await?;
        let mut conversation = thread
            .agent
            .conversation(&fw::ConversationKey::new(id))
            .await?;
        if conversation.messages.len() > limit {
            conversation.messages = conversation
                .messages
                .split_off(conversation.messages.len() - limit);
        }
        self.thread(id).await?;
        let summary = thread.state.lock().unwrap().summary.clone();
        Ok(fw::ThreadView {
            summary,
            conversation,
        })
    }
    async fn list_threads(
        &self,
        _parent: &fw::RunContext,
    ) -> Result<Vec<fw::ThreadSummary>, fw::AgentError> {
        let host = self.host()?;
        let authorized: std::collections::HashSet<_> = self
            .directory()?
            .into_iter()
            .map(|entry| entry.agent_code)
            .collect();
        Ok(host
            .thread_records()
            .await?
            .into_iter()
            .filter(|record| {
                !record.closed
                    && record.authorized(&self.context)
                    && authorized.contains(&record.agent_code)
            })
            .map(|record| record.summary)
            .collect())
    }
    async fn close_thread(&self, id: &str, parent: &fw::RunContext) -> Result<(), fw::AgentError> {
        let thread = self.thread(id).await?;
        let _transition = thread.transition.lock().await;
        self.ensure_registered(id, &thread).await?;
        if thread.state.lock().unwrap().summary.status == fw::ThreadStatus::Running {
            return Err(fw::AgentError::SessionBusy(id.into()));
        }
        let host = self.host()?;
        let mut record = thread.state.lock().unwrap().record.clone();
        record.closed = true;
        host.persist_thread(&mut record, false).await?;
        thread.state.lock().unwrap().record = record;
        host.0.collaboration_threads.write().unwrap().remove(id);
        parent
            .emit(fw::AgentEvent::Collaboration(Box::new(
                fw::CollaborationEvent::ThreadClosed {
                    session_id: id.into(),
                },
            )))
            .await;
        Ok(())
    }
}

#[cfg(test)]
#[path = "host_collaboration_thread_tests.rs"]
mod tests;
