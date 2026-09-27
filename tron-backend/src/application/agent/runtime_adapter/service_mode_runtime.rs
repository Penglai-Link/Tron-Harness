use std::{future::Future, pin::Pin};

use serde::Serialize;

use crate::domain::agent::{AgentRunContext, AgentRunResult, ModelStreamEvent};

use super::{
    AgentInvocationError, AgentRuntime, ModelStreamSink, RuntimeAdapterKey, RuntimeCapabilities,
};

#[derive(Clone)]
pub(super) struct ServiceModeRuntime {
    key: RuntimeAdapterKey,
    capabilities: RuntimeCapabilities,
    endpoint: String,
    working_directory: Option<String>,
    client: reqwest::Client,
}

impl ServiceModeRuntime {
    pub(super) fn new(
        key: RuntimeAdapterKey,
        capabilities: RuntimeCapabilities,
        endpoint: String,
        working_directory: Option<String>,
        client: reqwest::Client,
    ) -> Self {
        Self {
            key,
            capabilities,
            endpoint,
            working_directory,
            client,
        }
    }

    fn run<'a>(
        &'a self,
        action: &'static str,
        requested_session_id: Option<String>,
        mut context: AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        Box::pin(async move {
            if let Some(session_id) = requested_session_id.as_ref() {
                context.system_agent_session_id = Some(session_id.clone());
            }
            if context.variables.iter().any(|variable| variable.sensitive) {
                return Err(AgentInvocationError::RuntimeSecretDeliveryUntrusted);
            }
            let bridge = eliteforge_agent_capability_external::JsonAgentBridge::new(
                &self.endpoint,
                self.client.clone(),
            )
            .map_err(|error| AgentInvocationError::RuntimeFailed(error.to_string()))?;
            let body = serde_json::to_value(ServiceModeRunRequest {
                action: action.into(),
                requested_session_id,
                working_directory: self.working_directory.clone(),
                context,
            })
            .map_err(|error| AgentInvocationError::RuntimeFailed(error.to_string()))?;
            let payload = bridge
                .invoke(action == "resume", &body)
                .await
                .map_err(|error| AgentInvocationError::RuntimeFailed(error.to_string()))?;
            let run = serde_json::from_value::<AgentRunResult>(payload).map_err(|error| {
                AgentInvocationError::RuntimeFailed(format!(
                    "runtime adapter `{}` run decode failed: {error}",
                    self.key.code()
                ))
            })?;
            if let Some(sink) = sink
                && !run.final_output.is_empty()
            {
                for chunk in split_stream_chunks(&run.final_output, 72) {
                    sink(ModelStreamEvent::text_delta(chunk));
                }
            }
            Ok(run)
        })
    }

    fn cancel_request<'a>(
        &'a self,
        run_code: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), AgentInvocationError>> + Send + 'a>> {
        Box::pin(async move {
            eliteforge_agent_capability_external::JsonAgentBridge::new(
                &self.endpoint,
                self.client.clone(),
            )
            .map_err(|error| AgentInvocationError::RuntimeFailed(error.to_string()))?
            .cancel(run_code, &serde_json::json!({ "runCode": run_code }))
            .await
            .map_err(|error| AgentInvocationError::RuntimeFailed(error.to_string()))
        })
    }
}

impl AgentRuntime for ServiceModeRuntime {
    fn session_scope(&self) -> Option<String> {
        use sha2::{Digest, Sha256};
        let identity =
            serde_json::to_vec(&(self.key.code(), &self.endpoint, &self.working_directory)).ok()?;
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
        self.run("start", None, context, sink)
    }

    fn resume<'a>(
        &'a self,
        session_id: &'a str,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        self.run("resume", Some(session_id.to_string()), context, sink)
    }

    fn cancel<'a>(
        &'a self,
        run_code: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), AgentInvocationError>> + Send + 'a>> {
        self.cancel_request(run_code)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceModeRunRequest {
    action: String,
    requested_session_id: Option<String>,
    working_directory: Option<String>,
    context: AgentRunContext,
}

fn split_stream_chunks(output: &str, chunk_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for ch in output.chars() {
        current.push(ch);
        if current.chars().count() >= chunk_chars {
            chunks.push(current.clone());
            current.clear();
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}
