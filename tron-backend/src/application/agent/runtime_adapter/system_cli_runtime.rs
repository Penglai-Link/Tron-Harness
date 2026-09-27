//! Product context/result projection for the official external-Agent capability.
use super::{
    AgentInvocationError, AgentRuntime, ModelStreamSink, RuntimeAdapterKey, RuntimeCapabilities,
};
use crate::domain::agent::{AgentRunContext, AgentRunResult, ModelStreamEvent, ModelTokenUsage};
pub use eliteforge_agent_capability_external::ExternalCliKind as SystemAgentCliKind;
use eliteforge_agent_capability_external::{
    ExternalAgentEventKind, ExternalCli, ExternalCliRequest,
};
use std::{future::Future, pin::Pin};
#[derive(Debug, Clone)]
pub struct SystemCliRuntime {
    key: RuntimeAdapterKey,
    capabilities: RuntimeCapabilities,
    command_name: String,
    kind: SystemAgentCliKind,
    working_directory: Option<String>,
}
impl SystemCliRuntime {
    pub fn new(
        key: RuntimeAdapterKey,
        capabilities: RuntimeCapabilities,
        kind: SystemAgentCliKind,
        command_name: impl Into<String>,
        working_directory: Option<String>,
    ) -> Self {
        Self {
            key,
            capabilities,
            kind,
            command_name: command_name.into(),
            working_directory,
        }
    }
    async fn invoke(
        &self,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Result<AgentRunResult, AgentInvocationError> {
        let request = ExternalCliRequest {
            prompt: prompt_from_context(&context),
            model: Some(context.model.model_name.clone()),
            session_id: context.system_agent_session_id.clone(),
        };
        let sink = sink.map(|sink| {
            std::sync::Arc::new(
                move |event: eliteforge_agent_capability_external::ExternalAgentEvent| {
                    sink(match event.kind {
                        ExternalAgentEventKind::Text => ModelStreamEvent::text_delta(event.delta),
                        ExternalAgentEventKind::ReasoningSummary => {
                            ModelStreamEvent::reasoning_summary_delta(event.delta)
                        }
                    });
                },
            ) as eliteforge_agent_capability_external::ExternalAgentSink
        });
        let result = ExternalCli::new(
            self.kind,
            &self.command_name,
            self.working_directory.clone(),
        )
        .invoke(request, sink)
        .await
        .map_err(|error| match error {
            eliteforge_agent_capability_external::TransportError::PromptRequired => {
                AgentInvocationError::PromptRequired
            }
            error => AgentInvocationError::RuntimeFailed(error.to_string()),
        })?;
        Ok(AgentRunResult {
            run_code: context.run_code,
            conversation_code: context.conversation_code,
            workspace_code: context.workspace_code,
            final_output: result.output,
            step_count: 1,
            token_usage: result.usage.map(|usage| ModelTokenUsage {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                total_tokens: usage.total_tokens,
                cached_input_tokens: usage.cached_input_tokens,
                reasoning_tokens: usage.reasoning_tokens,
            }),
            latest_request_estimate: None,
            latest_usage_audit: None,
            system_agent_session_id: result.session_id,
            observations: vec![],
        })
    }
}
impl AgentRuntime for SystemCliRuntime {
    fn session_scope(&self) -> Option<String> {
        use sha2::{Digest, Sha256};
        let identity =
            serde_json::to_vec(&(self.key.code(), &self.command_name, &self.working_directory))
                .ok()?;
        Some(format!("{:x}", Sha256::digest(identity)))
    }

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
        Box::pin(async move { self.invoke(context, sink).await })
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
        Box::pin(async move { self.invoke(context, sink).await })
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

fn prompt_from_context(context: &AgentRunContext) -> String {
    let prompt = context
        .messages
        .iter()
        .rev()
        .find_map(|message| {
            (message.role == "user")
                .then(|| message.text_content().trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| {
            context
                .messages
                .last()
                .map(|message| message.text_content().trim().to_string())
                .unwrap_or_default()
        });
    if prompt.is_empty() {
        return prompt;
    }
    match crate::application::agent::execution_context_message(&context.execution_scope) {
        Some(scope) => format!("{}\n\n{prompt}", scope.text_content()),
        None => prompt,
    }
}
