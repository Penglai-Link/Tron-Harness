use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    domain::agent::{
        AgentError, AgentMessage, AgentModelConfig, AgentRunContext, AgentRunResult, ModelRequest,
        ModelResponse, ModelStreamEvent, ModelStreamEventKind, ToolRegistry, ToolRequest,
    },
    shared::event_bus::{BusEventDraft, BusOutcome, BusRequestRef, EventBusHandle, PublishOutcome},
};

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
            if let ModelResponse::FinalOutput(output) = &response
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
        self.publish(&context, "agent.run.started", BusOutcome::Started, None)?;
        let mut step_count = 0;
        let mut tool_calls = 0;

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
            let request = model_request_from_context(&context);
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

            match response {
                ModelResponse::FinalOutput(output) => {
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
                        observations: context.observations.clone(),
                    });
                }
                ModelResponse::ToolCall(call) => {
                    if tool_calls >= context.limits.max_tool_calls {
                        self.publish(
                            &context,
                            "agent.run.failed",
                            BusOutcome::Failure,
                            Some("max_tool_calls_exceeded"),
                        )?;
                        return Err(AgentError::MaxStepsExceeded);
                    }
                    tool_calls += 1;
                    self.publish(
                        &context,
                        "agent.tool.selected",
                        BusOutcome::Progress,
                        Some(call.tool_name.as_str()),
                    )?;
                    self.publish(
                        &context,
                        "agent.tool.started",
                        BusOutcome::Started,
                        Some(call.tool_name.as_str()),
                    )?;
                    match self.tool_registry.execute_allowed(
                        ToolRequest {
                            tool_name: call.tool_name,
                            input: call.input,
                        },
                        &context,
                    ) {
                        Ok(observation) => {
                            self.publish(
                                &context,
                                "agent.tool.completed",
                                BusOutcome::Completed,
                                Some(observation.tool_name.as_str()),
                            )?;
                            context.messages.push(AgentMessage::tool(format!(
                                "tool:{}",
                                observation.tool_name
                            )));
                            context.observations.push(observation);
                            self.publish(
                                &context,
                                "agent.observation.created",
                                BusOutcome::Progress,
                                Some("observation_created"),
                            )?;
                        }
                        Err(error) => {
                            self.publish(
                                &context,
                                "agent.tool.failed",
                                BusOutcome::Failure,
                                Some("tool_failed"),
                            )?;
                            self.publish(
                                &context,
                                "agent.run.failed",
                                BusOutcome::Failure,
                                Some("tool_failure"),
                            )?;
                            return Err(error.into());
                        }
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
        let mut draft = BusEventDraft::agui(event_name, "agent_runtime")
            .outcome(outcome)
            .subject("agent_run", context.run_code.clone())
            .attr("workspace_code", context.workspace_code.clone())
            .attr("agent_kind", "react")
            .attr("runtime_kind", "in_process")
            .attr("model_code", context.model.model_code.clone());
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
        let conversation_code = context.conversation_code.clone();
        let stream_key = context.stream_key.clone();
        let model_code = context.model.model_code.clone();
        Arc::new(move |event: ModelStreamEvent| {
            let event_name = match event.kind {
                ModelStreamEventKind::Text => "agent.output.delta",
                ModelStreamEventKind::Reasoning => "agent.reasoning.delta",
            };
            let mut draft = BusEventDraft::agui(event_name, "agent_runtime")
                .outcome(BusOutcome::Progress)
                .subject("agent_run", run_code.clone())
                .attr("workspace_code", workspace_code.clone())
                .attr("agent_kind", "react")
                .attr("runtime_kind", "in_process")
                .attr("model_code", model_code.clone())
                .payload_field(
                    "delta_json",
                    serde_json::to_string(&event.delta).unwrap_or_else(|_| "\"\"".into()),
                );
            if let Some(conversation_code) = &conversation_code {
                draft = draft.attr("conversation_code", conversation_code.clone());
            }
            if let Some(stream_key) = &stream_key {
                draft = draft.request(BusRequestRef::with_stream_key(stream_key.clone()));
            }
            let _ = event_bus.publish(draft);
            sink(event);
        })
    }
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
    Ok(())
}

fn model_request_from_context(context: &AgentRunContext) -> ModelRequest {
    let mut messages = context.messages.clone();
    for observation in &context.observations {
        messages.push(AgentMessage::tool(format!(
            "tool:{}",
            observation.tool_name
        )));
    }
    ModelRequest {
        messages,
        tools: context.tools.clone(),
    }
}
