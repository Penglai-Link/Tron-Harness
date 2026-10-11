//! Process-local thread bindings and result ownership. Native sessions use shared storage.
use super::*;
use std::sync::RwLock;
use tokio::sync::watch;

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
}

impl Runtime {
    fn thread(&self, id: &str) -> Result<Arc<Thread>, fw::AgentError> {
        let host = self.host()?;
        let thread = host
            .0
            .collaboration_threads
            .read()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| failure("thread not found"))?;
        if thread.context.workspace_code != self.context.workspace_code
            || thread.context.user_code != self.context.user_code
        {
            return Err(failure("thread not found"));
        }
        self.authorize(
            thread
                .context
                .agent_code
                .as_deref()
                .ok_or_else(|| failure("thread target missing"))?,
        )?;
        Ok(thread)
    }
    fn ensure_registered(&self, id: &str, expected: &Arc<Thread>) -> Result<(), fw::AgentError> {
        if !Arc::ptr_eq(&self.thread(id)?, expected) {
            return Err(failure("thread not found"));
        }
        Ok(())
    }
    async fn launch(
        &self,
        thread: Arc<Thread>,
        request: fw::RunRequest,
        parent: &fw::RunContext,
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
        let handle = thread.agent.start(request)?;
        let run_id = handle.run_id;
        let (sender, receiver) = watch::channel(None);
        let summary = {
            let mut state = thread.state.lock().unwrap();
            state.summary.status = fw::ThreadStatus::Running;
            state.summary.latest_run_id = Some(run_id);
            state.completion = receiver;
            state.origin_execution_id = parent.execution_id();
            state.summary.clone()
        };
        parent
            .emit(fw::AgentEvent::Collaboration(Box::new(
                fw::CollaborationEvent::ThreadRunStarted {
                    thread_id: summary.thread_id.clone(),
                    run_id,
                    cause,
                    interaction_id,
                },
            )))
            .await;
        let origin = parent.clone();
        let returned = summary.clone();
        crate::app::telemetry::spawn_traced(async move {
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
            thread.state.lock().unwrap().summary.status = status.clone();
            origin
                .emit(fw::AgentEvent::Collaboration(Box::new(
                    fw::CollaborationEvent::ThreadRunFinished {
                        thread_id: summary.thread_id,
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
            sender.send_replace(Some((status, result.map_err(|error| error.to_string()))));
        });
        Ok(returned)
    }
}

#[async_trait::async_trait]
impl AgentCollaborationRuntime for Runtime {
    async fn spawn_child(
        &self,
        request: fw::RunRequest,
        parent: &fw::RunContext,
    ) -> Result<fw::ChildRunResult, fw::AgentError> {
        let session = format!("tool-child-{}", fw::RunId::new_v4());
        let (agent, mut request, prepared) = self
            .prepare(
                request,
                session.clone(),
                None,
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
                session_id: session,
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
        Ok(fw::ChildRunResult {
            run_id,
            result: result?,
        })
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
        let id = format!("thread-{}", fw::RunId::new_v4());
        let (agent, request, prepared) = self
            .prepare(request, id.clone(), None, parent, ExecutionLimits::Request)
            .await?;
        let (_, completion) = watch::channel(None);
        let thread = Arc::new(Thread {
            context: prepared.clone(),
            agent,
            transition: Mutex::new(()),
            state: std::sync::Mutex::new(ThreadState {
                summary: fw::ThreadSummary {
                    thread_id: id.clone(),
                    title,
                    status: fw::ThreadStatus::Running,
                    latest_run_id: request.run_id,
                },
                completion,
                origin_execution_id: parent.execution_id(),
            }),
        });
        let host = self.host()?;
        {
            let mut threads = host.0.collaboration_threads.write().unwrap();
            if threads.len() >= 128 {
                return Err(failure(
                    "thread capacity exceeded; close completed threads first",
                ));
            }
            threads.insert(id.clone(), thread.clone());
        }
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
                parent,
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
        let thread = self.thread(id)?;
        let _transition = thread.transition.lock().await;
        self.ensure_registered(id, &thread)?;
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
            .prepare(
                request,
                id.into(),
                Some(&thread.context),
                parent,
                ExecutionLimits::Request,
            )
            .await?;
        self.launch(
            thread.clone(),
            request,
            parent,
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
        let thread = self.thread(id)?;
        let _transition = thread.transition.lock().await;
        self.ensure_registered(id, &thread)?;
        if thread.state.lock().unwrap().summary.status != fw::ThreadStatus::AwaitingInput {
            return Err(failure("thread is not awaiting input"));
        }
        let request = thread.agent.answer(id, interaction, value).await?;
        let (_, request, _) = self
            .prepare(
                request,
                id.into(),
                Some(&thread.context),
                parent,
                ExecutionLimits::Request,
            )
            .await?;
        self.launch(
            thread.clone(),
            request,
            parent,
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
        let thread = self.thread(id)?;
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
                self.thread(id)?;
                if parent.execution_id() != origin_execution_id {
                    let run_id = run_id
                        .or_else(|| result.as_ref().ok().map(|r| r.run_id))
                        .or_else(|| thread.state.lock().unwrap().summary.latest_run_id)
                        .ok_or_else(|| failure("thread has no run"))?;
                    parent
                        .emit(fw::AgentEvent::Collaboration(Box::new(
                            fw::CollaborationEvent::ThreadRunFinished {
                                thread_id: id.into(),
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
        let thread = self.thread(id)?;
        let mut conversation = thread
            .agent
            .conversation(&fw::ConversationKey::new(id))
            .await?;
        if conversation.messages.len() > limit {
            conversation.messages = conversation
                .messages
                .split_off(conversation.messages.len() - limit);
        }
        self.thread(id)?;
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
            .0
            .collaboration_threads
            .read()
            .unwrap()
            .values()
            .filter(|thread| {
                thread.context.workspace_code == self.context.workspace_code
                    && thread.context.user_code == self.context.user_code
                    && thread
                        .context
                        .agent_code
                        .as_ref()
                        .is_some_and(|code| authorized.contains(code))
            })
            .map(|thread| thread.state.lock().unwrap().summary.clone())
            .collect())
    }
    async fn close_thread(&self, id: &str, parent: &fw::RunContext) -> Result<(), fw::AgentError> {
        let thread = self.thread(id)?;
        let _transition = thread.transition.lock().await;
        self.ensure_registered(id, &thread)?;
        if thread.state.lock().unwrap().summary.status == fw::ThreadStatus::Running {
            return Err(fw::AgentError::SessionBusy(id.into()));
        }
        self.host()?
            .0
            .collaboration_threads
            .write()
            .unwrap()
            .remove(id);
        parent
            .emit(fw::AgentEvent::Collaboration(Box::new(
                fw::CollaborationEvent::ThreadClosed {
                    thread_id: id.into(),
                },
            )))
            .await;
        Ok(())
    }
}
