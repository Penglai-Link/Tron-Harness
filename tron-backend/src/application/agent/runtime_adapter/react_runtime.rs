use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    application::short_term_memory::ShortTermMemoryService,
    domain::agent::{AgentError, AgentRunContext, AgentRunResult, ToolRegistry},
    shared::event_bus::EventBusHandle,
};

use super::super::ReactAgentBuilder;
use super::{
    AgentInvocationError, AgentRuntime, ModelClient, ModelStreamSink, RuntimeAdapterKey,
    RuntimeCapabilities,
};

#[derive(Clone)]
pub(super) struct ReactRuntime {
    key: RuntimeAdapterKey,
    capabilities: RuntimeCapabilities,
    model_client: Arc<dyn ModelClient>,
    tool_registry: Arc<ToolRegistry>,
    event_bus: EventBusHandle,
    short_term_memory: Option<ShortTermMemoryService>,
}

impl ReactRuntime {
    pub(super) fn new(
        key: RuntimeAdapterKey,
        capabilities: RuntimeCapabilities,
        model_client: Arc<dyn ModelClient>,
        tool_registry: Arc<ToolRegistry>,
        event_bus: EventBusHandle,
        short_term_memory: Option<ShortTermMemoryService>,
    ) -> Self {
        Self {
            key,
            capabilities,
            model_client,
            tool_registry,
            event_bus,
            short_term_memory,
        }
    }

    fn run<'a>(
        &'a self,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        Box::pin(async move {
            let agent = ReactAgentBuilder::new(
                self.model_client.clone(),
                self.tool_registry.clone(),
                self.event_bus.clone(),
            )
            .with_short_term_memory(self.short_term_memory.clone())
            .build()
            .map_err(map_agent_error)?;
            match sink {
                Some(sink) => agent.run_stream(context, sink).await,
                None => agent.run(context).await,
            }
            .map_err(map_agent_error)
        })
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
        AgentError::MaxStepsExceeded => AgentInvocationError::MaxStepsExceeded,
        AgentError::MaxToolCallsExceeded => AgentInvocationError::MaxToolCallsExceeded,
        AgentError::InvalidConfiguration(message) | AgentError::EventPublishFailed(message) => {
            AgentInvocationError::RuntimeFailed(message)
        }
    }
}
