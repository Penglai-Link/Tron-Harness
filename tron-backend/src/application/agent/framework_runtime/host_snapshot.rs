//! Native execution observation crosses the application boundary as product data.
use crate::domain::agent::{AgentChargedUsage, AgentError, AgentNativeExecutionSnapshot};
use eliteforge_agent_core as fw;
pub(super) fn product_snapshot(
    snapshot: fw::RunExecutionSnapshot,
) -> Result<AgentNativeExecutionSnapshot, AgentError> {
    Ok(AgentNativeExecutionSnapshot {
        limits: serde_json::to_value(snapshot.limits)
            .map_err(|e| AgentError::InvalidConfiguration(e.to_string()))?,
        budget_usage: AgentChargedUsage {
            input_tokens: snapshot.budget_usage.input_tokens,
            output_tokens: snapshot.budget_usage.output_tokens,
        },
        model_calls: snapshot.model_calls,
        child_runs: snapshot.child_runs,
        steps: snapshot.steps,
        tool_calls: snapshot.tool_calls,
        deadline_unix_ms: snapshot.deadline_unix_ms,
        opaque: snapshot.opaque,
        incomplete: snapshot.incomplete,
        historical_calls_bound: snapshot.historical_calls_bound,
    })
}
impl TryFrom<AgentNativeExecutionSnapshot> for fw::RunExecutionSnapshot {
    type Error = AgentError;
    fn try_from(snapshot: AgentNativeExecutionSnapshot) -> Result<Self, Self::Error> {
        Ok(Self {
            limits: serde_json::from_value(snapshot.limits).map_err(|e| {
                AgentError::InvalidConfiguration(format!(
                    "native execution limits are invalid: {e}"
                ))
            })?,
            budget_usage: fw::Usage {
                input_tokens: snapshot.budget_usage.input_tokens,
                output_tokens: snapshot.budget_usage.output_tokens,
            },
            model_calls: snapshot.model_calls,
            child_runs: snapshot.child_runs,
            steps: snapshot.steps,
            tool_calls: snapshot.tool_calls,
            deadline_unix_ms: snapshot.deadline_unix_ms,
            opaque: snapshot.opaque,
            incomplete: snapshot.incomplete,
            historical_calls_bound: snapshot.historical_calls_bound,
        })
    }
}
