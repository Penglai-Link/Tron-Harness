use crate::domain::agent::{
    AgentError, AgentRunContext, ExpandedDomainPersistenceMode, ToolContextDelta, ToolExposureMode,
    ToolObservation,
};
use crate::shared::event_bus::BusOutcome;

use super::super::{DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL, TurnStateAdapter};
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
        );
    }
    pub(crate) fn framework_apply_delta(
        &self,
        context: &mut AgentRunContext,
        turn: &mut TurnStateAdapter,
        delta: &ToolContextDelta,
    ) -> Result<(), String> {
        self.apply_tool_context_delta(context, delta, turn)
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
        )?;
        let message = self.record_tool_observation_message(
            context,
            turn,
            &observation,
            Some(provider_id.into()),
            sequence,
        )?;
        collapse_ephemeral_expanded_domains(context, &observation.tool_name);
        self.publish_tool(
            context,
            "agent.observation.created",
            BusOutcome::Progress,
            Some("observation_created"),
            id,
            &observation.tool_name,
            None,
            Some(&observation.output),
        )?;
        context.observations.push(observation);
        Ok(message)
    }
}
