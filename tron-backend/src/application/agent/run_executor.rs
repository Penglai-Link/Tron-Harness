use std::sync::Arc;

use crate::domain::agent::{AgentRunContext, AgentRunResult};

use super::{
    ActiveRunGuard, AgentInvocationError, AgentRuntime, AgentRuntimeRegistry, ModelStreamSink,
    RuntimeAdapterSelection, run_cancellation::cancel_runtime_after_stop,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRunLifecycleState {
    Created,
    Running,
    Completed,
    AwaitingInput,
    StepLimit,
    ToolCallLimit,
    Failed,
    Cancelling,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct AgentRunExecutionResult {
    pub run: AgentRunResult,
    pub runtime_adapter_key: String,
    pub lifecycle_state: AgentRunLifecycleState,
    pub lifecycle_trace: Vec<AgentRunLifecycleState>,
    pub attempts: u32,
    pub resumed: bool,
}

#[derive(Clone)]
pub struct AgentRunExecutor {
    runtime_registry: AgentRuntimeRegistry,
}

impl AgentRunExecutor {
    pub fn new(runtime_registry: AgentRuntimeRegistry) -> Self {
        Self { runtime_registry }
    }

    pub fn resolve_runtime(&self, selection: RuntimeAdapterSelection) -> Arc<dyn AgentRuntime> {
        self.runtime_registry.resolve(selection)
    }

    pub async fn execute_with_selection(
        &self,
        selection: RuntimeAdapterSelection,
        context: AgentRunContext,
        requested_session_id: Option<String>,
        sink: Option<ModelStreamSink>,
        reservation: Option<ActiveRunGuard>,
    ) -> Result<AgentRunExecutionResult, AgentInvocationError> {
        let runtime = self.resolve_runtime(selection);
        self.execute_with_runtime_admission(
            runtime,
            context,
            requested_session_id,
            sink,
            reservation,
        )
        .await
    }

    pub async fn execute_with_runtime(
        &self,
        runtime: Arc<dyn AgentRuntime>,
        context: AgentRunContext,
        requested_session_id: Option<String>,
        sink: Option<ModelStreamSink>,
    ) -> Result<AgentRunExecutionResult, AgentInvocationError> {
        self.execute_with_runtime_admission(runtime, context, requested_session_id, sink, None)
            .await
    }

    async fn execute_with_runtime_admission(
        &self,
        runtime: Arc<dyn AgentRuntime>,
        context: AgentRunContext,
        requested_session_id: Option<String>,
        sink: Option<ModelStreamSink>,
        reservation: Option<ActiveRunGuard>,
    ) -> Result<AgentRunExecutionResult, AgentInvocationError> {
        let runtime_adapter_key = runtime.key().code().to_string();
        let active_runs = self.runtime_registry.active_runs();
        let active_run_guard = match reservation {
            Some(guard) => {
                let session_code = context.conversation_code.as_deref().ok_or_else(|| {
                    AgentInvocationError::RuntimeFailed(
                        "reserved run requires conversationCode".into(),
                    )
                })?;
                active_runs
                    .activate_reservation(
                        &guard,
                        session_code,
                        &context.run_code,
                        &runtime_adapter_key,
                        runtime.capabilities().supports_steer,
                    )
                    .map_err(|error| match error {
                        super::ActiveRunError::RunFinishing => AgentInvocationError::RunStopped,
                        super::ActiveRunError::AlreadyActive => {
                            AgentInvocationError::SessionBusy(session_code.into())
                        }
                        error => AgentInvocationError::RuntimeFailed(error.to_string()),
                    })?;
                Some(guard)
            }
            None => context
                .conversation_code
                .as_deref()
                .map(|session_code| {
                    active_runs.register(
                        session_code,
                        &context.run_code,
                        &runtime_adapter_key,
                        runtime.capabilities().supports_steer,
                    )
                })
                .transpose()
                .map_err(|error| match error {
                    super::ActiveRunError::AlreadyActive => AgentInvocationError::SessionBusy(
                        context
                            .conversation_code
                            .clone()
                            .unwrap_or_else(|| context.run_code.clone()),
                    ),
                    error => AgentInvocationError::RuntimeFailed(error.to_string()),
                })?,
        };
        let native = runtime.owns_native_lifecycle();
        let (caller_cancel, mut caller_cancelled) = tokio::sync::oneshot::channel();
        let execution = async move {
            let mut cancellation = active_run_guard.as_ref().map(|guard| guard.cancellation());
            let _admission = active_run_guard;
            let resumed = requested_session_id.is_some() && runtime.capabilities().supports_resume;
            let mut lifecycle_trace = vec![AgentRunLifecycleState::Created];

            lifecycle_trace.push(AgentRunLifecycleState::Running);
            let run_future = match requested_session_id.as_deref() {
                Some(session_id) if runtime.capabilities().supports_resume => {
                    runtime.resume(session_id, context.clone(), sink.clone())
                }
                _ => runtime.start(context.clone(), sink.clone()),
            };

            tokio::pin!(run_future);
            let stopped = async {
                match cancellation.as_mut() {
                    Some(cancellation) => {
                        tokio::select! { _ = cancellation.cancelled() => {}, _ = &mut caller_cancelled => {} }
                    }
                    None => {
                        let _ = caller_cancelled.await;
                    }
                }
            };
            let run_result = tokio::select! {
                    biased;
                    // Poll the native adapter first so its cancellation registration exists.
                    result = &mut run_future => result,
                    _ = stopped => {
                        lifecycle_trace.push(AgentRunLifecycleState::Cancelling);
                        cancel_runtime_after_stop(&runtime, &context.run_code).await;
                        if runtime.owns_native_lifecycle() {
                            // SDK wait includes hooks, observers and session/resource release.
                            let _ = run_future.await;
                        }
                        lifecycle_trace.push(AgentRunLifecycleState::Cancelled);
                        return Err(AgentInvocationError::RunStopped);
                    }
            };

            match run_result {
                Ok(run) => {
                    let state = match run.native.as_ref().map(|native| native.status) {
                        Some(crate::domain::agent::AgentNativeRunStatus::AwaitingInput) => {
                            AgentRunLifecycleState::AwaitingInput
                        }
                        Some(crate::domain::agent::AgentNativeRunStatus::StepLimit) => {
                            AgentRunLifecycleState::StepLimit
                        }
                        Some(crate::domain::agent::AgentNativeRunStatus::ToolCallLimit) => {
                            AgentRunLifecycleState::ToolCallLimit
                        }
                        _ => AgentRunLifecycleState::Completed,
                    };
                    lifecycle_trace.push(state);
                    return Ok(AgentRunExecutionResult {
                        run,
                        runtime_adapter_key,
                        lifecycle_state: state,
                        lifecycle_trace,
                        attempts: 1,
                        resumed,
                    });
                }
                Err(error) => {
                    lifecycle_trace.push(AgentRunLifecycleState::Failed);
                    return Err(error);
                }
            }
        };
        if native {
            let mut caller = CallerCancellation(Some(caller_cancel));
            let parent = eliteforge_agent_framework::current_continuation_parent();
            let result = crate::app::telemetry::spawn_traced(async move {
                if let Some(parent) = parent {
                    eliteforge_agent_framework::with_continuation_parent(&parent, execution).await
                } else {
                    execution.await
                }
            })
            .await
            .map_err(|error| {
                AgentInvocationError::RuntimeFailed(format!(
                    "native execution owner failed: {error}"
                ))
            })?;
            caller.0.take();
            result
        } else {
            // The receiver remains pending for existing non-SDK adapters.
            let _caller = caller_cancel;
            execution.await
        }
    }
}

struct CallerCancellation(Option<tokio::sync::oneshot::Sender<()>>);
impl Drop for CallerCancellation {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}
