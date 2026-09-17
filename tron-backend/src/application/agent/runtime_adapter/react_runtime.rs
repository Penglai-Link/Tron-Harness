use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    application::short_term_memory::ShortTermMemoryService,
    domain::agent::{AgentError, AgentRunContext, AgentRunResult, ToolRegistry},
    shared::event_bus::EventBusHandle,
};

use super::super::ReactAgentBuilder;
use super::{
    ActiveRunRegistry, AgentInvocationError, AgentRuntime, ModelClient, ModelStreamSink,
    RuntimeAdapterKey, RuntimeCapabilities,
};

#[derive(Clone)]
pub(super) struct ReactRuntime {
    key: RuntimeAdapterKey,
    capabilities: RuntimeCapabilities,
    model_client: Arc<dyn ModelClient>,
    tool_registry: Arc<ToolRegistry>,
    event_bus: EventBusHandle,
    short_term_memory: Option<ShortTermMemoryService>,
    active_runs: ActiveRunRegistry,
    auto_model_router: Option<crate::application::chat_auto_model_router::ChatAutoModelRouter>,
}

impl ReactRuntime {
    pub(super) fn new(
        key: RuntimeAdapterKey,
        capabilities: RuntimeCapabilities,
        model_client: Arc<dyn ModelClient>,
        tool_registry: Arc<ToolRegistry>,
        event_bus: EventBusHandle,
        short_term_memory: Option<ShortTermMemoryService>,
        active_runs: ActiveRunRegistry,
        auto_model_router: Option<crate::application::chat_auto_model_router::ChatAutoModelRouter>,
    ) -> Self {
        Self {
            key,
            capabilities,
            model_client,
            tool_registry,
            event_bus,
            short_term_memory,
            active_runs,
            auto_model_router,
        }
    }

    fn run<'a>(
        &'a self,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        Box::pin(async move {
            let steer_control = context
                .conversation_code
                .as_deref()
                .and_then(|session_code| {
                    self.active_runs
                        .control(session_code, &context.run_code)
                        .ok()
                });
            let agent = ReactAgentBuilder::new(
                self.model_client.clone(),
                self.tool_registry.clone(),
                self.event_bus.clone(),
            )
            .with_short_term_memory(self.short_term_memory.clone())
            .with_auto_model_router(self.auto_model_router.clone())
            .build()
            .map_err(map_agent_error)?;
            let mut cancellation_cleanup = ReactRuntimeCancellationCleanup::new(
                self.short_term_memory.clone(),
                context.clone(),
            );
            let result = match sink {
                Some(sink) => {
                    agent
                        .run_stream_with_control(context, sink, steer_control)
                        .await
                }
                None => agent.run_with_control(context, steer_control).await,
            };
            cancellation_cleanup.disarm();
            result.map_err(map_agent_error)
        })
    }
}

struct ReactRuntimeCancellationCleanup {
    short_term_memory: Option<ShortTermMemoryService>,
    context: AgentRunContext,
    armed: bool,
}

impl ReactRuntimeCancellationCleanup {
    fn new(short_term_memory: Option<ShortTermMemoryService>, context: AgentRunContext) -> Self {
        Self {
            armed: short_term_memory.is_some() && context.short_term_memory.is_some(),
            short_term_memory,
            context,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for ReactRuntimeCancellationCleanup {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let Some(short_term_memory) = self.short_term_memory.as_ref() {
            let _ = short_term_memory.record_run_failed(&self.context, "run_stopped");
        }
    }
}

impl AgentRuntime for ReactRuntime {
    fn key(&self) -> RuntimeAdapterKey {
        self.key
    }

    fn capabilities(&self) -> RuntimeCapabilities {
        self.capabilities
    }

    fn start<'a>(
        &'a self,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        self.run(context, sink)
    }

    fn resume<'a>(
        &'a self,
        session_id: &'a str,
        mut context: AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        if !self.capabilities.supports_resume {
            return Box::pin(async move {
                Err(AgentInvocationError::RuntimeFailed(format!(
                    "runtime adapter `{}` does not support resume",
                    self.key.code()
                )))
            });
        }
        context.system_agent_session_id = Some(session_id.to_string());
        self.run(context, sink)
    }

    fn cancel<'a>(
        &'a self,
        run_code: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), AgentInvocationError>> + Send + 'a>> {
        Box::pin(async move {
            Err(AgentInvocationError::RuntimeFailed(format!(
                "runtime adapter `{}` does not support cancel for run `{run_code}`",
                self.key.code()
            )))
        })
    }
}

fn map_agent_error(error: AgentError) -> AgentInvocationError {
    match error {
        AgentError::ModelFailure(message) => AgentInvocationError::ModelFailed(message),
        AgentError::ToolFailure(message)
        | AgentError::MissingTool(message)
        | AgentError::UnauthorizedTool(message) => AgentInvocationError::ToolFailed(message),
        AgentError::EmptyFinalOutput(message) => AgentInvocationError::EmptyFinalOutput(message),
        AgentError::MaxStepsExceeded => AgentInvocationError::MaxStepsExceeded,
        AgentError::MaxToolCallsExceeded => AgentInvocationError::MaxToolCallsExceeded,
        AgentError::InvalidConfiguration(message) | AgentError::EventPublishFailed(message) => {
            AgentInvocationError::RuntimeFailed(message)
        }
        AgentError::AwaitingUserInput => AgentInvocationError::RuntimeFailed(
            "agent unexpectedly returned an awaiting-user-input error".into(),
        ),
    }
}
