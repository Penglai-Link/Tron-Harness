use super::*;
use crate::application::agent::AgentRunLifecycleState;
struct ResultRuntime {
    status: Option<AgentNativeRunStatus>,
    calls: Arc<AtomicUsize>,
}
impl AgentRuntime for ResultRuntime {
    fn key(&self) -> RuntimeAdapterKey {
        RuntimeAdapterKey::React
    }
    fn capabilities(&self) -> RuntimeCapabilities {
        RuntimeAdapterKey::React.descriptor().capabilities
    }
    fn start<'a>(
        &'a self,
        context: AgentRunContext,
        _: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(AgentRunResult {
                run_code: context.run_code,
                conversation_code: context.conversation_code,
                workspace_code: context.workspace_code,
                final_output: "partial".into(),
                step_count: 1,
                token_usage: None,
                latest_request_estimate: None,
                latest_usage_audit: None,
                system_agent_session_id: None,
                observations: vec![],
                sdk_execution: None,
                effective_model: None,
                native: self.status.map(|status| AgentNativeRunOutcome {
                    run_id: "native-original".into(),
                    status,
                    interactions: vec![],
                }),
            })
        })
    }
    fn cancel<'a>(
        &'a self,
        _: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), AgentInvocationError>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }
    fn resume<'a>(
        &'a self,
        _: &'a str,
        c: AgentRunContext,
        s: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        self.start(c, s)
    }
}
fn executor(active: ActiveRunRegistry) -> AgentRunExecutor {
    AgentRunExecutor::new(
        AgentRuntimeRegistry::new(
            Arc::new(NoResources),
            Arc::new(ToolRegistry::default()),
            crate::shared::event_bus::EventBusHandle::disabled(),
            reqwest::Client::new(),
        )
        .with_active_runs(active),
    )
}
#[tokio::test]
async fn executor_preserves_native_terminal_status_and_legacy_completion_without_extra_attempts() {
    for (native, state) in [
        (None, AgentRunLifecycleState::Completed),
        (
            Some(AgentNativeRunStatus::Completed),
            AgentRunLifecycleState::Completed,
        ),
        (
            Some(AgentNativeRunStatus::AwaitingInput),
            AgentRunLifecycleState::AwaitingInput,
        ),
        (
            Some(AgentNativeRunStatus::StepLimit),
            AgentRunLifecycleState::StepLimit,
        ),
        (
            Some(AgentNativeRunStatus::ToolCallLimit),
            AgentRunLifecycleState::ToolCallLimit,
        ),
    ] {
        let active = ActiveRunRegistry::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let runtime = Arc::new(ResultRuntime {
            status: native,
            calls: calls.clone(),
        });
        let mut context = test_context(0);
        context.limits.max_steps = 1;
        context.limits.max_tool_calls = 1;
        let result = executor(active.clone())
            .execute_with_runtime(runtime, context, None, None)
            .await
            .unwrap();
        assert_eq!(result.lifecycle_state, state);
        assert_eq!(
            result.lifecycle_trace,
            vec![
                AgentRunLifecycleState::Created,
                AgentRunLifecycleState::Running,
                state
            ]
        );
        assert_eq!(
            result.run.native.as_ref().map(|outcome| outcome.status),
            native
        );
        assert_eq!(result.run.step_count, 1);
        assert_eq!(result.attempts, 1);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(active.snapshot("CONV-1").unwrap().is_none());
    }
}
#[tokio::test]
async fn product_admission_returns_typed_session_busy_without_dispatch() {
    let active = ActiveRunRegistry::new();
    let _guard = active
        .register("CONV-1", "existing", "REACT", true)
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let runtime = Arc::new(ResultRuntime {
        status: None,
        calls: calls.clone(),
    });
    let error = executor(active)
        .execute_with_runtime(runtime, test_context(0), None, None)
        .await
        .err()
        .expect("busy session");
    assert!(matches!(error,AgentInvocationError::SessionBusy(ref session) if session=="CONV-1"));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
