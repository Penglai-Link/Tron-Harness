//! Tron model policies used by the public framework's model adapter.
//! This performs one model turn; tool dispatch and run iteration belong to Framework.
use super::*;
use crate::domain::agent::{TokenEstimateSnapshot, TokenUsageAuditSnapshot};

impl ReactAgent {
    pub(crate) async fn framework_tool_catalog_update(&self, context: &AgentRunContext, bindings: &[eliteforge_agent_core::ToolBinding]) -> Result<Option<serde_json::Value>, AgentError> {
        match &self.short_term_memory {
            Some(memory) => memory.admitted_tool_catalog_update(context, bindings).await.map_err(short_term_memory_error),
            None => Ok(None),
        }
    }
    pub(crate) fn framework_dudes(&self)->Option<crate::application::dude::DudeService> {
        self.framework_host.as_ref().and_then(|h|h.dudes())
    }

    pub(crate) fn framework_knowledge_bases(
        &self,
    ) -> Option<crate::application::knowledge_base::KnowledgeBaseService> {
        self.framework_host
            .as_ref()
            .and_then(|host| host.knowledge_bases())
    }

    pub(crate) fn framework_topics(&self) -> Option<crate::application::topic::TopicService> {
        self.framework_host.as_ref().and_then(|host| host.topics())
    }

    pub(crate) fn framework_live_memory_source(
        &self,
        context: &AgentRunContext,
        id: &str,
    ) -> Result<
        (Vec<crate::domain::agent::AgentMessage>, Option<String>),
        crate::domain::short_term_memory::ShortTermMemoryError,
    > {
        self.short_term_memory
            .as_ref()
            .ok_or(crate::domain::short_term_memory::ShortTermMemoryError::StorageFailure)?
            .framework_data_source(context, id)
    }
    pub(crate) fn framework_record_context(
        &self,
        context: &AgentRunContext,
        prepared: &PreparedModelContext,
        report: &eliteforge_agent_core::ContextAssemblyReport,
    ) -> Result<Option<u64>, AgentError> {
        if let Some(memory) = &self.short_term_memory {
            return memory
                .record_framework_context(context, prepared, report)
                .map_err(short_term_memory_error);
        }
        Ok(None)
    }
    pub(crate) fn framework_context_config(
        &self,
    ) -> crate::application::short_term_memory::ShortTermMemoryConfig {
        self.short_term_memory
            .as_ref()
            .map(|memory| memory.framework_context_config().clone())
            .unwrap_or_default()
    }
    pub(crate) fn framework_compaction_model_config(
        &self,
        model: &AgentModelConfig,
    ) -> AgentModelConfig {
        self.short_term_memory
            .as_ref()
            .map(|memory| memory.compaction_model_config(model))
            .unwrap_or_else(|| model.clone())
    }
    pub(crate) async fn prepare_framework_context(
        &self,
        context: &mut AgentRunContext,
        turn_state: &mut TurnStateAdapter,
    ) -> Result<PreparedModelContext, AgentError> {
        if context.is_owning_dude() { context.model.responses_stateful_enabled = false; }
        if let Some(inspector) = self.directory_inspector.clone() {
            if crate::application::agent::refresh_execution_context(context, inspector).await? {
                // Stateful providers must replay the updated prefix, not retain stale cwd facts.
                *turn_state = TurnStateAdapter::new(
                    context.model.protocol,
                    context.model.responses_stateful_enabled,
                );
            }
        }
        let tool_schema = match turn_state.prepare_tool_schema(
            context.model.protocol,
            &context.tools,
            self.tool_schema_limit(context),
        ) {
            Ok(snapshot) => snapshot,
            Err(message) => {
                let failure_code = if message.starts_with("TOOL_SCHEMA_OVER_BUDGET:") {
                    "tool_schema_over_budget"
                } else {
                    "tool_schema_prepare_failed"
                };
                self.publish_with_payload(
                    context,
                    "agent.run.failed",
                    BusOutcome::Failure,
                    Some(&message),
                    vec![("failureCode".to_string(), failure_code.to_string())],
                )?;
                self.record_short_term_memory_failure(context, &message)?;
                return Err(AgentError::InvalidConfiguration(message));
            }
        };
        let prepared_context = if let Some(short_term_memory) = self.short_term_memory.as_ref() {
            record_failure_on_short_term_memory_error(
                short_term_memory,
                context,
                "context_prepare_failed",
                short_term_memory.prepare_model_context_with_tool_schema_estimate(
                    context,
                    tool_schema.estimated_tokens(),
                ),
            )?
        } else {
            let budget = self.model_context_budget_plan(context);
            let message_token_estimate =
                crate::application::short_term_memory::estimate_messages_tokens(&context.messages);
            let estimated_tokens =
                message_token_estimate.saturating_add(tool_schema.estimated_tokens());
            let over_budget = estimated_tokens > budget.effective_input_budget;
            PreparedModelContext {
                sources: None,
                messages: context.messages.clone(),
                rebased: false,
                context_hash: None,
                checkpoint_code: None,
                estimated_tokens,
                message_token_estimate,
                tool_schema_token_estimate: tool_schema.estimated_tokens(),
                budget_breakdown: None,
                over_budget_reason: over_budget
                    .then(|| "NON_COMPRESSIBLE_CONTEXT_OVER_EFFECTIVE_BUDGET".into()),
                context_window_limit: Some(budget.model_context_window),
                effective_input_budget: Some(budget.effective_input_budget),
                output_token_reserve: Some(budget.output_token_reserve),
                over_budget,
                compaction_trigger_reached: false,
                compaction_required: false,
                compaction_tail_start_seq: None,
                compaction_trigger_tokens: None,
                effective_retained_history_tokens: None,
            }
        };
        context.messages = prepared_context.messages.clone();
        Ok(prepared_context)
    }
    pub(crate) async fn framework_model_turn(
        &self,
        context: &mut AgentRunContext,
        turn_state: &mut TurnStateAdapter,
        previous_usage: Option<ModelTokenUsage>,
        sink: Option<ModelStreamSink>,
        mut prepared: Option<PreparedModelContext>,
        native_request: Option<&eliteforge_agent_core::ChatRequest>,
    ) -> Result<
        (
            ModelResponse,
            TokenEstimateSnapshot,
            TokenUsageAuditSnapshot,
            Option<ModelTokenUsage>,
        ),
        AgentError,
    > {
        let turn_state_kind = turn_state.kind();
        let mut token_usage = previous_usage;
        let prepared_context = match prepared.take() {
            Some(prepared) => prepared,
            None => self.prepare_framework_context(context, turn_state).await?,
        };
        context.messages = prepared_context.messages.clone();
        let request = turn_state.build_request(
            context,
            &context.messages,
            prepared_context.rebased,
            prepared_context.context_hash.as_deref(),
        );
        let request_estimate = request_estimate_snapshot(context, &prepared_context);
        let model_attempt_id = eliteforge_agent_core::RunId::new_v4().to_string();
        self.publish_with_payload(
            &context,
            "agent.model.requested",
            BusOutcome::Started,
            Some("model_requested"),
            {
                let mut payload = usage_event_payload("usage_estimate_json", &request_estimate);
                payload.extend(prepared_context_event_payload(&prepared_context));
                payload.push(("model_attempt_id".into(), model_attempt_id.clone()));
                payload
            },
        )?;
        // The admitted snapshot has served its purpose. Release its Arc before
        // appending tool calls so copy-on-write does not duplicate the history.
        drop(prepared_context);

        let model_started = std::time::Instant::now();
        let first_delta_at = Arc::new(Mutex::new(None::<std::time::Instant>));
        let output_started = Arc::new(Mutex::new(false));
        let stream_sink = sink.map(|sink| {
            let forward = self.model_stream_event_sink(context, turn_state_kind, sink, Arc::clone(&output_started));
            let first_delta_at = Arc::clone(&first_delta_at);
            Arc::new(move |event: ModelStreamEvent| {
                use crate::domain::agent::ModelStreamEventKind as Kind;
                if !event.delta.is_empty() && matches!(event.kind, Kind::Text | Kind::Reasoning | Kind::ReasoningSummary | Kind::ToolCallDelta | Kind::ToolBoundary) {
                    first_delta_at.lock().unwrap().get_or_insert_with(std::time::Instant::now);
                }
                forward(event);
            }) as ModelStreamSink
        });
        let response_result = match native_request {
            Some(native) => {
                self.model_client
                    .generate_native_once(&context.model, native, &request, stream_sink)
                    .await
            }
            None => match stream_sink {
                Some(sink) => {
                    self.model_client
                        .generate_stream(&context.model, &request, sink)
                        .await
                }
                None => {
                    self.model_client
                        .generate_once(&context.model, &request)
                        .await
                }
            },
        };
        // Stop before argument normalization, router persistence, event publication,
        // tool dispatch or the Agent's final response. Durations use one monotonic clock.
        let model_completed = std::time::Instant::now();
        let first_delta = *first_delta_at.lock().unwrap();
        let model_timing = serde_json::json!({
            "requestMs": model_completed.duration_since(model_started).as_secs_f64() * 1000.0,
            "ttftMs": first_delta.map(|first| first.duration_since(model_started).as_secs_f64() * 1000.0),
            "decodeMs": first_delta.map(|first| model_completed.duration_since(first).as_secs_f64() * 1000.0),
        });
        let mut response = match response_result {
            Ok(response) => response,
            Err(message) => {
                self.publish_with_payload(
                    context,
                    "agent.model.failed",
                    BusOutcome::Failure,
                    Some("model_failed"),
                    {
                        let mut payload = vec![
                            ("message".into(), message.clone()),
                            ("model_attempt_id".into(), model_attempt_id.clone()),
                        ];
                        if let Some(request_id) = context
                            .request_id
                            .as_ref()
                            .filter(|id| !id.trim().is_empty())
                        {
                            payload.push(("request_id".into(), request_id.clone()));
                        }
                        payload
                    },
                )?;
                return Err(AgentError::ModelFailure(message));
            }
        };
        crate::application::agent::tool_arguments::normalize(&mut response, &context.tools);
        if let (Some(router), Some(route)) = (
            self.auto_model_router.as_ref(),
            context.auto_model_route.as_mut(),
        ) && let Err(message) =
            router.mark_success(&context.workspace_code, route, &context.model)
        {
            self.publish(
                context,
                "agent.model.route.persist_failed",
                BusOutcome::Failure,
                Some(&message),
            )?;
        }
        let next_token_usage = response.token_usage().map_or(token_usage, |usage| {
            Some(match token_usage {
                Some(current) => current.add(usage),
                None => usage,
            })
        });
        for part in &mut response.assistant.parts {
            if let crate::domain::agent::AgentMessagePart::ToolCall { call_id, .. } = part {
                if call_id.as_ref().is_none_or(|id| id.is_empty()) {
                    *call_id = Some(format!("call-{}", eliteforge_agent_core::RunId::new_v4()));
                }
            }
        }
        let usage_audit =
            usage_audit_snapshot(request_estimate.clone(), &response, next_token_usage);
        self.publish_with_payload(
            &context,
            "agent.model.responded",
            BusOutcome::Completed,
            Some("model_responded"),
            {
                let mut payload = usage_event_payload("usage_audit_json", &usage_audit);
                payload.push(("model_attempt_id".into(), model_attempt_id));
                payload.push(("model_timing_json".into(), model_timing.to_string()));
                payload
            },
        )?;

        if let Some(usage) = response.token_usage() {
            token_usage = Some(match token_usage {
                Some(current) => current.add(usage),
                None => usage,
            });
        }
        if response.finish == crate::domain::agent::ModelFinish::ToolCalls
            && !response.assistant.parts.is_empty()
        {
            std::sync::Arc::make_mut(&mut context.messages).extend(response.transcript_messages());
        }
        turn_state.apply_model_response(&response, context.messages.len());

        Ok((response, request_estimate, usage_audit, token_usage))
    }
}

impl ReactAgent {
    pub(crate) fn framework_started(&self, context: &AgentRunContext) -> Result<(), AgentError> {
        validate_context(context)?;
        Ok(())
    }
    pub(crate) fn framework_finished(
        &self,
        context: &AgentRunContext,
        result: &Result<eliteforge_agent_core::RunResult, eliteforge_agent_core::AgentError>,
    ) -> Result<(), AgentError> {
        if let Err(error) = result {
            self.record_short_term_memory_failure(context, &error.to_string())?;
        }
        Ok(())
    }
    pub(crate) fn framework_steering(
        &self,
        context: &AgentRunContext,
        inputs: Vec<super::super::AgentSteerInput>,
    ) -> Result<Vec<crate::domain::agent::AgentMessage>, AgentError> {
        runtime_steering::record_inputs(self.short_term_memory.as_ref(), context, inputs)
    }
    pub(crate) fn framework_registry(&self) -> Arc<ToolRegistry> {
        self.tool_registry.clone()
    }
}
