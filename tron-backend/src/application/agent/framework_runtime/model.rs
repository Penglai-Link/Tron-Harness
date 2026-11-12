use super::super::{ModelStreamSink, TurnStateAdapter, runtime::ReactAgent};
use crate::domain::agent::{
    AgentRunContext, ModelTokenUsage, TokenEstimateSnapshot, TokenUsageAuditSnapshot,
};
use async_trait::async_trait;
use eliteforge_agent_core as fw;
use std::sync::Arc;
use tokio::sync::Mutex;

pub(super) struct HostRunState {
    pub onboarding: bool,
    /// Admitted onboarding schemas, captured once per run and reused at every step.
    pub onboarding_tools: Vec<fw::ToolDescriptor>,
    pub context: AgentRunContext,
    /// Product ACL captured before model exposure rewrites `context.tools`.
    pub product_tool_authority: std::collections::BTreeSet<String>,
    pub trusted_prefix: Vec<crate::domain::agent::AgentMessage>,
    /// Actual admitted native IDs for product exposure audits; never an executor.
    pub tool_catalog: std::collections::BTreeMap<String, String>,
    pub pending_tool_bindings: Vec<fw::ToolBinding>,
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
    pub context_force_compression: bool,
    pub context_auxiliary_calls: Option<(usize, Arc<std::sync::atomic::AtomicUsize>)>,
    pub prepared: Option<crate::application::short_term_memory::PreparedModelContext>,
}

impl HostRunState {
    pub fn new(context: AgentRunContext) -> Self {
        let turn = TurnStateAdapter::new(
            context.model.protocol,
            context.model.responses_stateful_enabled && !context.is_owning_dude(),
        );
        let trusted_prefix = context
            .messages
            .iter()
            .take_while(|message| super::messages::is_prompt_prefix(message))
            .cloned()
            .collect();
        let product_tool_authority = context
            .authorized_tools()
            .iter()
            .filter(|tool| context.is_tool_authorized(&tool.tool_name))
            .map(|tool| tool.tool_name.clone())
            .collect();
        Self {
            onboarding: false,
            onboarding_tools: Vec::new(),
            context,
            product_tool_authority,
            trusted_prefix,
            tool_catalog: Default::default(),
            pending_tool_bindings: Vec::new(),
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
            context_force_compression: false,
            context_auxiliary_calls: None,
        }
    }
}

pub(super) struct HostExecution {
    pub harness: ReactAgent,
    pub state: Arc<Mutex<HostRunState>>,
    pub sink: Option<ModelStreamSink>,
    pub summaries: Arc<dyn eliteforge_agent_official_plugin_storage_local::SummaryStore>,
}

impl HostExecution {
    pub fn from_extensions(extensions: &fw::Extensions) -> Result<Arc<Self>, fw::AgentError> {
        extensions.get::<Self>().ok_or_else(|| {
            fw::AgentError::InvalidConfiguration("Tron execution context is missing".into())
        })
    }
}

#[path = "native_models.rs"]
mod native;
#[path = "model_retry.rs"]
mod retry;
pub(crate) use native::NativeModelScope;
pub(super) use native::{compaction_model_reference, risk_review_model, synchronize_selected_model};
pub(super) struct TronChatModel {
    reference: fw::ModelRef,
    config: crate::domain::agent::AgentModelConfig,
    client: Arc<dyn super::super::ModelClient>,
    auxiliary: bool,
}

#[async_trait]
impl fw::ChatModel for TronChatModel {
    fn on_failure(&self, _: &fw::Extensions, error: &fw::ModelError) {
        self.client.on_model_failure(&self.config, error);
    }
    async fn recover_authentication(
        &self,
        _: &fw::Extensions,
        error: &fw::ModelError,
    ) -> Result<bool, fw::ModelError> {
        self.client
            .recover_authentication(&self.config, error)
            .await
    }

    async fn generate(
        &self,
        model_id: &str,
        request: fw::ChatRequest,
        stream: &dyn fw::ChatStream,
    ) -> Result<fw::ChatResponse, fw::ModelError> {
        if self.auxiliary {
            if model_id != self.config.model_code {
                return Err(fw::ModelError::Fatal(
                    "native model binding identity mismatch".into(),
                ));
            }
            let input = request
                .extensions
                .get::<super::super::runtime::CompactionModelInput>()
                .ok_or_else(|| fw::ModelError::Fatal("compaction input is missing".into()))?;
            return input
                .with_client(self.client.clone())
                .generate(&request)
                .await;
        }
        if let Some(retry) = request.extensions.get::<Arc<retry::SessionModelRetry>>() {
            retry.wait().await;
        }
        let host = HostExecution::from_extensions(&request.extensions)
            .map_err(|e| fw::ModelError::Fatal(e.to_string()))?;
        let mut state = host.state.lock().await;
        synchronize_selected_model(
            &mut state,
            &self.reference,
            &request.extensions,
            &request.options,
        )
        .map_err(|e| fw::ModelError::Fatal(e.to_string()))?;
        let harness = host
            .harness
            .framework_with_model_client(self.client.clone());
        let native = request
            .messages
            .iter()
            .map(super::messages::decode)
            .collect::<Result<Vec<_>, _>>()
            .map_err(fw::ModelError::Fatal)?;
        let provider_identity = provider_scope(&state.context)?;
        if let Some(saved) = request
            .messages
            .iter()
            .rev()
            .find_map(|message| message.model_context.as_ref())
        {
            if saved.format == "tron.responses.v2"
                && saved.model == self.reference
                && saved.protocol == native::protocol(&self.config)
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
        state.context.messages = native.into();
        state.context.model.temperature = request.options.temperature;
        state.context.model.top_p = request.options.top_p;
        state.context.model.max_tokens = request.options.max_output_tokens;
        let selected_messages = state.context.messages.clone();
        if let Some(prepared) = &mut state.prepared {
            prepared.messages = selected_messages;
        }
        // The Framework catalog has already applied authorization and exposure.
        // Preserve product policy metadata while using the public descriptor schema.
        let tools = request
            .tools
            .iter()
            .map(|descriptor| {
                super::tools::map_authorized_descriptor(
                    &state.context,
                    descriptor,
                    &request.extensions,
                )
                .map_err(|error| fw::ModelError::Fatal(error.to_string()))
            })
            .collect::<Result<Vec<_>, fw::ModelError>>()?;
        state.context.tools = tools;
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1024);
        let overflow = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let callback_overflow = overflow.clone();
        let sink = host.sink.clone();
        let callback: ModelStreamSink = Arc::new(move |event| {
            if matches!(event.kind, crate::domain::agent::ModelStreamEventKind::Text | crate::domain::agent::ModelStreamEventKind::Reasoning | crate::domain::agent::ModelStreamEventKind::ReasoningSummary)
                && (event.delta.is_empty() || (event.kind != crate::domain::agent::ModelStreamEventKind::Text && event.delta.trim().is_empty())) {
                return;
            }
            // Raw chain-of-thought is internal evidence: present the reasoning
            // summary to streams, never the raw reasoning itself.
            if matches!(event.kind, crate::domain::agent::ModelStreamEventKind::Reasoning | crate::domain::agent::ModelStreamEventKind::ToolCallDelta) {
                return;
            }
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
                let invocation = harness.framework_model_turn(
                    context,
                    turn,
                    *usage,
                    Some(callback),
                    prepared.take(),
                    Some(&request),
                );
                tokio::pin!(invocation);
                loop {
                    if overflow.load(std::sync::atomic::Ordering::Acquire) {
                        break Err(crate::domain::agent::AgentError::InvalidConfiguration(
                            "model stream exceeded the bounded host handoff capacity".into(),
                        ));
                    }
                    tokio::select! {
                        biased;
                        event = receiver.recv() => {
                            if let Some(event) = event {
                                emit_model_event(stream, event).await;
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
                    if matches!(&classification, fw::ModelError::Provider(error) if error.category == fw::ProviderErrorCategory::Quota)
                        && let Some(retry) = request.extensions.get::<Arc<retry::SessionModelRetry>>()
                    {
                        retry.exhaust(&self.reference);
                    }
                    if classification.is_transient()
                        && let Some(retry) = request.extensions.get::<Arc<retry::SessionModelRetry>>()
                    {
                        retry.failed();
                    }
                    return Err(classification);
                }
            }
        };
        while let Ok(event) = receiver.try_recv() {
            emit_model_event(stream, event).await;
        }
        if overflow.load(std::sync::atomic::Ordering::Acquire) {
            *native_error = Some(crate::domain::agent::AgentError::InvalidConfiguration(
                "model stream exceeded the bounded host handoff capacity".into(),
            ));
            *last_model_failure = None;
            return Err(fw::ModelError::Fatal(
                "model stream exceeded the bounded host handoff capacity".into(),
            ));
        }
        *last_model_failure = None;
        *native_error = None;
        if let Some(retry) = request.extensions.get::<Arc<retry::SessionModelRetry>>() {
            retry.succeeded();
        }
        let (response, request_estimate, usage_audit, total_usage) = response;
        if let Some(session) = response.system_agent_session_id() {
            context.system_agent_session_id = Some(session.into());
        }
        *usage = total_usage;
        *estimate = Some(request_estimate);
        *audit = Some(usage_audit);
        if response.finish == crate::domain::agent::ModelFinish::Final {
            let text = response.output_text();
            // Segmented responses still need to reach the Framework so it can
            // persist late phase metadata before reporting an incomplete run.
            if text.trim().is_empty() && response.output_items.is_empty() {
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
        let persisted_messages: std::borrow::Cow<'_, [crate::domain::agent::AgentMessage]> =
            if matches!(response.finish, crate::domain::agent::ModelFinish::Final | crate::domain::agent::ModelFinish::Continue) {
                let mut messages = context.messages.as_ref().clone();
                messages.extend(response.transcript_messages());
                turn.apply_model_response(&response, messages.len());
                std::borrow::Cow::Owned(messages)
            } else {
                std::borrow::Cow::Borrowed(&context.messages)
            };
        let provider_identity = provider_scope(context)?;
        let continuation = turn
            .checkpoint(&persisted_messages)
            .map_err(fw::ModelError::Fatal)?;
        let model_context = continuation.map(|continuation| fw::ModelMessageContext {
            model: self.reference.clone(),
            protocol: native::protocol(&self.config).into(),
            format: "tron.responses.v2".into(),
            data: serde_json::json!({"providerScope": provider_identity, "continuation": continuation}),
        });
        let metrics = response.token_usage.map(|usage| fw::ProviderMetrics {
            cached_input_tokens: usage.cached_input_tokens.map(u64::from),
            reasoning_tokens: usage.reasoning_tokens.map(u64::from),
        });
        let mut tool_index = 0;
        let output_items = response.output_items.iter().filter_map(|item| match item {
            crate::domain::agent::ModelOutputItem::Message { output_index, item_id, phase, text } => Some(fw::ModelOutputItem::Message { output_index: *output_index, item_id: item_id.clone(), phase: phase.clone(), content: text.clone().into() }),
            crate::domain::agent::ModelOutputItem::ReasoningSummary { output_index, text } => Some(fw::ModelOutputItem::ReasoningSummary { output_index: *output_index, text: text.clone() }),
            crate::domain::agent::ModelOutputItem::ToolCall { output_index, .. } => { let call = message.tool_calls.get(tool_index).cloned(); tool_index += 1; call.map(|call| fw::ModelOutputItem::ToolCall { output_index: *output_index, call }) },
        }).collect();
        let segmented_content = (!response.output_items.is_empty()).then(|| fw::Content::from(response.output_text()));
        let result = fw::ChatResponse {
            output_items,
            disposition: Some(match response.finish { crate::domain::agent::ModelFinish::Final => fw::ModelDisposition::Final, crate::domain::agent::ModelFinish::ToolCalls => fw::ModelDisposition::ToolCalls, crate::domain::agent::ModelFinish::Continue => fw::ModelDisposition::Continue }),
            public_content: segmented_content.clone().or_else(|| Some(fw::Content(
                message
                    .content
                    .0
                    .iter()
                    .filter(|block| match block {
                        fw::ContentBlock::Json { value } => {
                            !value.get("tronMessagePart").is_some_and(|part| {
                                matches!(
                                    part.get("type").and_then(serde_json::Value::as_str),
                                    Some("reasoning" | "reasoningSummary" | "toolCall")
                                )
                            })
                        }
                        _ => true,
                    })
                    .cloned()
                    .collect(),
            ))),
            reasoning: {
                let text: String = response
                    .assistant
                    .parts
                    .iter()
                    .filter_map(|part| match part {
                        crate::domain::agent::AgentMessagePart::Reasoning { text }
                        | crate::domain::agent::AgentMessagePart::ReasoningSummary { text } => {
                            Some(text.as_str())
                        }
                        _ => None,
                    })
                    .collect();
                (!text.is_empty()).then_some(text)
            },
            content: segmented_content.unwrap_or(message.content),
            structured: None,
            tool_calls: message.tool_calls,
            usage: fw::Usage {
                input_tokens: response.token_usage.unwrap_or_default().input_tokens as u64,
                output_tokens: response.token_usage.unwrap_or_default().output_tokens as u64,
            },
            model_context,
        };
        Ok(match metrics {
            Some(metrics) => result.with_provider_metrics(metrics),
            None => result,
        })
    }
}

async fn emit_model_event(stream: &dyn fw::ChatStream, event: crate::domain::agent::ModelStreamEvent) {
    use crate::domain::agent::ModelStreamEventKind;
    match event.kind {
        ModelStreamEventKind::Text => if let Some(index) = event.output_index { stream.message_delta(index, &event.delta).await } else { stream.text_delta(&event.delta).await },
        ModelStreamEventKind::ReasoningSummary => stream.reasoning_delta(&event.delta).await,
        ModelStreamEventKind::Reasoning => {},
        ModelStreamEventKind::MessageStart => if let Some(index) = event.output_index { stream.message_start(index, event.phase.as_deref()).await },
        ModelStreamEventKind::MessageEnd => if let Some(index) = event.output_index { stream.message_end(index, event.phase.as_deref(), &event.delta).await },
        ModelStreamEventKind::ToolCallDelta => {},
        ModelStreamEventKind::ToolBoundary => if let Some(index) = event.output_index { stream.tool_boundary(index).await },
    }
}

fn provider_scope(
    context: &crate::domain::agent::AgentRunContext,
) -> Result<String, fw::ModelError> {
    use sha2::{Digest, Sha256};
    let model = &context.model;
    let identity = serde_json::to_vec(&(
        &context.user_code,
        &context.workspace_code,
        &model.api_key,
        &model.model_code,
        &model.model_name,
        &model.base_url,
        &model.api_path,
        model.protocol,
    ))
    .map_err(|e| fw::ModelError::Fatal(e.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(identity)))
}
