use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use crate::{
    domain::agent::{
        AgentError, AgentKind, AgentModelConfig, AgentRunContext, AgentRunResult,
        ExpandedDomainPersistenceMode, ModelRequest, ModelResponse, ModelStreamEvent,
        ModelStreamEventKind, ModelTokenUsage, ToolContextDelta, ToolExposureMode, ToolRegistry,
        responses_input_items_from_messages,
    },
    shared::event_bus::{BusEventDraft, BusOutcome, BusRequestRef, EventBusHandle, PublishOutcome},
};

use super::{DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL, discovery_expand_tool_definition};

#[path = "runtime_tool_calls.rs"]
mod runtime_tool_calls;

pub type ModelStreamSink = Arc<dyn Fn(ModelStreamEvent) + Send + Sync>;

pub trait ModelClient: Send + Sync {
    fn generate<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>>;

    fn generate_stream<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
        sink: ModelStreamSink,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            let response = self.generate(config, request).await?;
            if let ModelResponse::FinalOutput { output, .. } = &response
                && !output.is_empty()
            {
                sink(ModelStreamEvent::text_delta(output.clone()));
            }
            Ok(response)
        })
    }
}

pub struct ReactAgentBuilder {
    model_client: Arc<dyn ModelClient>,
    tool_registry: Arc<ToolRegistry>,
    event_bus: EventBusHandle,
}

impl ReactAgentBuilder {
    pub fn new(
        model_client: Arc<dyn ModelClient>,
        tool_registry: Arc<ToolRegistry>,
        event_bus: EventBusHandle,
    ) -> Self {
        Self {
            model_client,
            tool_registry,
            event_bus,
        }
    }

    pub fn build(self) -> Result<ReactAgent, AgentError> {
        Ok(ReactAgent {
            model_client: self.model_client,
            tool_registry: self.tool_registry,
            event_bus: self.event_bus,
        })
    }
}

#[derive(Clone)]
pub struct ReactAgent {
    model_client: Arc<dyn ModelClient>,
    tool_registry: Arc<ToolRegistry>,
    event_bus: EventBusHandle,
}

#[derive(Debug, Default, Clone)]
struct ResponsesContinuationState {
    previous_response_id: Option<String>,
    consumed_message_count: usize,
}

impl ResponsesContinuationState {
    fn build_request(&self, context: &AgentRunContext) -> ModelRequest {
        let current_messages = self.current_messages(context);
        ModelRequest {
            messages: current_messages.to_vec(),
            tools: context.tools.clone(),
            system_agent_session_id: context.system_agent_session_id.clone(),
            response_input_items: Some(responses_input_items_from_messages(current_messages)),
            previous_response_id: self.previous_response_id.clone(),
        }
    }

    fn apply_response(&mut self, response: &ModelResponse, message_count: usize) {
        self.previous_response_id = response.response_id().map(str::to_owned);
        self.consumed_message_count = message_count;
    }

    fn current_messages<'a>(&self, context: &'a AgentRunContext) -> &'a [crate::domain::agent::AgentMessage] {
        let start = self.consumed_message_count.min(context.messages.len());
        &context.messages[start..]
    }
}

impl ReactAgent {
    pub async fn run(&self, mut context: AgentRunContext) -> Result<AgentRunResult, AgentError> {
        self.run_with_model(&mut context, None).await
    }

    pub async fn run_stream(
        &self,
        mut context: AgentRunContext,
        sink: ModelStreamSink,
    ) -> Result<AgentRunResult, AgentError> {
        self.run_with_model(&mut context, Some(sink)).await
    }

    async fn run_with_model(
        &self,
        context: &mut AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Result<AgentRunResult, AgentError> {
        validate_context(&context)?;
        let mut responses_state = if context.model.protocol == crate::domain::agent::AgentModelProtocol::Responses
            && context.model.responses_stateful_enabled
        {
            Some(ResponsesContinuationState::default())
        } else {
            None
        };
        self.publish(
            &context,
            "agent.run.requested",
            BusOutcome::Started,
            Some("run_requested"),
        )?;
        self.publish(&context, "agent.run.started", BusOutcome::Started, None)?;
        let mut step_count = 0;
        let mut tool_calls = 0;
        let mut token_usage: Option<ModelTokenUsage> = None;

        loop {
            if step_count >= context.limits.max_steps {
                self.publish(
                    &context,
                    "agent.run.failed",
                    BusOutcome::Failure,
                    Some("max_steps_exceeded"),
                )?;
                return Err(AgentError::MaxStepsExceeded);
            }
            step_count += 1;
            let request = match responses_state.as_ref() {
                Some(state) => state.build_request(context),
                None => model_request_from_context(&context),
            };
            self.publish(
                &context,
                "agent.model.requested",
                BusOutcome::Started,
                Some("model_requested"),
            )?;
            let response_result = if let Some(sink) = sink.clone() {
                let stream_sink = self.model_stream_event_sink(context, sink);
                self.model_client
                    .generate_stream(&context.model, &request, stream_sink)
                    .await
            } else {
                self.model_client.generate(&context.model, &request).await
            };
            let response = match response_result {
                Ok(response) => response,
                Err(message) => {
                    self.publish(
                        &context,
                        "agent.run.failed",
                        BusOutcome::Failure,
                        Some("model_failure"),
                    )?;
                    return Err(AgentError::ModelFailure(message));
                }
            };
            self.publish(
                &context,
                "agent.model.responded",
                BusOutcome::Completed,
                Some("model_responded"),
            )?;
            if let Some(usage) = response.token_usage() {
                token_usage = Some(match token_usage {
                    Some(current) => current.add(usage),
                    None => usage,
                });
            }
            if let Some(state) = responses_state.as_mut() {
                state.apply_response(&response, context.messages.len());
            }

            match response {
                ModelResponse::FinalOutput {
                    output,
                    system_agent_session_id,
                    ..
                } => {
                    self.publish(
                        &context,
                        "agent.output.created",
                        BusOutcome::Completed,
                        Some("output_created"),
                    )?;
                    self.publish(
                        &context,
                        "agent.run.completed",
                        BusOutcome::Completed,
                        Some("run_completed"),
                    )?;
                    return Ok(AgentRunResult {
                        run_code: context.run_code.clone(),
                        conversation_code: context.conversation_code.clone(),
                        workspace_code: context.workspace_code.clone(),
                        final_output: output,
                        step_count,
                        token_usage,
                        system_agent_session_id,
                        observations: context.observations.clone(),
                    });
                }
                ModelResponse::ToolCalls {
                    calls,
                    ..
                } => {
                    self.execute_tool_calls(context, &mut tool_calls, calls)
                        .await?;
                }
            }
        }
    }

    fn publish(
        &self,
        context: &AgentRunContext,
        event_name: &'static str,
        outcome: BusOutcome,
        summary: Option<&str>,
    ) -> Result<(), AgentError> {
        self.publish_with_payload(context, event_name, outcome, summary, Vec::new())
    }

    fn publish_tool(
        &self,
        context: &AgentRunContext,
        event_name: &'static str,
        outcome: BusOutcome,
        summary: Option<&str>,
        tool_call_id: &str,
        tool_name: &str,
        tool_input: Option<&serde_json::Value>,
        tool_output: Option<&str>,
    ) -> Result<(), AgentError> {
        let mut payload_fields = vec![
            (
                "tool_call_id".to_string(),
                compact_event_value(tool_call_id),
            ),
            ("tool_name".to_string(), compact_event_value(tool_name)),
        ];
        if let Some(tool_input) = tool_input {
            let value = serde_json::to_string(tool_input).unwrap_or_else(|_| "{}".into());
            payload_fields.push(("tool_args_json".to_string(), value));
        }
        if let Some(tool_output) = tool_output {
            payload_fields.push(("tool_result_json".to_string(), tool_output.to_string()));
        }

        self.publish_with_payload(context, event_name, outcome, summary, payload_fields)
    }

    fn publish_with_payload(
        &self,
        context: &AgentRunContext,
        event_name: &'static str,
        outcome: BusOutcome,
        summary: Option<&str>,
        payload_fields: Vec<(String, String)>,
    ) -> Result<(), AgentError> {
        let mut draft = BusEventDraft::agui(event_name, "agent_runtime")
            .outcome(outcome)
            .subject("agent_run", context.run_code.clone())
            .attr("workspace_code", context.workspace_code.clone())
            .attr("agent_kind", agent_kind_label(context.agent_kind))
            .attr("runtime_kind", "in_process")
            .attr("model_code", context.model.model_code.clone())
            .payload_field("model_name", context.model.model_name.clone());
        if let Some(context_window_tokens) = context.model.context_window_tokens {
            draft = draft.payload_field("context_window_limit", context_window_tokens.to_string());
        }
        if let Some(summary) = summary {
            draft = draft.summary(summary);
        }
        if let Some(conversation_code) = &context.conversation_code {
            draft = draft.attr("conversation_code", conversation_code.clone());
        }
        if let Some(user_code) = &context.user_code {
            draft = draft.attr("user_code", user_code.clone());
        }
        if let Some(app_code) = &context.execution_scope.app_code {
            draft = draft.attr("app_code", app_code.clone());
        }
        if let Some(task_code) = &context.execution_scope.task_code {
            draft = draft.attr("task_code", task_code.clone());
        }
        for (key, value) in payload_fields {
            draft = draft.payload_field(key, value);
        }
        if let Some(stream_key) = &context.stream_key {
            draft = draft.request(BusRequestRef::with_stream_key(stream_key.clone()));
        }

        match self.event_bus.publish(draft) {
            PublishOutcome::Disabled | PublishOutcome::Accepted { .. } => Ok(()),
            outcome => Err(AgentError::EventPublishFailed(format!("{outcome:?}"))),
        }
    }

    fn model_stream_event_sink(
        &self,
        context: &AgentRunContext,
        sink: ModelStreamSink,
    ) -> ModelStreamSink {
        let event_bus = self.event_bus.clone();
        let run_code = context.run_code.clone();
        let workspace_code = context.workspace_code.clone();
        let agent_kind = context.agent_kind;
        let conversation_code = context.conversation_code.clone();
        let stream_key = context.stream_key.clone();
        let model_code = context.model.model_code.clone();
        let model_name = context.model.model_name.clone();
        let context_window_tokens = context
            .model
            .context_window_tokens
            .map(|value| value.to_string());
        let output_started = Arc::new(Mutex::new(false));
        Arc::new(move |event: ModelStreamEvent| {
            let event_name = match event.kind {
                ModelStreamEventKind::Text => "agent.output.delta",
                ModelStreamEventKind::Reasoning => "agent.reasoning.raw_delta",
                ModelStreamEventKind::ReasoningSummary => "agent.reasoning.delta",
            };
            if event.kind == ModelStreamEventKind::Text {
                let mut started = output_started
                    .lock()
                    .expect("agent output start mutex poisoned");
                if !*started {
                    *started = true;
                    let mut start_draft =
                        BusEventDraft::agui("agent.output.started", "agent_runtime")
                            .outcome(BusOutcome::Started)
                            .subject("agent_run", run_code.clone())
                            .attr("workspace_code", workspace_code.clone())
                            .attr("agent_kind", agent_kind_label(agent_kind))
                            .attr("runtime_kind", "in_process")
                            .attr("model_code", model_code.clone())
                            .payload_field("model_name", model_name.clone());
                    if let Some(context_window_tokens) = &context_window_tokens {
                        start_draft = start_draft
                            .payload_field("context_window_limit", context_window_tokens.clone());
                    }
                    if let Some(conversation_code) = &conversation_code {
                        start_draft =
                            start_draft.attr("conversation_code", conversation_code.clone());
                    }
                    if let Some(stream_key) = &stream_key {
                        start_draft =
                            start_draft.request(BusRequestRef::with_stream_key(stream_key.clone()));
                    }
                    let _ = event_bus.publish(start_draft);
                }
            }
            for delta_payload in model_stream_delta_payloads(&event.delta) {
                let mut draft = BusEventDraft::agui(event_name, "agent_runtime")
                    .outcome(BusOutcome::Progress)
                    .subject("agent_run", run_code.clone())
                    .attr("workspace_code", workspace_code.clone())
                    .attr("agent_kind", agent_kind_label(agent_kind))
                    .attr("runtime_kind", "in_process")
                    .attr("model_code", model_code.clone())
                    .payload_field("model_name", model_name.clone())
                    .payload_field("delta_json", delta_payload);
                if let Some(context_window_tokens) = &context_window_tokens {
                    draft =
                        draft.payload_field("context_window_limit", context_window_tokens.clone());
                }
                if let Some(conversation_code) = &conversation_code {
                    draft = draft.attr("conversation_code", conversation_code.clone());
                }
                if let Some(stream_key) = &stream_key {
                    draft = draft.request(BusRequestRef::with_stream_key(stream_key.clone()));
                }
                let _ = event_bus.publish(draft);
            }
            sink(event);
        })
    }
}

fn agent_kind_label(agent_kind: AgentKind) -> &'static str {
    match agent_kind {
        AgentKind::React => "react",
        AgentKind::SystemCli => "system_cli",
    }
}

fn model_stream_delta_payloads(delta: &str) -> Vec<String> {
    const MAX_DELTA_JSON_BYTES: usize = 90;
    let mut payloads = Vec::new();
    let mut current = String::new();

    for character in delta.chars() {
        let mut candidate = current.clone();
        candidate.push(character);
        let candidate_json = serde_json::to_string(&candidate).unwrap_or_else(|_| "\"\"".into());
        if !current.is_empty() && candidate_json.len() > MAX_DELTA_JSON_BYTES {
            payloads.push(serde_json::to_string(&current).unwrap_or_else(|_| "\"\"".into()));
            current.clear();
        }
        current.push(character);
    }

    if !current.is_empty() {
        payloads.push(serde_json::to_string(&current).unwrap_or_else(|_| "\"\"".into()));
    }
    payloads
}

fn compact_event_value(value: &str) -> String {
    const LIMIT: usize = 96;
    if value.len() <= LIMIT {
        return value.to_string();
    }

    let mut end = LIMIT.saturating_sub(3);
    while !value.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    format!("{}...", &value[..end])
}

fn validate_context(context: &AgentRunContext) -> Result<(), AgentError> {
    if context.run_code.trim().is_empty() {
        return Err(AgentError::InvalidConfiguration(
            "runCode is required".into(),
        ));
    }
    if context.workspace_code.trim().is_empty() {
        return Err(AgentError::InvalidConfiguration(
            "workspaceCode is required".into(),
        ));
    }
    if context.model.model_name.trim().is_empty() {
        return Err(AgentError::InvalidConfiguration(
            "modelName is required".into(),
        ));
    }
    if context.limits.max_steps == 0 {
        return Err(AgentError::InvalidConfiguration(
            "maxSteps must be greater than zero".into(),
        ));
    }
    if context.limits.max_tool_calls == 0 {
        return Err(AgentError::InvalidConfiguration(
            "maxToolCalls must be greater than zero".into(),
        ));
    }
    if context.limits.max_parallel_tool_calls == 0 {
        return Err(AgentError::InvalidConfiguration(
            "maxParallelToolCalls must be greater than zero".into(),
        ));
    }
    Ok(())
}

fn model_request_from_context(context: &AgentRunContext) -> ModelRequest {
    ModelRequest {
        messages: context.messages.clone(),
        tools: context.tools.clone(),
        system_agent_session_id: context.system_agent_session_id.clone(),
        response_input_items: None,
        previous_response_id: None,
    }
}

fn format_tool_observation_message(observation: &crate::domain::agent::ToolObservation) -> String {
    format!(
        "tool:{}\nresult:\n{}",
        observation.tool_name, observation.output
    )
}

fn apply_tool_context_delta(context: &mut AgentRunContext, delta: ToolContextDelta) {
    if !delta.add_tools.is_empty() {
        for tool in delta.add_tools {
            if context
                .tools
                .iter()
                .all(|existing| existing.tool_name != tool.tool_name)
            {
                context.tools.push(tool);
            }
        }
    }
    for domain_code in delta.expanded_domain_codes {
        if context
            .tool_exposure
            .expanded_domain_codes
            .iter()
            .all(|existing| !existing.eq_ignore_ascii_case(&domain_code))
        {
            context
                .tool_exposure
                .expanded_domain_codes
                .push(domain_code);
        }
    }
}

fn collapse_ephemeral_expanded_domains(context: &mut AgentRunContext, tool_name: &str) {
    if context.tool_exposure.tool_exposure_mode != Some(ToolExposureMode::DomainDiscovery) {
        return;
    }
    if context.tool_exposure.expanded_domain_persistence_mode
        != Some(ExpandedDomainPersistenceMode::EphemeralExpanded)
    {
        return;
    }
    if context.tool_exposure.expanded_domain_codes.is_empty() {
        return;
    }
    if tool_name == DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL {
        return;
    }
    context.tools = vec![discovery_expand_tool_definition(&context.locale)];
    context.tool_exposure.expanded_domain_codes.clear();
}
