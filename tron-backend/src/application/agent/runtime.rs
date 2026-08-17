use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use crate::{
    application::short_term_memory::{
        ModelResponseTranscriptFacts, PreparedModelContext, ShortTermMemoryService,
    },
    domain::agent::{
        AgentError, AgentModelConfig, AgentRunContext, AgentRunResult, ModelRequest, ModelResponse,
        ModelStreamEvent, ModelStreamEventKind, ModelTokenUsage, ToolRegistry,
    },
    shared::event_bus::{BusEventDraft, BusOutcome, BusRequestRef, EventBusHandle, PublishOutcome},
};

use super::{
    ActiveRunControl, TurnStateAdapter, TurnStateKind, request_estimate_snapshot,
    usage_audit_snapshot, usage_event_payload,
};

#[path = "runtime_tool_calls.rs"]
mod runtime_tool_calls;

#[path = "runtime_helpers.rs"]
mod runtime_helpers;

#[path = "runtime_compaction.rs"]
mod runtime_compaction;

#[path = "runtime_model_request.rs"]
mod runtime_model_request;

#[path = "runtime_steering.rs"]
mod runtime_steering;

use runtime_helpers::{
    agent_kind_label, compact_event_payload_value, compact_event_value,
    model_stream_delta_payloads, record_failure_on_short_term_memory_error,
    short_term_memory_error, validate_context,
};
use runtime_model_request::{
    estimate_tool_schema_tokens, model_request_facts, prepared_context_event_payload,
};
use runtime_steering::RuntimeSteering;

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
            let output = response.output_text();
            if response.finish == crate::domain::agent::ModelFinish::Final && !output.is_empty() {
                sink(ModelStreamEvent::text_delta(output));
            }
            Ok(response)
        })
    }
}

pub struct ReactAgentBuilder {
    model_client: Arc<dyn ModelClient>,
    tool_registry: Arc<ToolRegistry>,
    event_bus: EventBusHandle,
    short_term_memory: Option<ShortTermMemoryService>,
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
            short_term_memory: None,
        }
    }

    pub fn with_short_term_memory(mut self, service: Option<ShortTermMemoryService>) -> Self {
        self.short_term_memory = service;
        self
    }

    pub fn build(self) -> Result<ReactAgent, AgentError> {
        Ok(ReactAgent {
            model_client: self.model_client,
            tool_registry: self.tool_registry,
            event_bus: self.event_bus,
            short_term_memory: self.short_term_memory,
        })
    }
}

#[derive(Clone)]
pub struct ReactAgent {
    model_client: Arc<dyn ModelClient>,
    tool_registry: Arc<ToolRegistry>,
    event_bus: EventBusHandle,
    short_term_memory: Option<ShortTermMemoryService>,
}

impl ReactAgent {
    pub async fn run(&self, mut context: AgentRunContext) -> Result<AgentRunResult, AgentError> {
        self.run_with_model(&mut context, None, None).await
    }

    pub async fn run_stream(
        &self,
        mut context: AgentRunContext,
        sink: ModelStreamSink,
    ) -> Result<AgentRunResult, AgentError> {
        self.run_with_model(&mut context, Some(sink), None).await
    }

    async fn run_with_model(
        &self,
        context: &mut AgentRunContext,
        sink: Option<ModelStreamSink>,
        steer_control: Option<ActiveRunControl>,
    ) -> Result<AgentRunResult, AgentError> {
        let mut steering = RuntimeSteering::new(steer_control);
        steering.install_continuity_guidance(context);
        validate_context(&context)?;
        let mut turn_state = TurnStateAdapter::new(
            context.model.protocol,
            context.model.responses_stateful_enabled,
        );
        let turn_state_kind = turn_state.kind();
        self.publish(
            &context,
            "agent.run.requested",
            BusOutcome::Started,
            Some("run_requested"),
        )?;
        self.publish(&context, "agent.run.started", BusOutcome::Started, None)?;
        if let Some(short_term_memory) = self.short_term_memory.as_ref() {
            short_term_memory
                .record_run_started(context)
                .map_err(short_term_memory_error)?;
        }
        let mut step_count = 0;
        let mut tool_calls = 0;
        let mut token_usage: Option<ModelTokenUsage> = None;
        let mut overflow_retries = 0;
        let mut final_output = String::new();
        loop {
            if step_count >= context.limits.max_steps {
                self.publish(
                    &context,
                    "agent.run.failed",
                    BusOutcome::Failure,
                    Some("max_steps_exceeded"),
                )?;
                self.record_short_term_memory_failure(context, "max_steps_exceeded")?;
                return Err(AgentError::MaxStepsExceeded);
            }
            steering.inject_at_loop_boundary(context, self.short_term_memory.as_ref())?;
            step_count += 1;
            let prepared_context = if let Some(short_term_memory) = self.short_term_memory.as_ref()
            {
                record_failure_on_short_term_memory_error(
                    short_term_memory,
                    context,
                    "context_prepare_failed",
                    short_term_memory.prepare_model_context(context),
                )?
            } else {
                PreparedModelContext {
                    messages: context.messages.clone(),
                    rebased: false,
                    context_hash: None,
                    checkpoint_code: None,
                    estimated_tokens: 0,
                    message_token_estimate: 0,
                    tool_schema_token_estimate: estimate_tool_schema_tokens(&context.tools),
                    budget_breakdown: None,
                    over_budget_reason: None,
                    context_window_limit: context.model.context_window_tokens,
                    effective_input_budget: None,
                    output_token_reserve: context.model.max_tokens,
                    over_budget: false,
                    compaction_trigger_reached: false,
                    compaction_required: false,
                    compaction_tail_start_seq: None,
                    compaction_trigger_tokens: None,
                    effective_retained_history_tokens: None,
                }
            };
            if prepared_context.compaction_required
                && prepared_context.compaction_trigger_reached
                && self
                    .try_auto_context_compaction(context, &prepared_context)
                    .await?
            {
                continue;
            }
            if prepared_context.over_budget {
                let failure_reason = prepared_context
                    .over_budget_reason
                    .as_deref()
                    .unwrap_or("context_budget_exceeded");
                let mut payload = prepared_context_event_payload(&prepared_context);
                payload.push((
                    "failureCode".to_string(),
                    "context_budget_exceeded".to_string(),
                ));
                self.publish_with_payload(
                    &context,
                    "agent.run.failed",
                    BusOutcome::Failure,
                    Some(failure_reason),
                    payload,
                )?;
                if let Some(short_term_memory) = self.short_term_memory.as_ref() {
                    short_term_memory
                        .record_run_failed_with_budget(context, failure_reason, &prepared_context)
                        .map_err(short_term_memory_error)?;
                }
                return Err(AgentError::InvalidConfiguration(
                    "short-term memory context exceeded budget".into(),
                ));
            }
            context.messages = prepared_context.messages.clone();
            steering.inject_before_model_request(context, self.short_term_memory.as_ref())?;
            let request = turn_state.build_request(
                context,
                &context.messages,
                prepared_context.rebased,
                prepared_context.context_hash.as_deref(),
            );
            let request_estimate = request_estimate_snapshot(&request, context, &prepared_context);
            self.publish_with_payload(
                &context,
                "agent.model.requested",
                BusOutcome::Started,
                Some("model_requested"),
                {
                    let mut payload = usage_event_payload("usage_estimate_json", &request_estimate);
                    payload.extend(prepared_context_event_payload(&prepared_context));
                    payload
                },
            )?;
            if let Some(short_term_memory) = self.short_term_memory.as_ref() {
                record_failure_on_short_term_memory_error(
                    short_term_memory,
                    context,
                    "model_request_persist_failed",
                    short_term_memory.record_model_requested(
                        context,
                        model_request_facts(
                            &request,
                            context,
                            &prepared_context,
                            request_estimate.clone(),
                        ),
                    ),
                )?;
            }
            let output_started = Arc::new(Mutex::new(false));
            let response_result = if let Some(sink) = sink.clone() {
                let stream_sink = self.model_stream_event_sink(
                    context,
                    sink,
                    turn_state_kind,
                    Arc::clone(&output_started),
                );
                self.model_client
                    .generate_stream(&context.model, &request, stream_sink)
                    .await
            } else {
                self.model_client.generate(&context.model, &request).await
            };
            let response = match response_result {
                Ok(response) => response,
                Err(message) => {
                    match self
                        .try_recover_context_overflow(
                            context,
                            &prepared_context,
                            &message,
                            &output_started,
                            &mut overflow_retries,
                        )
                        .await
                    {
                        Ok(true) => continue,
                        Ok(false) => {}
                        Err(error) => {
                            self.record_short_term_memory_failure(
                                context,
                                "context_overflow_recovery_failed",
                            )?;
                            return Err(error);
                        }
                    }
                    self.publish(
                        &context,
                        "agent.run.failed",
                        BusOutcome::Failure,
                        Some("model_failure"),
                    )?;
                    self.record_short_term_memory_failure(context, "model_failure")?;
                    return Err(AgentError::ModelFailure(message));
                }
            };
            let next_token_usage = response.token_usage().map_or(token_usage, |usage| {
                Some(match token_usage {
                    Some(current) => current.add(usage),
                    None => usage,
                })
            });
            let usage_audit =
                usage_audit_snapshot(request_estimate.clone(), &response, next_token_usage);
            self.publish_with_payload(
                &context,
                "agent.model.responded",
                BusOutcome::Completed,
                Some("model_responded"),
                usage_event_payload("usage_audit_json", &usage_audit),
            )?;
            if let Some(short_term_memory) = self.short_term_memory.as_ref() {
                record_failure_on_short_term_memory_error(
                    short_term_memory,
                    context,
                    "assistant_message_persist_failed",
                    short_term_memory.record_model_response_with_facts(
                        context,
                        &response,
                        ModelResponseTranscriptFacts {
                            usage_audit: Some(usage_audit.clone()),
                        },
                    ),
                )?;
            }
            if let Some(usage) = response.token_usage() {
                token_usage = Some(match token_usage {
                    Some(current) => current.add(usage),
                    None => usage,
                });
            }
            if response.finish == crate::domain::agent::ModelFinish::ToolCalls
                && !response.assistant.parts.is_empty()
            {
                context.messages.push(response.assistant.clone());
            }
            turn_state.apply_model_response(&response, context.messages.len());

            match response.finish {
                crate::domain::agent::ModelFinish::Final => {
                    final_output.push_str(&response.output_text());
                    if steering.continue_after_final_response(
                        context,
                        self.short_term_memory.as_ref(),
                        turn_state_kind,
                        &response,
                    )? {
                        continue;
                    }
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
                    if let Some(short_term_memory) = self.short_term_memory.as_ref() {
                        record_failure_on_short_term_memory_error(
                            short_term_memory,
                            context,
                            "run_completed_persist_failed",
                            short_term_memory.record_run_completed(context),
                        )?;
                    }
                    return Ok(AgentRunResult {
                        run_code: context.run_code.clone(),
                        conversation_code: context.conversation_code.clone(),
                        workspace_code: context.workspace_code.clone(),
                        final_output,
                        step_count,
                        token_usage,
                        latest_request_estimate: Some(request_estimate),
                        latest_usage_audit: Some(usage_audit),
                        system_agent_session_id: response
                            .system_agent_session_id()
                            .map(str::to_string),
                        observations: context.observations.clone(),
                    });
                }
                crate::domain::agent::ModelFinish::ToolCalls => {
                    let calls = response.tool_calls();
                    if let Err(error) = self
                        .execute_tool_calls(context, &mut tool_calls, &mut turn_state, calls)
                        .await
                    {
                        self.record_short_term_memory_failure(context, "tool_failure")?;
                        return Err(error);
                    }
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

    fn record_short_term_memory_failure(
        &self,
        context: &AgentRunContext,
        reason: &str,
    ) -> Result<(), AgentError> {
        if let Some(short_term_memory) = self.short_term_memory.as_ref() {
            short_term_memory
                .record_run_failed(context, reason)
                .map_err(short_term_memory_error)?;
        }
        Ok(())
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
        if let Some(reasoning_effort) = context.model.reasoning_effort {
            draft = draft.payload_field(
                "reasoning_effort",
                reasoning_effort.storage_value().to_string(),
            );
        }
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
            draft = draft.payload_field(key, compact_event_payload_value(&value));
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
        turn_state_kind: TurnStateKind,
        output_started: Arc<Mutex<bool>>,
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
        Arc::new(move |event: ModelStreamEvent| {
            let event_name = turn_state_kind.stream_event_name(event.kind);
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
