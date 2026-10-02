//! The product completion/audit is owned along with the native wait, not by its caller.
use super::FrameworkHost;
use crate::{
    application::agent::{ActiveRunControl, ModelStreamSink, runtime::ReactAgent},
    domain::agent::{AgentError, AgentRunContext, AgentRunResult},
};
use eliteforge_agent_core as fw;
impl FrameworkHost {
    pub async fn run(
        &self,
        harness: ReactAgent,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
        steering: Option<ActiveRunControl>,
    ) -> Result<AgentRunResult, AgentError> {
        let registration = self.reserve_native(&context.run_code)?;
        let mut caller = CancelOnDrop {
            host: self.clone(),
            key: Some(context.run_code.clone()),
        };
        let host = self.clone();
        let parent = eliteforge_agent_framework::current_continuation_parent();
        let result = tokio::spawn(async move {
            let execution = host.run_inner(harness, context, sink, steering, registration);
            if let Some(parent) = parent {
                eliteforge_agent_framework::with_continuation_parent(&parent, execution).await
            } else {
                execution.await
            }
        })
        .await
        .map_err(|error| {
            AgentError::InvalidConfiguration(format!("native host owner failed: {error}"))
        })?;
        caller.key.take();
        result
    }
}
pub(super) struct CancelOnDrop {
    pub(super) host: FrameworkHost,
    pub(super) key: Option<String>,
}
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if let Some(key) = self.key.take() {
            let _ = self.host.cancel_run(&key);
        }
    }
}
pub(super) struct ProductLifecycle {
    harness: ReactAgent,
    context: AgentRunContext,
    armed: bool,
}
impl ProductLifecycle {
    pub(super) fn start(
        harness: &ReactAgent,
        context: &AgentRunContext,
    ) -> Result<Self, AgentError> {
        let guard = Self {
            harness: harness.clone(),
            context: context.clone(),
            armed: true,
        };
        harness.framework_started(context)?;
        Ok(guard)
    }
    pub(super) fn finish(
        &mut self,
        context: &AgentRunContext,
        result: &Result<fw::RunResult, fw::AgentError>,
    ) -> Result<(), AgentError> {
        self.armed = false;
        self.harness.framework_finished(context, result)
    }
}
impl Drop for ProductLifecycle {
    fn drop(&mut self) {
        if self.armed {
            // Preparation failures and panic unwinds still pair the admitted product audit.
            let error = Err(fw::AgentError::Run(
                "native preparation failed or was stopped before completion".into(),
            ));
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.harness.framework_finished(&self.context, &error)
            })) {
                Ok(Err(error)) => eprintln!("product native completion audit failed: {error}"),
                Err(_) => eprintln!("product native completion audit panicked"),
                _ => {}
            }
        }
    }
}
