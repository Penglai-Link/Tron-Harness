//! Explicit product bridges into a native continuation execution tree.
use super::*;
use eliteforge_agent_core as fw;
use eliteforge_agent_framework::{Agent, RunControl};
struct CallerCancellation {
    agent: Agent,
    control: RunControl,
}
impl Drop for CallerCancellation {
    fn drop(&mut self) {
        let _ = self.agent.cancel(&self.control);
    }
}
impl AgentInvocationService {
    pub(crate) fn ensure_continuation_active(
        &self,
        session: &str,
        parent: &fw::RunContext,
    ) -> Result<(), AgentInvocationError> {
        self.active_runs
            .bind_continuation_context(session, parent)
            .map_err(|error| match error {
                super::super::ActiveRunError::RunFinishing => AgentInvocationError::RunStopped,
                _ => AgentInvocationError::RuntimeFailed(error.to_string()),
            })?;
        parent.check_cancelled().map_err(map_continuation_error)
    }
    pub(crate) async fn run_goal_driver(
        &self,
        mut request: fw::RunRequest,
        session: &str,
        previous_run: &str,
        adapter: &str,
        model: &AgentModelConfig,
    ) -> Result<fw::RunResult, AgentInvocationError> {
        let host = self.framework_host.as_ref().ok_or_else(|| {
            AgentInvocationError::RuntimeFailed("native continuation host is unavailable".into())
        })?;
        let group = self
            .active_runs
            .continuation_group(session, previous_run, adapter)
            .map_err(|e| AgentInvocationError::RuntimeFailed(e.to_string()))?;
        let agent = host
            .continuation_agent()
            .await
            .map_err(AgentInvocationError::RuntimeFailed)?;
        super::super::framework_runtime::attach_auxiliary_model(
            &mut request,
            model,
            self.model_client.clone(),
        )
        .map_err(map_continuation_error)?;
        group.attach(&mut request.extensions);
        request
            .extensions
            .insert(eliteforge_agent_official_plugin_otel::TraceParent(
                opentelemetry::Context::current(),
            ));
        let presentation = host
            .presentation_service()
            .map_err(AgentInvocationError::RuntimeFailed)?;
        let (handle, output) = agent
            .start_with_output::<eliteforge_agent_official_plugin_agui::AgUiStream>(request)
            .map_err(map_continuation_error)?;
        let session = session.to_owned();
        let previous_run = previous_run.to_owned();
        let control = agent.control(&handle).map_err(map_continuation_error)?;
        group
            .bind(agent.clone(), control.clone())
            .map_err(|e| AgentInvocationError::RuntimeFailed(e.to_string()))?;
        let caller = CallerCancellation {
            agent: agent.clone(),
            control,
        };
        // This owner retains the group through SDK child/resource cleanup even
        // if the HTTP caller disappears. Drop only sends native cancellation.
        let outcome = crate::app::telemetry::spawn_traced(async move {
            let _group = group;
            let (outcome, ()) = tokio::join!(
                agent.wait(handle),
                super::super::framework_runtime::consume_native_agui_group(
                    output,
                    &session,
                    &previous_run,
                    presentation
                )
            );
            outcome
        })
        .await
        .map_err(|e| {
            AgentInvocationError::RuntimeFailed(format!("continuation owner failed: {e}"))
        })?;
        drop(caller);
        outcome.map_err(map_continuation_error)
    }
    pub(crate) async fn invoke_continuation(
        &self,
        command: AgentInvocationCommand,
        user: Option<String>,
        locale: Option<String>,
        parent: &fw::RunContext,
    ) -> Result<AgentInvocationResult, AgentInvocationError> {
        eliteforge_agent_framework::with_continuation_parent(
            parent,
            self.invoke(command, user, locale),
        )
        .await
    }
}
fn map_continuation_error(error: fw::AgentError) -> AgentInvocationError {
    match error.primary() {
        fw::AgentError::Cancelled => AgentInvocationError::RunStopped,
        fw::AgentError::SessionBusy(session) => AgentInvocationError::SessionBusy(session.clone()),
        _ => AgentInvocationError::RuntimeFailed(error.to_string()),
    }
}
