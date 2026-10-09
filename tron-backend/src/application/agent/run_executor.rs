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
            let result = tokio::spawn(async move {
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

#[cfg(test)]
mod tests {
    use std::{future::Future, pin::Pin, sync::Mutex};

    use serde_json::json;

    use crate::application::agent::{ModelClient, RuntimeAdapterKey, RuntimeCapabilities};
    use crate::domain::agent::{
        AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
        AgentRunLimits, EffectiveSkill, EffectiveTool, EffectiveVariable, ModelRequest,
        ModelResponse,
    };

    use super::{
        AgentInvocationError, AgentRunContext, AgentRunExecutionResult, AgentRunExecutor,
        AgentRunLifecycleState, AgentRunResult, AgentRuntime, ModelStreamSink,
    };

    #[derive(Clone)]
    struct FakeRuntime {
        key: RuntimeAdapterKey,
        capabilities: RuntimeCapabilities,
        results: std::sync::Arc<Mutex<Vec<Result<AgentRunResult, AgentInvocationError>>>>,
        calls: std::sync::Arc<Mutex<Vec<String>>>,
        delay_ms: u64,
    }

    impl FakeRuntime {
        fn new(
            key: RuntimeAdapterKey,
            capabilities: RuntimeCapabilities,
            results: Vec<Result<AgentRunResult, AgentInvocationError>>,
            delay_ms: u64,
        ) -> Self {
            Self {
                key,
                capabilities,
                results: std::sync::Arc::new(Mutex::new(results)),
                calls: std::sync::Arc::new(Mutex::new(Vec::new())),
                delay_ms,
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls
                .lock()
                .expect("fake runtime calls mutex poisoned")
                .clone()
        }

        fn pop_result(&self) -> Result<AgentRunResult, AgentInvocationError> {
            let mut guard = self
                .results
                .lock()
                .expect("fake runtime result mutex poisoned");
            if guard.is_empty() {
                return Err(AgentInvocationError::RuntimeFailed(
                    "no scripted runtime result".into(),
                ));
            }
            guard.remove(0)
        }
    }

    impl AgentRuntime for FakeRuntime {
        fn key(&self) -> RuntimeAdapterKey {
            self.key
        }

        fn capabilities(&self) -> RuntimeCapabilities {
            self.capabilities
        }

        fn start<'a>(
            &'a self,
            _context: AgentRunContext,
            _sink: Option<ModelStreamSink>,
        ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
        {
            Box::pin(async move {
                self.calls
                    .lock()
                    .expect("fake runtime calls mutex poisoned")
                    .push("start".into());
                if self.delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
                }
                self.pop_result()
            })
        }

        fn resume<'a>(
            &'a self,
            session_id: &'a str,
            _context: AgentRunContext,
            _sink: Option<ModelStreamSink>,
        ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
        {
            Box::pin(async move {
                self.calls
                    .lock()
                    .expect("fake runtime calls mutex poisoned")
                    .push(format!("resume:{session_id}"));
                if self.delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
                }
                self.pop_result()
            })
        }

        fn cancel<'a>(
            &'a self,
            run_code: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<(), AgentInvocationError>> + Send + 'a>> {
            Box::pin(async move {
                self.calls
                    .lock()
                    .expect("fake runtime calls mutex poisoned")
                    .push(format!("cancel:{run_code}"));
                Ok(())
            })
        }
    }

    #[derive(Clone)]
    struct NoopModelClient;

    impl ModelClient for NoopModelClient {
        fn generate_once<'a>(
            &'a self,
            config: &'a AgentModelConfig,
            request: &'a ModelRequest,
        ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
            self.generate(config, request)
        }
        fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
            Box::pin(async move {
                let _ = self.close_observed().await;
            })
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
        fn generate<'a>(
            &'a self,
            _config: &'a AgentModelConfig,
            _request: &'a ModelRequest,
        ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
            Box::pin(async move { Err("noop model client".into()) })
        }
    }

    fn run_executor() -> AgentRunExecutor {
        AgentRunExecutor::new(super::AgentRuntimeRegistry::new(
            std::sync::Arc::new(NoopModelClient),
            std::sync::Arc::new(crate::domain::agent::ToolRegistry::default()),
            crate::shared::event_bus::EventBusHandle::disabled(),
            reqwest::Client::new(),
        ))
    }

    fn scripted_run(run_code: &str, output: &str) -> AgentRunResult {
        AgentRunResult {
            run_code: run_code.into(),
            conversation_code: Some("CONV-1".into()),
            workspace_code: "TRON_CORE".into(),
            final_output: output.into(),
            step_count: 1,
            token_usage: None,
            latest_request_estimate: None,
            latest_usage_audit: None,
            system_agent_session_id: Some("ses-1".into()),
            observations: Vec::new(),
            native: None,
            sdk_execution: None,
            effective_model: None,
        }
    }

    fn context(max_retries: u32) -> AgentRunContext {
        AgentRunContext {
            agent_code: None,
        auto_model_route: None,
            conversation_code: Some("CONV-1".into()),
            run_code: "RUN-1".into(),
            workspace_code: "TRON_CORE".into(),
            user_code: Some("USR-1".into()),
            locale: "en-US".into(),
            request_id: Some("REQ-1".into()),
            stream_key: Some("CONV-1".into()),
            agent_kind: AgentKind::SystemCli,
            system_agent_session_id: None,
            model: AgentModelConfig {
                model_code: "MODEL-1".into(),
                protocol: AgentModelProtocol::ChatCompletions,
                base_url: "http://127.0.0.1:1/v1".into(),
                api_path: None,
                api_key: "sk-test".into(),
                model_name: "test-model".into(),
                thinking_enabled: false,
                reasoning_effort: None,
                responses_stateful_enabled: false,
                temperature: Some(0.2),
                max_tokens: Some(128),
                context_window_tokens: Some(8_192),
                top_p: Some(1.0),
                max_retries: Some(max_retries),
            },
            variables: vec![EffectiveVariable {
                key: "REGION".into(),
                value: "us".into(),
                source: "LOCAL".into(),
                sensitive: false,
            }],
            skills: vec![EffectiveSkill {
                skill_name: "planner".into(),
                description: "plan".into(),
                instruction: Some("think".into()),
                tool_names: vec!["echo".into()],
            }],
            tools: vec![EffectiveTool {
                tool_name: "echo".into(),
                domain_code: None,
                description: "echo".into(),
                input_schema: json!({ "type": "object" }),
                skill_name: Some("planner".into()),
                required_variable_keys: Vec::new(),
            }],
            effective_tools: Vec::new(),
            tool_exposure: Default::default(),
            execution_scope: AgentExecutionScope::default(),
            limits: AgentRunLimits::default(),
            messages: vec![AgentMessage::user("hello")],
            observations: Vec::new(),
            short_term_memory: None,
            started_at: "2026-05-30T00:00:00Z".into(),
        }
    }

    fn runtime_caps(resume: bool, cancel: bool) -> RuntimeCapabilities {
        RuntimeCapabilities {
            supports_resume: resume,
            supports_cancel: cancel,
            supports_steer: false,
            supports_streaming: true,
            supports_ephemeral: false,
            supports_workdir: true,
        }
    }

    fn assert_success(
        result: Result<AgentRunExecutionResult, AgentInvocationError>,
    ) -> AgentRunExecutionResult {
        result.expect("agent run execution should succeed")
    }

    #[tokio::test]
    async fn uses_resume_when_runtime_supports_resume() {
        let executor = run_executor();
        let runtime = std::sync::Arc::new(FakeRuntime::new(
            RuntimeAdapterKey::CodexExec,
            runtime_caps(true, false),
            vec![Ok(scripted_run("RUN-1", "ok"))],
            0,
        ));

        let result = assert_success(
            executor
                .execute_with_runtime(runtime.clone(), context(0), Some("thr-1".into()), None)
                .await,
        );

        assert_eq!(result.runtime_adapter_key, "CODEX_EXEC");
        assert_eq!(result.lifecycle_state, AgentRunLifecycleState::Completed);
        assert_eq!(
            result.lifecycle_trace,
            vec![
                AgentRunLifecycleState::Created,
                AgentRunLifecycleState::Running,
                AgentRunLifecycleState::Completed,
            ]
        );
        assert_eq!(result.attempts, 1);
        assert!(result.resumed);
        assert_eq!(runtime.calls(), vec!["resume:thr-1"]);
    }

    #[tokio::test]
    async fn falls_back_to_start_when_resume_is_not_supported() {
        let executor = run_executor();
        let runtime = std::sync::Arc::new(FakeRuntime::new(
            RuntimeAdapterKey::React,
            runtime_caps(false, false),
            vec![Ok(scripted_run("RUN-1", "ok"))],
            0,
        ));

        let result = assert_success(
            executor
                .execute_with_runtime(runtime.clone(), context(0), Some("thr-1".into()), None)
                .await,
        );

        assert_eq!(result.runtime_adapter_key, "REACT");
        assert_eq!(result.lifecycle_state, AgentRunLifecycleState::Completed);
        assert_eq!(
            result.lifecycle_trace,
            vec![
                AgentRunLifecycleState::Created,
                AgentRunLifecycleState::Running,
                AgentRunLifecycleState::Completed,
            ]
        );
        assert!(!result.resumed);
        assert_eq!(runtime.calls(), vec!["start"]);
    }

    #[tokio::test]
    async fn does_not_replay_a_whole_run_after_a_model_failure() {
        let executor = run_executor();
        let runtime = std::sync::Arc::new(FakeRuntime::new(
            RuntimeAdapterKey::OpenCodeRun,
            runtime_caps(false, false),
            vec![
                Err(AgentInvocationError::ModelFailed("transient".into())),
                Ok(scripted_run("RUN-1", "retried")),
            ],
            0,
        ));

        let result = executor
            .execute_with_runtime(runtime.clone(), context(3), None, None)
            .await;
        assert!(
            matches!(result, Err(AgentInvocationError::ModelFailed(message)) if message == "transient")
        );
        assert_eq!(runtime.calls(), vec!["start"]);
    }

    #[tokio::test]
    async fn slow_runtime_completes_without_cancel_when_supported() {
        let executor = run_executor();
        let runtime = std::sync::Arc::new(FakeRuntime::new(
            RuntimeAdapterKey::OpenCodeServe,
            runtime_caps(false, true),
            vec![Ok(scripted_run("RUN-1", "slow"))],
            30,
        ));

        let result = assert_success(
            executor
                .execute_with_runtime(runtime.clone(), context(0), None, None)
                .await,
        );
        assert_eq!(result.lifecycle_state, AgentRunLifecycleState::Completed);
        assert_eq!(runtime.calls(), vec!["start"]);
    }

    #[tokio::test]
    async fn pre_start_stop_of_reserved_run_maps_to_run_stopped() {
        let executor = run_executor();
        let active_runs = executor.runtime_registry.active_runs();
        let reservation = active_runs
            .reserve("CONV-1", "RUN-1", "CODEX_EXEC", false)
            .expect("run admission should reserve the conversation");
        active_runs
            .request_stop("CONV-1", "RUN-1")
            .expect("pre-start stop should be accepted");
        let runtime = std::sync::Arc::new(FakeRuntime::new(
            RuntimeAdapterKey::CodexExec,
            runtime_caps(false, false),
            vec![Ok(scripted_run("RUN-1", "must not run"))],
            0,
        ));

        let error = executor
            .execute_with_runtime_admission(
                runtime.clone(),
                context(0),
                None,
                None,
                Some(reservation),
            )
            .await
            .expect_err("stopped admission must fail as stopped");

        assert_eq!(error, AgentInvocationError::RunStopped);
        assert!(
            runtime.calls().is_empty(),
            "runtime must not start after stop"
        );
    }
}
