use super::super::{ModelStreamSink, TurnStateAdapter, runtime::ReactAgent};
use crate::domain::agent::{
    AgentRunContext, ModelTokenUsage, TokenEstimateSnapshot, TokenUsageAuditSnapshot,
};
use async_trait::async_trait;
use eliteforge_agent_core as fw;
use std::sync::Arc;
use tokio::sync::Mutex;

pub(super) struct HostRunState {
    pub context: AgentRunContext,
    pub trusted_prefix: Vec<crate::domain::agent::AgentMessage>,
    pub tool_sequence: u32,
    pub tool_sequences: std::collections::BTreeMap<String, u32>,
    pub error: Option<crate::domain::agent::AgentError>,
    pub final_output: String,
    pub last_tool_failure: Option<String>,
    pub turn: TurnStateAdapter,
    pub usage: Option<ModelTokenUsage>,
    pub estimate: Option<TokenEstimateSnapshot>,
    pub audit: Option<TokenUsageAuditSnapshot>,
    pub last_model_failure: Option<String>,
    pub prepared: Option<crate::application::short_term_memory::PreparedModelContext>,
}

impl HostRunState {
    pub fn new(context: AgentRunContext) -> Self {
        let turn = TurnStateAdapter::new(
            context.model.protocol,
            context.model.responses_stateful_enabled,
        );
        let trusted_prefix = context
            .messages
            .iter()
            .take_while(|message| matches!(message.role.as_str(), "system" | "developer"))
            .cloned()
            .collect();
        Self {
            context,
            trusted_prefix,
            tool_sequence: 0,
            tool_sequences: Default::default(),
            error: None,
            final_output: String::new(),
            last_tool_failure: None,
            turn,
            usage: None,
            estimate: None,
            audit: None,
            prepared: None,
            last_model_failure: None,
        }
    }
}

pub(super) struct HostExecution {
    pub harness: ReactAgent,
    pub state: Arc<Mutex<HostRunState>>,
    pub sink: Option<ModelStreamSink>,
}

impl HostExecution {
    pub fn from_extensions(extensions: &fw::Extensions) -> Result<Arc<Self>, fw::AgentError> {
        extensions.get::<Self>().ok_or_else(|| {
            fw::AgentError::InvalidConfiguration("Tron execution context is missing".into())
        })
    }
}

pub(super) struct TronChatModel;

#[async_trait]
impl fw::ChatModel for TronChatModel {
    async fn generate(
        &self,
        model_id: &str,
        request: fw::ChatRequest,
        stream: &dyn fw::ChatStream,
    ) -> Result<fw::ChatResponse, fw::ModelError> {
        if model_id == "compaction" {
            let input = request
                .extensions
                .get::<super::super::runtime::CompactionModelInput>()
                .ok_or_else(|| fw::ModelError::Fatal("compaction input is missing".into()))?;
            return input.generate().await;
        }
        let host = HostExecution::from_extensions(&request.extensions)
            .map_err(|e| fw::ModelError::Fatal(e.to_string()))?;
        let mut state = host.state.lock().await;
        let native = request
            .messages
            .iter()
            .map(super::messages::decode)
            .collect::<Result<Vec<_>, _>>()
            .map_err(fw::ModelError::Fatal)?;
        let provider_identity = provider_scope(&state.context.model)?;
        if let Some(saved) = request
            .messages
            .iter()
            .rev()
            .find_map(|message| message.model_context.as_ref())
        {
            if saved.format == "tron.responses.v1"
                && saved.data.get("providerScope").and_then(|v| v.as_str())
                    == Some(provider_identity.as_str())
            {
                state
                    .turn
                    .restore_checkpoint(&saved.data["continuation"], &native)
                    .map_err(fw::ModelError::Fatal)?;
            } else {
                state.turn = TurnStateAdapter::new(
                    state.context.model.protocol,
                    state.context.model.responses_stateful_enabled,
                );
            }
        }
        state.context.messages = native;
        // The Framework catalog has already applied authorization and exposure.
        // Preserve product policy metadata while using the public descriptor schema.
        let tools = request
            .tools
            .iter()
            .map(|descriptor| {
                let mut tool = state
                    .context
                    .authorized_tools()
                    .iter()
                    .find(|tool| tool.tool_name == descriptor.call_name)
                    .cloned()
                    .ok_or_else(|| {
                        fw::ModelError::Fatal(format!(
                            "unauthorized tool in model request: {}",
                            descriptor.call_name
                        ))
                    })?;
                tool.description = descriptor.description.clone();
                tool.input_schema = descriptor.input_schema.clone();
                Ok(tool)
            })
            .collect::<Result<Vec<_>, fw::ModelError>>()?;
        state.context.tools = tools;
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1024);
        let overflow = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let callback_overflow = overflow.clone();
        let sink = host.sink.clone();
        let callback: ModelStreamSink = Arc::new(move |event| {
            if event.delta.len() > 256 * 1024 || sender.try_send(event.clone()).is_err() {
                callback_overflow.store(true, std::sync::atomic::Ordering::Release);
                return;
            }
            if let Some(sink) = &sink {
                sink(event);
            }
        });
        let HostRunState {
            context,
            turn,
            usage,
            estimate,
            audit,
            prepared,
            error: native_error,
            final_output,
            last_tool_failure,
            last_model_failure,
            ..
        } = &mut *state;
        let response = {
            let response = {
                let invocation = host.harness.framework_model_turn(
                    context,
                    turn,
                    *usage,
                    host.sink.as_ref().map(|_| callback),
                    prepared.clone(),
                );
                tokio::pin!(invocation);
                loop {
                    tokio::select! {
                        biased;
                        event = receiver.recv() => {
                            if let Some(event) = event {
                                match event.kind {
                                    crate::domain::agent::ModelStreamEventKind::Text => stream.text_delta(&event.delta).await,
                                    crate::domain::agent::ModelStreamEventKind::ReasoningSummary => stream.reasoning_delta(&event.delta).await,
                                    crate::domain::agent::ModelStreamEventKind::Reasoning => {},
                                }
                            } else {
                                break invocation.await;
                            }
                        }
                        result = &mut invocation => break result,
                    }
                }
            };
            match response {
                Ok(response) => response,
                Err(error) => {
                    let message = error.to_string();
                    let classification = match &error {
                        crate::domain::agent::AgentError::ModelFailure(message) => {
                            host.harness.framework_model_error(context, message)
                        }
                        _ => fw::ModelError::Fatal(message.clone()),
                    };
                    *last_model_failure = match &error {
                        crate::domain::agent::AgentError::ModelFailure(message) => {
                            Some(message.clone())
                        }
                        _ => None,
                    };
                    *native_error = Some(error);
                    return Err(classification);
                }
            }
        };
        while let Ok(event) = receiver.try_recv() {
            match event.kind {
                crate::domain::agent::ModelStreamEventKind::Text => {
                    stream.text_delta(&event.delta).await
                }
                crate::domain::agent::ModelStreamEventKind::ReasoningSummary => {
                    stream.reasoning_delta(&event.delta).await
                }
                crate::domain::agent::ModelStreamEventKind::Reasoning => {}
            }
        }
        if overflow.load(std::sync::atomic::Ordering::Acquire) {
            return Err(fw::ModelError::Fatal(
                "model stream exceeded the bounded host handoff capacity".into(),
            ));
        }
        *last_model_failure = None;
        *native_error = None;
        let (response, request_estimate, usage_audit, total_usage) = response;
        if let Some(session) = response.system_agent_session_id() {
            context.system_agent_session_id = Some(session.into());
        }
        *usage = total_usage;
        *estimate = Some(request_estimate);
        *audit = Some(usage_audit);
        if response.finish == crate::domain::agent::ModelFinish::Final {
            let text = response.output_text();
            if text.trim().is_empty() {
                let message = last_tool_failure.as_ref().map(|failure| format!("agent completed without an assistant response; last tool failure: {failure}"))
                    .unwrap_or_else(|| "agent completed without an assistant response".into());
                *native_error = Some(crate::domain::agent::AgentError::EmptyFinalOutput(
                    message.clone(),
                ));
                return Err(fw::ModelError::Fatal(message));
            }
            final_output.push_str(&text);
        }
        let message =
            super::messages::encode(&response.assistant).map_err(fw::ModelError::Fatal)?;
        let mut persisted_messages = context.messages.clone();
        if response.finish == crate::domain::agent::ModelFinish::Final {
            persisted_messages.push(response.assistant.clone());
            turn.apply_model_response(&response, persisted_messages.len());
        }
        let provider_identity = provider_scope(&context.model)?;
        let continuation = turn
            .checkpoint(&persisted_messages)
            .map_err(fw::ModelError::Fatal)?;
        let model_context = continuation.map(|continuation| fw::ModelMessageContext {
            model: fw::ModelRef::new("tron", "route"),
            protocol: "tron-host".into(),
            format: "tron.responses.v1".into(),
            data: serde_json::json!({"providerScope": provider_identity, "continuation": continuation}),
        });
        Ok(fw::ChatResponse {
            content: message.content,
            structured: None,
            tool_calls: message.tool_calls,
            usage: fw::Usage {
                input_tokens: response.token_usage.unwrap_or_default().input_tokens as u64,
                output_tokens: response.token_usage.unwrap_or_default().output_tokens as u64,
            },
            model_context,
        })
    }
}

fn provider_scope(
    model: &crate::domain::agent::AgentModelConfig,
) -> Result<String, fw::ModelError> {
    use sha2::{Digest, Sha256};
    let identity = serde_json::to_vec(&(
        &model.model_code,
        &model.model_name,
        &model.base_url,
        &model.api_path,
        model.protocol,
    ))
    .map_err(|e| fw::ModelError::Fatal(e.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(identity)))
}
