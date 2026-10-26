//! Tron supplies status semantics through generic Framework predicates.
use super::*;
use eliteforge_agent_core::{
    TelemetryBucketQuery, TelemetryTracePredicate as P, TelemetryTraceStatistics,
};
fn attr(key: &str, value: &str) -> P {
    P::Attribute {
        key: key.into(),
        value: value.into(),
    }
}
fn classifications() -> BTreeMap<String, P> {
    let active = attr("tron.operation.active", "true");
    let outcomes = [
        "unknown",
        "cancelled",
        "waiting",
        "awaiting_input",
        "interrupted",
        "step_limit",
        "tool_call_limit",
    ];
    BTreeMap::from([
        (
            "running".into(),
            P::Any(vec![P::HasEnded(false), active.clone()]),
        ),
        (
            "error".into(),
            P::All(vec![
                P::HasEnded(true),
                P::Not(Box::new(active)),
                P::Not(Box::new(P::Abandoned(true))),
                P::Any(vec![
                    attr("agent.operation.outcome", "error"),
                    P::All(vec![
                        P::StatusCode(2),
                        P::Not(Box::new(P::Any(
                            outcomes
                                .into_iter()
                                .map(|o| attr("agent.operation.outcome", o))
                                .collect(),
                        ))),
                    ]),
                ]),
            ]),
        ),
    ])
}
impl TelemetryProjection {
    pub(super) fn statistics(
        &self,
        q: &TraceQuery,
        at: Option<i64>,
    ) -> Result<(TelemetryTraceStatistics, BTreeMap<String, u64>), ObservabilityError> {
        blocking(async {
            let buckets = at.map(|end| TelemetryBucketQuery {
                start: end / 60_000 * 60_000 - 14 * 60_000,
                end,
                width_ms: 60_000,
            });
            let stats = self
                .store
                .aggregate_traces(filter(q), classifications(), buckets)
                .await
                .map_err(failure)?;
            let sums = self
                .store
                .sum_span_attributes(
                    filter(q),
                    "agent.model".into(),
                    vec![
                        "gen_ai.usage.input_tokens".into(),
                        "gen_ai.usage.output_tokens".into(),
                    ],
                )
                .await
                .map_err(failure)?;
            Ok((stats, sums))
        })
    }
}
