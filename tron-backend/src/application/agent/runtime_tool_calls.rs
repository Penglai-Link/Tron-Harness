use std::sync::Arc;

use crate::shared::event_bus::BusOutcome;
use crate::{
    application::agent::CHAT_ASK_USER_TOOL,
    application::short_term_memory::ToolTranscriptStatus,
    domain::agent::{
        AgentError, AgentRunContext, ExpandedDomainPersistenceMode, ToolCall, ToolContextDelta,
        ToolError, ToolExecutionResult, ToolExposureMode, ToolObservation, ToolRequest,
    },
};

use super::super::{DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL, TurnStateAdapter};
use super::ReactAgent;

impl ReactAgent {
    pub(super) async fn execute_tool_calls(
        &self,
        context: &mut AgentRunContext,
        tool_calls: &mut u32,
        turn_state: &mut TurnStateAdapter,
        last_tool_failure: &mut Option<String>,
        calls: Vec<ToolCall>,
    ) -> Result<(), AgentError> {
        if calls.len() > 1 {
            if calls
                .iter()
                .any(|call| call.tool_name == CHAT_ASK_USER_TOOL)
            {
                return Err(AgentError::InvalidConfiguration(
                    "chat_ask_user must be the only tool call in a model turn".into(),
                ));
            }
            self.execute_tool_call_batches(
                context,
                tool_calls,
                turn_state,
                last_tool_failure,
                calls,
            )
            .await
        } else {
            for call in calls {
                if *tool_calls >= context.limits.max_tool_calls {
                    self.fail_max_tool_calls(context)?;
                    return Err(AgentError::MaxToolCallsExceeded);
                }
                self.execute_tool_call(context, tool_calls, turn_state, last_tool_failure, call)
                    .await?;
            }

            Ok(())
        }
    }

    async fn execute_tool_call(
        &self,
        context: &mut AgentRunContext,
        tool_calls: &mut u32,
        turn_state: &mut TurnStateAdapter,
        last_tool_failure: &mut Option<String>,
        call: ToolCall,
    ) -> Result<(), AgentError> {
        *tool_calls += 1;
        let tool_call_id = format!("{}-tool-{}", context.run_code, *tool_calls);
        let sequence = *tool_calls;
        let model_call_id = call.call_id.clone();
        let invocation_id = format!(
            "{}:{}",
            context.run_code,
            model_call_id
                .as_deref()
                .filter(|id| !id.is_empty())
                .unwrap_or(&tool_call_id)
        );
        let tool_name = call.tool_name.clone();
        let tool_input = call.input.clone();
        let input_error = call.input_error.clone();
        self.publish_tool(
            &context,
            "agent.tool.selected",
            BusOutcome::Progress,
            Some(tool_name.as_str()),
            &tool_call_id,
            &tool_name,
            Some(&tool_input),
            None,
        )?;
        self.publish_tool(
            &context,
            "agent.tool.started",
            BusOutcome::Started,
            Some(tool_name.as_str()),
            &tool_call_id,
            &tool_name,
            Some(&tool_input),
            None,
        )?;
        if let Some(short_term_memory) = self.short_term_memory.as_ref() {
            short_term_memory
                .record_tool_started(context, &tool_name, model_call_id.as_deref(), sequence)
                .map_err(|error| AgentError::InvalidConfiguration(error.to_string()))?;
        }
        if let Some(input_error) = input_error {
            self.record_recoverable_tool_failure(
                context,
                &tool_call_id,
                &tool_name,
                model_call_id,
                turn_state,
                sequence,
                format!("tool input is invalid: {input_error}"),
                last_tool_failure,
            )?;
            return Ok(());
        }
        let registry = Arc::clone(&self.tool_registry);
        let context_snapshot = context.clone();
        let activity_publisher = self.clone();
        let activity_tool_call_id = tool_call_id.clone();
        let activity_tool_name = tool_name.clone();
        let call_tool_name = call.tool_name;
        let call_tool_input = call.input;
        let handle = tokio::task::spawn_blocking(move || {
            let mut publish_activity = |line: String| {
                if line.trim().is_empty() {
                    return;
                }
                let _ = activity_publisher.publish_tool(
                    &context_snapshot,
                    "agent.tool.activity",
                    BusOutcome::Progress,
                    Some("tool_activity"),
                    &activity_tool_call_id,
                    &activity_tool_name,
                    None,
                    Some(line.as_str()),
                );
            };
            registry.execute_allowed_with_progress(
                ToolRequest {
                    tool_name: call_tool_name,
                    input: call_tool_input,
                    invocation_id: Some(invocation_id),
                },
                &context_snapshot,
                &mut publish_activity,
            )
        });
        let execution_result = handle.await.map_err(|error| {
            AgentError::ToolFailure(format!("serial tool execution join failed: {error}"))
        })?;
        let mut awaiting_user_input = false;
        match execution_result {
            Ok(execution) => {
                let event_output = agui_tool_result_output(&execution);
                if let Err(error_message) =
                    self.apply_tool_context_delta(context, &execution.context_delta, turn_state)
                {
                    self.record_recoverable_tool_failure(
                        context,
                        &tool_call_id,
                        &tool_name,
                        model_call_id,
                        turn_state,
                        sequence,
                        error_message,
                        last_tool_failure,
                    )?;
                    return Ok(());
                }
                let observation = execution.observation;
                self.publish_tool(
                    &context,
                    "agent.tool.completed",
                    BusOutcome::Completed,
                    Some(observation.tool_name.as_str()),
                    &tool_call_id,
                    &observation.tool_name,
                    None,
                    Some(event_output.as_str()),
                )?;
                let message = self.record_tool_observation_message(
                    context,
                    turn_state,
                    &observation,
                    model_call_id,
                    *tool_calls,
                    ToolTranscriptStatus::Completed,
                )?;
                context.messages.push(message);
                let completed_tool_name = observation.tool_name.clone();
                context.observations.push(observation);
                awaiting_user_input = completed_tool_name == CHAT_ASK_USER_TOOL;
                collapse_ephemeral_expanded_domains(context, completed_tool_name.as_str());
                self.publish_tool(
                    &context,
                    "agent.observation.created",
                    BusOutcome::Progress,
                    Some("observation_created"),
                    &tool_call_id,
                    &tool_name,
                    None,
                    context.observations.last().map(|item| item.output.as_str()),
                )?;
            }
            Err(error) => {
                let error_message = error.to_string();
                self.record_recoverable_tool_failure(
                    context,
                    &tool_call_id,
                    &tool_name,
                    model_call_id,
                    turn_state,
                    sequence,
                    error_message,
                    last_tool_failure,
                )?;
            }
        }

        if awaiting_user_input {
            return Err(AgentError::AwaitingUserInput);
        }

        Ok(())
    }

    async fn execute_tool_call_batches(
        &self,
        context: &mut AgentRunContext,
        tool_calls: &mut u32,
        turn_state: &mut TurnStateAdapter,
        last_tool_failure: &mut Option<String>,
        calls: Vec<ToolCall>,
    ) -> Result<(), AgentError> {
        let mut remaining_calls = calls;
        while !remaining_calls.is_empty() {
            let remaining_budget = context.limits.max_tool_calls.saturating_sub(*tool_calls);
            if remaining_budget == 0 {
                self.fail_max_tool_calls(context)?;
                return Err(AgentError::MaxToolCallsExceeded);
            }

            let batch_size = remaining_calls
                .len()
                .min(context.limits.max_parallel_tool_calls as usize)
                .min(remaining_budget as usize);
            let batch_calls = remaining_calls.drain(..batch_size).collect::<Vec<_>>();
            self.execute_tool_calls_parallel_batch(
                context,
                tool_calls,
                turn_state,
                last_tool_failure,
                batch_calls,
            )
            .await?;
        }

        Ok(())
    }

    async fn execute_tool_calls_parallel_batch(
        &self,
        context: &mut AgentRunContext,
        tool_calls: &mut u32,
        turn_state: &mut TurnStateAdapter,
        last_tool_failure: &mut Option<String>,
        calls: Vec<ToolCall>,
    ) -> Result<(), AgentError> {
        let context_snapshot = Arc::new(context.clone());
        let mut planned_calls = Vec::with_capacity(calls.len());

        for call in calls {
            *tool_calls += 1;
            let tool_call_id = format!("{}-tool-{}", context.run_code, *tool_calls);
            let sequence = *tool_calls;
            let model_call_id = call.call_id.clone();
            let invocation_id = format!(
                "{}:{}",
                context.run_code,
                model_call_id
                    .as_deref()
                    .filter(|id| !id.is_empty())
                    .unwrap_or(&tool_call_id)
            );
            let tool_name = call.tool_name.clone();
            let tool_input = call.input.clone();
            let input_error = call.input_error.clone();

            self.publish_tool(
                &context,
                "agent.tool.selected",
                BusOutcome::Progress,
                Some(tool_name.as_str()),
                &tool_call_id,
                &tool_name,
                Some(&tool_input),
                None,
            )?;
            self.publish_tool(
                &context,
                "agent.tool.started",
                BusOutcome::Started,
                Some(tool_name.as_str()),
                &tool_call_id,
                &tool_name,
                Some(&tool_input),
                None,
            )?;
            if let Some(short_term_memory) = self.short_term_memory.as_ref() {
                short_term_memory
                    .record_tool_started(context, &tool_name, model_call_id.as_deref(), sequence)
                    .map_err(|error| AgentError::InvalidConfiguration(error.to_string()))?;
            }
            if let Some(input_error) = input_error {
                self.record_recoverable_tool_failure(
                    context,
                    &tool_call_id,
                    &tool_name,
                    model_call_id,
                    turn_state,
                    sequence,
                    format!("tool input is invalid: {input_error}"),
                    last_tool_failure,
                )?;
                continue;
            }

            let registry = Arc::clone(&self.tool_registry);
            let context_snapshot = Arc::clone(&context_snapshot);
            let activity_publisher = self.clone();
            let activity_tool_call_id = tool_call_id.clone();
            let activity_tool_name = tool_name.clone();
            let call_tool_name = call.tool_name;
            let call_tool_input = call.input;
            let handle = tokio::task::spawn_blocking(move || {
                let mut publish_activity = |line: String| {
                    if line.trim().is_empty() {
                        return;
                    }
                    let _ = activity_publisher.publish_tool(
                        &context_snapshot,
                        "agent.tool.activity",
                        BusOutcome::Progress,
                        Some("tool_activity"),
                        &activity_tool_call_id,
                        &activity_tool_name,
                        None,
                        Some(line.as_str()),
                    );
                };
                registry.execute_allowed_with_progress(
                    ToolRequest {
                        tool_name: call_tool_name,
                        input: call_tool_input,
                        invocation_id: Some(invocation_id),
                    },
                    &context_snapshot,
                    &mut publish_activity,
                )
            });

            planned_calls.push(ParallelToolCall {
                tool_call_id,
                tool_name,
                model_call_id,
                sequence,
                handle,
            });
        }

        for planned_call in planned_calls {
            let ParallelToolCall {
                tool_call_id,
                tool_name,
                model_call_id,
                sequence,
                handle,
            } = planned_call;
            let execution_result = handle.await.map_err(|error| {
                AgentError::ToolFailure(format!("parallel tool execution join failed: {error}"))
            })?;
            match execution_result {
                Ok(execution) => {
                    if let Err(error_message) =
                        self.apply_tool_context_delta(context, &execution.context_delta, turn_state)
                    {
                        self.record_recoverable_tool_failure(
                            context,
                            &tool_call_id,
                            &tool_name,
                            model_call_id,
                            turn_state,
                            sequence,
                            error_message,
                            last_tool_failure,
                        )?;
                        continue;
                    }
                    self.finish_parallel_tool_call(
                        context,
                        &tool_call_id,
                        &tool_name,
                        model_call_id,
                        turn_state,
                        sequence,
                        execution,
                    )?;
                }
                Err(error) => {
                    let error_message = error.to_string();
                    self.record_recoverable_tool_failure(
                        context,
                        &tool_call_id,
                        &tool_name,
                        model_call_id,
                        turn_state,
                        sequence,
                        error_message,
                        last_tool_failure,
                    )?;
                }
            }
        }

        Ok(())
    }

    fn fail_max_tool_calls(&self, context: &AgentRunContext) -> Result<(), AgentError> {
        self.publish(
            context,
            "agent.run.failed",
            BusOutcome::Failure,
            Some("max_tool_calls_exceeded"),
        )
    }

    fn finish_parallel_tool_call(
        &self,
        context: &mut AgentRunContext,
        tool_call_id: &str,
        tool_name: &str,
        model_call_id: Option<String>,
        turn_state: &mut TurnStateAdapter,
        sequence: u32,
        execution: ToolExecutionResult,
    ) -> Result<(), AgentError> {
        let event_output = agui_tool_result_output(&execution);
        let observation = execution.observation;
        self.publish_tool(
            &context,
            "agent.tool.completed",
            BusOutcome::Completed,
            Some(observation.tool_name.as_str()),
            tool_call_id,
            &observation.tool_name,
            None,
            Some(event_output.as_str()),
        )?;
        let message = self.record_tool_observation_message(
            context,
            turn_state,
            &observation,
            model_call_id,
            sequence,
            ToolTranscriptStatus::Completed,
        )?;
        context.messages.push(message);
        let completed_tool_name = observation.tool_name.clone();
        context.observations.push(observation);
        collapse_ephemeral_expanded_domains(context, completed_tool_name.as_str());
        self.publish_tool(
            &context,
            "agent.observation.created",
            BusOutcome::Progress,
            Some("observation_created"),
            tool_call_id,
            tool_name,
            None,
            context.observations.last().map(|item| item.output.as_str()),
        )?;

        Ok(())
    }

    fn record_recoverable_tool_failure(
        &self,
        context: &mut AgentRunContext,
        tool_call_id: &str,
        tool_name: &str,
        model_call_id: Option<String>,
        turn_state: &mut TurnStateAdapter,
        sequence: u32,
        error_message: String,
        last_tool_failure: &mut Option<String>,
    ) -> Result<(), AgentError> {
        self.publish_tool(
            &context,
            "agent.tool.failed",
            BusOutcome::Failure,
            Some("tool_failed"),
            tool_call_id,
            tool_name,
            None,
            Some(error_message.as_str()),
        )?;
        *last_tool_failure = Some(format!("{tool_name}: {error_message}"));
        let observation = ToolObservation {
            tool_name: tool_name.to_string(),
            output: error_message,
        };
        let message = self.record_tool_observation_message(
            context,
            turn_state,
            &observation,
            model_call_id,
            sequence,
            ToolTranscriptStatus::Failed,
        )?;
        context.messages.push(message);
        context.observations.push(observation);
        self.publish_tool(
            &context,
            "agent.observation.created",
            BusOutcome::Progress,
            Some("observation_created"),
            tool_call_id,
            tool_name,
            None,
            context.observations.last().map(|item| item.output.as_str()),
        )?;

        Ok(())
    }

    fn record_tool_observation_message(
        &self,
        context: &AgentRunContext,
        turn_state: &mut TurnStateAdapter,
        observation: &ToolObservation,
        model_call_id: Option<String>,
        sequence: u32,
        status: ToolTranscriptStatus,
    ) -> Result<crate::domain::agent::AgentMessage, AgentError> {
        if let Some(short_term_memory) = self.short_term_memory.as_ref() {
            short_term_memory
                .record_tool_result(context, observation, model_call_id, sequence, status)
                .map_err(|error| AgentError::InvalidConfiguration(error.to_string()))
        } else {
            Ok(turn_state.record_tool_observation(observation, model_call_id))
        }
    }
}

struct ParallelToolCall {
    tool_call_id: String,
    tool_name: String,
    model_call_id: Option<String>,
    sequence: u32,
    handle: tokio::task::JoinHandle<Result<ToolExecutionResult, ToolError>>,
}

impl ReactAgent {
    fn apply_tool_context_delta(
        &self,
        context: &mut AgentRunContext,
        delta: &ToolContextDelta,
        turn_state: &mut TurnStateAdapter,
    ) -> Result<(), String> {
        if delta.add_tools.is_empty() && delta.expanded_domain_codes.is_empty() {
            return Ok(());
        }
        let mut candidate_tools = context.tools.clone();
        for tool in &delta.add_tools {
            if candidate_tools
                .iter()
                .all(|existing| existing.tool_name != tool.tool_name)
            {
                candidate_tools.push(tool.clone());
            }
        }
        // Validate the merged set against the current runtime state, including earlier
        // results from the same parallel batch, before publishing expansion success.
        turn_state.prepare_tool_schema(
            context.model.protocol,
            &candidate_tools,
            self.tool_schema_limit(context),
        )?;
        context.tools = candidate_tools;
        for domain_code in &delta.expanded_domain_codes {
            if context
                .tool_exposure
                .expanded_domain_codes
                .iter()
                .all(|existing| !existing.eq_ignore_ascii_case(&domain_code))
            {
                context
                    .tool_exposure
                    .expanded_domain_codes
                    .push(domain_code.clone());
            }
        }
        Ok(())
    }
}

fn agui_tool_result_output(execution: &ToolExecutionResult) -> String {
    if execution.observation.tool_name != DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL
        || execution.context_delta.add_tools.is_empty()
    {
        return execution.observation.output.clone();
    }

    let mut output = serde_json::from_str::<serde_json::Value>(&execution.observation.output)
        .unwrap_or_else(|_| serde_json::json!({
            "result": execution.observation.output,
        }));
    let schemas = execution
        .context_delta
        .add_tools
        .iter()
        .map(|tool| {
            serde_json::json!({
                "toolName": tool.tool_name,
                "domainCode": tool.domain_code,
                "description": tool.description,
                "inputSchema": tool.input_schema,
            })
        })
        .collect::<Vec<_>>();
    if let Some(object) = output.as_object_mut() {
        object.insert("tools".to_string(), serde_json::Value::Array(schemas));
    }
    serde_json::to_string(&output).unwrap_or_else(|_| execution.observation.output.clone())
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
    let expanded_domain_codes = context.tool_exposure.expanded_domain_codes.clone();
    context.tools.retain(|tool| {
        tool.tool_name == DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL
            || tool.domain_code.as_deref().is_none_or(|domain_code| {
                expanded_domain_codes
                    .iter()
                    .all(|expanded| !expanded.eq_ignore_ascii_case(domain_code))
            })
    });
    context.tool_exposure.expanded_domain_codes.clear();
}
