//! Independent Framework ExternalAgentLoop delegates the native tool loop.
//! Tron owns session identity binding; external runtimes own native checkpoints.
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
        let context = &self.context;
        let identity = serde_json::to_vec(&(
            self.transport.session_scope()?,
            &context.user_code,
            &context.workspace_code,
            &context.agent_code,
            &context.execution_scope,
            &context.model.base_url,
            &context.model.api_key,
            &context.model.model_name,
            &context.model.api_path,
            context.model.protocol,
        ))
        .ok()?;
        Some(format!("{:x}", Sha256::digest(identity)))
    }
}
pub(super) struct HostExternalAgent;
#[async_trait::async_trait]
impl fw::ExternalAgent for HostExternalAgent {
    async fn run(
        &self,
        request: fw::ExternalAgentRequest,
        sink: &dyn fw::ExternalAgentEventSink,
    ) -> Result<fw::ExternalAgentResult, fw::AgentError> {
        let execution = request
            .extensions
            .get::<ExternalExecution>()
            .ok_or_else(|| fw::AgentError::Run("external execution is missing".into()))?;
        let mut context = execution.context.clone();
        if let Some(value) = fw::annotation_model_context(&request.annotations)
            && let Some(message) = std::sync::Arc::make_mut(&mut context.messages)
                .iter_mut()
                .rfind(|message| message.role == "user")
        {
            message
                .parts
                .push(crate::domain::agent::AgentMessagePart::Context {
                    kind: "conversation_annotations".into(),
                    text: value.to_string(),
                    metadata: Some(value),
                });
        }
        let session = request.session.map(|session| session.session_id);
        context.system_agent_session_id = session.clone();
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1024);
        let product_sink = execution.sink.clone();
        let overflow = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let callback_overflow = overflow.clone();
        let observed: ModelStreamSink = Arc::new(move |event| {
            if event.delta.len() > 256 * 1024 || sender.try_send(event.clone()).is_err() {
                callback_overflow.store(true, std::sync::atomic::Ordering::Release);
                return;
            }
            if let Some(sink) = &product_sink {
                sink(event);
            }
        });
        let (observation_sender, mut observation_receiver) = tokio::sync::mpsc::channel(1024);
        let observation_overflow = overflow.clone();
        let observations = Arc::new(move |observation| {
            if let Some(event) = observation_event(&observation) {
                if observation_sender.try_send(event).is_err() {
                    observation_overflow.store(true, std::sync::atomic::Ordering::Release);
                }
            }
        });
        let invocation = execution.transport.run_observed(
            session.as_deref(),
            context,
            Some(observed),
            observations,
        );
        let mut observed_events = std::collections::HashSet::new();
        tokio::pin!(invocation);
        let result = loop {
            if overflow.load(std::sync::atomic::Ordering::Acquire) {
                return Err(fw::AgentError::Run(
                    "external stream exceeded bounded event capacity".into(),
                ));
            }
            tokio::select! {
                result = &mut invocation => break result,
                Some(event) = observation_receiver.recv() => {
                    observed_events.insert(serde_json::to_string(&event).expect("serializable observation"));
                    sink.emit(event).await;
                },
                event = receiver.recv() => match event {
                    Some(event) => forward_event(event, sink).await,
                    None => break invocation.await,
                },
            }
        };
        while let Ok(event) = receiver.try_recv() {
            forward_event(event, sink).await;
        }
        while let Ok(event) = observation_receiver.try_recv() {
            observed_events
                .insert(serde_json::to_string(&event).expect("serializable observation"));
            sink.emit(event).await;
        }
        if overflow.load(std::sync::atomic::Ordering::Acquire) {
            return Err(fw::AgentError::Run(
                "external stream exceeded bounded event capacity".into(),
            ));
        }
        if let Ok(result) = &result {
            for observation in &result.observations {
                if let Some(event) = observation_event(observation) {
                    if observed_events
                        .insert(serde_json::to_string(&event).expect("serializable observation"))
                    {
                        sink.emit(event).await;
                    }
                }
            }
        }
        if let Some(binding) = request
            .extensions
            .get::<super::native_sessions::CollaborationBinding>()
        {
            match &result {
                Ok(result) => binding.0.save(result).await,
                Err(_) => binding.0.failed().await,
            }
            .map_err(|error| fw::AgentError::Run(error.message()))?;
        }
        let response = match &result {
            Ok(result) => Ok(fw::ExternalAgentResult {
                content: result.final_output.clone().into(),
                structured: serde_json::to_value(result).ok(),
                usage: result
                    .token_usage
                    .map(|usage| fw::Usage {
                        input_tokens: usage.input_tokens as u64,
                        output_tokens: usage.output_tokens as u64,
                    })
                    .unwrap_or_default(),
                session: result.system_agent_session_id.clone().map(|session_id| {
                    fw::ExternalSessionRef {
                        session_id,
                        run_id: None,
                    }
                }),
            }),
            Err(AgentInvocationError::RuntimeNotStarted(message)) => {
                Err(fw::AgentError::ExternalNotStarted(message.clone()))
            }
            Err(AgentInvocationError::PromptRequired) => Err(fw::AgentError::ExternalNotStarted(
                "prompt is required".into(),
            )),
            Err(error) => Err(fw::AgentError::Run(format!("{error:?}"))),
        };
        *execution
            .result
            .lock()
            .map_err(|_| fw::AgentError::Run("external result lock unavailable".into()))? =
            Some(result);
        response
    }
}
fn observation_event(
    observation: &crate::domain::agent::ToolObservation,
) -> Option<fw::ExternalAgentEvent> {
    let value: serde_json::Value = serde_json::from_str(&observation.output).ok()?;
    match observation.tool_name.as_str() {
        "external_ToolObserved" => Some(fw::ExternalAgentEvent::Tool {
            name: value["delta"].as_str().unwrap_or("external").into(),
            detail: value["detail"].clone(),
        }),
        "external_ApprovalObserved" => Some(fw::ExternalAgentEvent::Approval {
            detail: value["detail"].clone(),
        }),
        "external_Status" => Some(fw::ExternalAgentEvent::Status(
            value["delta"].as_str().unwrap_or_default().into(),
        )),
        _ => None,
    }
}

async fn forward_event(
    event: crate::domain::agent::ModelStreamEvent,
    sink: &dyn fw::ExternalAgentEventSink,
) {
    use crate::domain::agent::ModelStreamEventKind;
    match event.kind {
        ModelStreamEventKind::Text => {
            sink.emit(fw::ExternalAgentEvent::TextDelta(event.delta))
                .await
        }
        ModelStreamEventKind::ReasoningSummary | ModelStreamEventKind::Reasoning => {
            if event.delta.trim().is_empty() {
                return;
            }
            sink.emit(fw::ExternalAgentEvent::ReasoningDelta(event.delta))
                .await
        }
        ModelStreamEventKind::MessageStart | ModelStreamEventKind::MessageEnd | ModelStreamEventKind::ToolBoundary | ModelStreamEventKind::ToolCallDelta => {}
    }
}

#[derive(Clone)]
pub(crate) struct FrameworkExternalRuntime {
    pub host: FrameworkHost,
    pub transport: Arc<dyn AgentRuntime>,
}
impl AgentRuntime for FrameworkExternalRuntime {
    fn owns_native_lifecycle(&self) -> bool {
        true
    }
    fn session_scope(&self) -> Option<String> {
        self.transport.session_scope()
    }
    fn key(&self) -> RuntimeAdapterKey {
        self.transport.key()
    }
    fn capabilities(&self) -> RuntimeCapabilities {
        let mut capabilities = self.transport.capabilities();
        capabilities.supports_cancel = true;
        capabilities
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
        Box::pin(async move {
            self.host
                .cancel_run(run)
                .map_err(|error| AgentInvocationError::RuntimeFailed(error.to_string()))?;
            if self.transport.capabilities().supports_cancel {
                super::super::run_cancellation::cancel_runtime_after_stop(&self.transport, run)
                    .await;
            }
            Ok(())
        })
    }
}
