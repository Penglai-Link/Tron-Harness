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
        calls: Vec<ToolCall>,
    ) -> Result<(), AgentError> {
        if calls.len() > 1 {
            if calls
                .iter()
                .any(|call| call.tool_name == CHAT_ASK_USER_TOOL)
            {
                return Err(AgentError::InvalidConfiguration(
                    "chat.ask_user must be the only tool call in a model turn".into(),
                ));
            }
            self.execute_tool_call_batches(context, tool_calls, turn_state, calls)
                .await
        } else {
            for call in calls {
                if *tool_calls >= context.limits.max_tool_calls {
                    self.fail_max_tool_calls(context)?;
                    return Err(AgentError::MaxToolCallsExceeded);
                }
                self.execute_tool_call(context, tool_calls, turn_state, call)
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
        call: ToolCall,
    ) -> Result<(), AgentError> {
        *tool_calls += 1;
        let tool_call_id = format!("{}-tool-{}", context.run_code, *tool_calls);
        let sequence = *tool_calls;
        let model_call_id = call.call_id.clone();
        let tool_name = call.tool_name.clone();
        let tool_input = call.input.clone();
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
                let observation = execution.observation;
                apply_tool_context_delta(context, execution.context_delta);
                self.publish_tool(
                    &context,
                    "agent.tool.completed",
                    BusOutcome::Completed,
                    Some(observation.tool_name.as_str()),
                    &tool_call_id,
                    &observation.tool_name,
                    None,
                    Some(observation.output.as_str()),
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
                self.publish_tool(
                    &context,
                    "agent.tool.failed",
                    BusOutcome::Failure,
                    Some("tool_failed"),
                    &tool_call_id,
                    &tool_name,
                    None,
                    Some(error_message.as_str()),
                )?;
                self.record_tool_failure_observation(
                    context,
                    &tool_call_id,
                    &tool_name,
                    model_call_id,
                    turn_state,
                    sequence,
                    error_message,
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
            self.execute_tool_calls_parallel_batch(context, tool_calls, turn_state, batch_calls)
                .await?;
        }

        Ok(())
    }

    async fn execute_tool_calls_parallel_batch(
        &self,
        context: &mut AgentRunContext,
        tool_calls: &mut u32,
        turn_state: &mut TurnStateAdapter,
        calls: Vec<ToolCall>,
    ) -> Result<(), AgentError> {
        let context_snapshot = Arc::new(context.clone());
        let mut planned_calls = Vec::with_capacity(calls.len());

        for call in calls {
            *tool_calls += 1;
            let tool_call_id = format!("{}-tool-{}", context.run_code, *tool_calls);
            let sequence = *tool_calls;
            let model_call_id = call.call_id.clone();
            let tool_name = call.tool_name.clone();
            let tool_input = call.input.clone();

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
                    self.publish_tool(
                        &context,
                        "agent.tool.failed",
                        BusOutcome::Failure,
                        Some("tool_failed"),
                        &tool_call_id,
                        &tool_name,
                        None,
                        Some(error_message.as_str()),
                    )?;
                    self.record_tool_failure_observation(
                        context,
                        &tool_call_id,
                        &tool_name,
                        model_call_id,
                        turn_state,
                        sequence,
                        error_message,
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
        let observation = execution.observation;
        apply_tool_context_delta(context, execution.context_delta);
        self.publish_tool(
            &context,
            "agent.tool.completed",
            BusOutcome::Completed,
            Some(observation.tool_name.as_str()),
            tool_call_id,
            &observation.tool_name,
            None,
            Some(observation.output.as_str()),
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

    fn record_tool_failure_observation(
        &self,
        context: &mut AgentRunContext,
        tool_call_id: &str,
        tool_name: &str,
        model_call_id: Option<String>,
        turn_state: &mut TurnStateAdapter,
        sequence: u32,
        error_message: String,
    ) -> Result<(), AgentError> {
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
