use std::sync::Arc;

use crate::domain::agent::{
    AgentError, AgentRunContext, ToolCall, ToolError, ToolExecutionResult, ToolRequest,
};
use crate::shared::event_bus::BusOutcome;

use super::{ReactAgent, apply_tool_context_delta, collapse_ephemeral_expanded_domains};
use super::super::TurnStateAdapter;

impl ReactAgent {
    pub(super) async fn execute_tool_calls(
        &self,
        context: &mut AgentRunContext,
        tool_calls: &mut u32,
        turn_state: &mut TurnStateAdapter,
        calls: Vec<ToolCall>,
    ) -> Result<(), AgentError> {
        if calls.len() > 1 {
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
        match self.tool_registry.execute_allowed(
            ToolRequest {
                tool_name: call.tool_name,
                input: call.input,
            },
            &context,
        ) {
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
                let message = turn_state.record_tool_observation(&observation, model_call_id);
                context.messages.push(message);
                let completed_tool_name = observation.tool_name.clone();
                context.observations.push(observation);
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
                self.publish(
                    &context,
                    "agent.run.failed",
                    BusOutcome::Failure,
                    Some("tool_failure"),
                )?;
                return Err(error.into());
            }
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

            let registry = Arc::clone(&self.tool_registry);
            let context_snapshot = Arc::clone(&context_snapshot);
            let call_tool_name = call.tool_name;
            let call_tool_input = call.input;
            let handle = tokio::task::spawn_blocking(move || {
                registry.execute_allowed(
                    ToolRequest {
                        tool_name: call_tool_name,
                        input: call_tool_input,
                    },
                    &context_snapshot,
                )
            });

            planned_calls.push(ParallelToolCall {
                tool_call_id,
                tool_name,
                model_call_id,
                handle,
            });
        }

        let mut first_error: Option<AgentError> = None;
        for planned_call in planned_calls {
            let ParallelToolCall {
                tool_call_id,
                tool_name,
                model_call_id,
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
                    if first_error.is_none() {
                        first_error = Some(error.into());
                    }
                }
            }
        }

        if let Some(error) = first_error {
            self.publish(
                &context,
                "agent.run.failed",
                BusOutcome::Failure,
                Some("tool_failure"),
            )?;
            return Err(error);
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
        let message = turn_state.record_tool_observation(&observation, model_call_id);
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
}

struct ParallelToolCall {
    tool_call_id: String,
    tool_name: String,
    model_call_id: Option<String>,
    handle: tokio::task::JoinHandle<Result<ToolExecutionResult, ToolError>>,
}
