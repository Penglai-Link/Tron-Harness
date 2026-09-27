//! External agent transports run as a single Framework model stage. Framework
//! owns run admission, cancellation and durable conversation/checkpoint state;
//! the remote/CLI agent retains its own tool loop and native session protocol.
use super::FrameworkHost;
use crate::{
    application::agent::{
        AgentInvocationError, AgentRuntime, ModelStreamSink, RuntimeAdapterKey, RuntimeCapabilities,
    },
    domain::agent::{AgentRunContext, AgentRunResult},
};
use eliteforge_agent_core as fw;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

pub(super) struct ExternalExecution {
    pub transport: Arc<dyn AgentRuntime>,
    pub context: AgentRunContext,
    pub sink: Option<ModelStreamSink>,
    pub result: Mutex<Option<Result<AgentRunResult, AgentInvocationError>>>,
}
impl ExternalExecution {
    pub fn session_scope(&self) -> Option<String> {
        use sha2::{Digest, Sha256};
        let transport = self.transport.session_scope()?;
        let context = &self.context;
        let identity = serde_json::to_vec(&(
            transport,
            &context.user_code,
            &context.workspace_code,
            &context.execution_scope,
            &context.model.base_url,
            &context.model.api_key,
            &context.model.model_name,
        ))
        .ok()?;
        Some(format!("{:x}", Sha256::digest(identity)))
    }
}
#[async_trait::async_trait]
impl eliteforge_agent_capability_external::ExternalAgentInvocation for ExternalExecution {
    async fn invoke(
        &self,
        session: Option<String>,
        _stream: &dyn fw::ChatStream,
    ) -> Result<eliteforge_agent_capability_external::ExternalStageResult, fw::ModelError> {
        let mut context = self.context.clone();
        context.system_agent_session_id = session.clone();
        let result = match session {
            Some(session) => {
                self.transport
                    .resume(&session, context, self.sink.clone())
                    .await
            }
            None => self.transport.start(context, self.sink.clone()).await,
        };
        let response = match &result {
            Ok(result) => Ok(eliteforge_agent_capability_external::ExternalStageResult {
                output: result.final_output.clone(),
                payload: serde_json::to_value(result).ok(),
                usage: result
                    .token_usage
                    .map(|usage| fw::Usage {
                        input_tokens: usage.input_tokens as u64,
                        output_tokens: usage.output_tokens as u64,
                    })
                    .unwrap_or_default(),
                session_id: result.system_agent_session_id.clone(),
            }),
            Err(error) => Err(fw::ModelError::Fatal(format!("{error:?}"))),
        };
        *self
            .result
            .lock()
            .map_err(|_| fw::ModelError::Fatal("external result lock unavailable".into()))? =
            Some(result);
        response
    }
}

#[derive(Clone)]
pub(crate) struct FrameworkExternalRuntime {
    pub host: FrameworkHost,
    pub transport: Arc<dyn AgentRuntime>,
}
impl AgentRuntime for FrameworkExternalRuntime {
    fn key(&self) -> RuntimeAdapterKey {
        self.transport.key()
    }
    fn capabilities(&self) -> RuntimeCapabilities {
        self.transport.capabilities()
    }
    fn start<'a>(
        &'a self,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        Box::pin(
            self.host
                .run_external(self.transport.clone(), context, sink, false),
        )
    }
    fn resume<'a>(
        &'a self,
        session: &'a str,
        mut context: AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        context.system_agent_session_id = Some(session.into());
        Box::pin(
            self.host
                .run_external(self.transport.clone(), context, sink, true),
        )
    }
    fn cancel<'a>(
        &'a self,
        run: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), AgentInvocationError>> + Send + 'a>> {
        self.transport.cancel(run)
    }
}
