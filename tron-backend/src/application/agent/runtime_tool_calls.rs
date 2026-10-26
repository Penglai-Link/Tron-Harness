use crate::domain::agent::{AgentError, AgentRunContext, ToolObservation};
use crate::shared::event_bus::BusOutcome;

use super::super::TurnStateAdapter;
use super::ReactAgent;

impl ReactAgent {
    fn record_tool_observation_message(
        &self,
        _context: &AgentRunContext,
        turn_state: &mut TurnStateAdapter,
        observation: &ToolObservation,
        model_call_id: Option<String>,
        _sequence: u32,
    ) -> Result<crate::domain::agent::AgentMessage, AgentError> {
        Ok(turn_state.record_tool_observation(observation, model_call_id))
    }
}

// Product audit/persistence hooks. Framework owns invocation, scheduling,
// cancellation and durable tool identity; these hooks perform no dispatch.
impl ReactAgent {
    pub(crate) fn framework_tool_started(
        &self,
        context: &AgentRunContext,
        id: &str,
        name: &str,
        _provider_id: &str,
        input: &serde_json::Value,
        _sequence: u32,
        batch: Option<&eliteforge_agent_core::ToolBatchItem>,
    ) -> Result<(), AgentError> {
        self.publish_tool(
            context,
            "agent.tool.selected",
            BusOutcome::Progress,
            Some(name),
            id,
            name,
            Some(input),
            None,
            batch,
        )?;
        self.publish_tool(
            context,
            "agent.tool.started",
            BusOutcome::Started,
            Some(name),
            id,
            name,
            Some(input),
            None,
            batch,
        )?;
        Ok(())
    }
    pub(crate) fn framework_tool_activity(
        &self,
        context: &AgentRunContext,
        id: &str,
        name: &str,
        line: &str,
    ) {
        // Blank lines remain in native progress and the complete tool result.
        if line.is_empty() {
            return;
        }
        let _ = self.publish_tool(
            context,
            "agent.tool.activity",
            BusOutcome::Progress,
            Some("tool_activity"),
            id,
            name,
            None,
            Some(line),
            None,
        );
    }
    pub(crate) fn framework_tool_finished(
        &self,
        context: &mut AgentRunContext,
        turn: &mut TurnStateAdapter,
        id: &str,
        provider_id: &str,
        sequence: u32,
        observation: ToolObservation,
        failed: bool,
        batch: Option<&eliteforge_agent_core::ToolBatchItem>,
    ) -> Result<crate::domain::agent::AgentMessage, AgentError> {
        self.publish_tool(
            context,
            if failed {
                "agent.tool.failed"
            } else {
                "agent.tool.completed"
            },
            if failed {
                BusOutcome::Failure
            } else {
                BusOutcome::Completed
            },
            Some(&observation.tool_name),
            id,
            &observation.tool_name,
            None,
            Some(&observation.output),
            batch,
        )?;
        let message = self.record_tool_observation_message(
            context,
            turn,
            &observation,
            Some(provider_id.into()),
            sequence,
        )?;
        self.publish_tool(
            context,
            "agent.observation.created",
            BusOutcome::Progress,
            Some("observation_created"),
            id,
            &observation.tool_name,
            None,
            Some(&observation.output),
            batch,
        )?;
        context.observations.push(observation);
        Ok(message)
    }
}
