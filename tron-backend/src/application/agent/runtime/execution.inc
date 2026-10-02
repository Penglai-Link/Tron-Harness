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
        let host = self.framework_host.clone().ok_or_else(|| AgentError::InvalidConfiguration("Framework host was not configured".into()))?;
        host.run(self.clone(), context.clone(), sink, steer_control).await
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
            .attr("model_code", context.model.model_code.clone());
        if !context.model.model_name.trim().is_empty() {
            draft = draft.payload_field("model_name", context.model.model_name.clone());
        }
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
            outcome => {
                // Presentation delivery cannot change the SDK execution result. The bus
                // reports delivery failures to replay sinks, which expose incomplete replay.
                eprintln!("agent presentation delivery incomplete for run {}: {outcome:?}", context.run_code);
                Ok(())
            }
        }
    }

    fn model_stream_event_sink(
        &self,
        _context: &AgentRunContext,
        _turn_state_kind: TurnStateKind,
        sink: ModelStreamSink,
        _output_started: Arc<Mutex<bool>>,
    ) -> ModelStreamSink {
        // The SDK owns message/reasoning lifecycle. Preserve only the optional
        // product/raw transport callback; never synthesize a second protocol stream.
        sink
    }
}
