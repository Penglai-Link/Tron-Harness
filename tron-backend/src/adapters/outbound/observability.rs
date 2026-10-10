//! Tron projection of Framework telemetry records. No transport or physical storage.
use crate::adapters::outbound::persistence::sqlite::framework_storage::blocking;
use crate::application::observability::*;
use eliteforge_agent_core::{TelemetrySpan, TelemetryStore, TelemetryTrace, TelemetryTraceFilter};
use std::{collections::BTreeMap, sync::Arc};
pub struct TelemetryProjection {
    pub store: Arc<dyn TelemetryStore>,
    pub instance_id: String,
}
fn failure(_: impl std::fmt::Display) -> ObservabilityError {
    ObservabilityError::Storage
}
fn filter(q: &TraceQuery) -> TelemetryTraceFilter {
    let mut attributes = BTreeMap::from([("tron.workspace.code".into(), q.workspace_code.clone())]);
    if let Some(session) = &q.session_code {
        attributes.insert("tron.session.code".into(), session.clone());
    }
    TelemetryTraceFilter {
        trace_id: q.trace_id.clone(),
        attributes,
    }
}
fn status(attrs: &BTreeMap<String, String>, code: i32) -> String {
    match attrs.get("agent.operation.outcome").map(String::as_str) {
        Some("unknown") => "Unknown",
        Some("cancelled") => "Cancelled",
        Some("waiting" | "awaiting_input") => "Waiting",
        Some("interrupted" | "step_limit" | "tool_call_limit") => "Interrupted",
        Some("error") => "Error",
        _ if code == 2 => "Error",
        _ => "Completed",
    }
    .into()
}
fn trace_status(t: &TelemetryTrace) -> String {
    if t.ended_at.is_none() {
        "Running".into()
    } else if t.abandoned {
        "Unknown".into()
    } else {
        status(&t.attributes, t.status_code)
    }
}
fn span(s: TelemetrySpan) -> TraceSpan {
    TraceSpan {
        span_id: s.span_id,
        parent_span_id: s.parent_span_id,
        name: s.name,
        started_at: s.started_at,
        duration_ms: s
            .data
            .get("durationMs")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or((s.ended_at - s.started_at).max(0) as f64),
        status: status(&s.attributes, s.status_code),
        attributes: s.attributes,
        events: s
            .events
            .into_iter()
            .map(|e| SpanEvent {
                name: e.name,
                timestamp: e.timestamp,
                attributes: e.attributes,
            })
            .collect(),
        links: s
            .links
            .into_iter()
            .map(|l| SpanLink {
                trace_id: l.trace_id,
                span_id: l.span_id,
            })
            .collect(),
    }
}
fn summary(t: TelemetryTrace, spans: &[TraceSpan]) -> TraceSummary {
    let attr = |key: &str| t.attributes.get(key).cloned().unwrap_or_default();
    let mut record = TraceSummary {
        trace_id: t.trace_id.clone(),
        session_code: attr("tron.session.code"),
        run_code: attr("tron.run.code"),
        agent_code: attr("tron.agent.code"),
        started_at: t.started_at,
        ended_at: t.ended_at,
        duration_ms: (t.ended_at.unwrap_or_else(now) - t.started_at).max(0) as f64,
        status: trace_status(&t),
        span_count: 0,
        error_count: 0,
        model_count: 0,
        tool_count: 0,
        input_tokens: 0,
        output_tokens: 0,
    };
    for s in spans {
        if s.name == "tron.session.run" {
            continue;
        }
        record.span_count += 1;
        record.error_count += usize::from(s.status == "Error");
        if s.name == "agent.model" {
            record.model_count += 1;
            record.input_tokens += s
                .attributes
                .get("gen_ai.usage.input_tokens")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            record.output_tokens += s
                .attributes
                .get("gen_ai.usage.output_tokens")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
        }
        record.tool_count += usize::from(s.name == "agent.tool");
    }
    record
}
pub(crate) fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}
impl TraceRepository for TelemetryProjection {
    fn query(&self, q: &TraceQuery) -> Result<TracePage, ObservabilityError> {
        blocking(async {
            let page = self
                .store
                .query_traces(filter(q), (q.current - 1) * q.size, q.size)
                .await
                .map_err(failure)?;
            let mut records = Vec::new();
            for t in page.records {
                let spans = self
                    .store
                    .spans(t.trace_id.clone())
                    .await
                    .map_err(failure)?
                    .into_iter()
                    .map(span)
                    .collect::<Vec<_>>();
                records.push(summary(t, &spans));
            }
            Ok(TracePage {
                current: q.current,
                size: q.size,
                total: page.total,
                records,
            })
        })
    }
    fn detail(&self, workspace: &str, id: &str) -> Result<TraceDetail, ObservabilityError> {
        blocking(async {
            let filter = TelemetryTraceFilter {
                trace_id: Some(id.into()),
                attributes: BTreeMap::from([("tron.workspace.code".into(), workspace.into())]),
            };
            let mut page = self
                .store
                .query_traces(filter, 0, 1)
                .await
                .map_err(failure)?;
            let t = page.records.pop().ok_or(ObservabilityError::NotFound)?;
            let spans = self
                .store
                .spans(id.into())
                .await
                .map_err(failure)?
                .into_iter()
                .map(span)
                .collect::<Vec<_>>();
            Ok(TraceDetail {
                trace: summary(t, &spans),
                spans,
            })
        })
    }
    fn metrics(&self, q: &TraceQuery) -> Result<TraceMetrics, ObservabilityError> {
        blocking(async {
            let f = filter(q);
            let mut durations = Vec::new();
            let mut failed = 0;
            let mut offset = 0;
            let total = loop {
                let page = self
                    .store
                    .query_traces(f.clone(), offset, 5000)
                    .await
                    .map_err(failure)?;
                let total = page.total;
                let count = page.records.len();
                for t in page.records {
                    failed += usize::from(trace_status(&t) == "Error");
                    if let Some(end) = t.ended_at {
                        durations.push((end - t.started_at).max(0) as f64);
                    }
                }
                offset += count;
                if offset >= total || count == 0 {
                    break total;
                }
            };
            let sums = self
                .store
                .sum_span_attributes(
                    f,
                    "agent.model".into(),
                    vec![
                        "gen_ai.usage.input_tokens".into(),
                        "gen_ai.usage.output_tokens".into(),
                    ],
                )
                .await
                .map_err(failure)?;
            durations.sort_by(f64::total_cmp);
            let average = if durations.is_empty() {
                0.
            } else {
                durations.iter().sum::<f64>() / durations.len() as f64
            };
            let p95 = durations
                .get((durations.len() * 95).div_ceil(100).saturating_sub(1))
                .copied()
                .unwrap_or(0.);
            Ok(TraceMetrics {
                trace_count: total,
                failed_trace_count: failed,
                input_tokens: *sums.get("gen_ai.usage.input_tokens").unwrap_or(&0),
                output_tokens: *sums.get("gen_ai.usage.output_tokens").unwrap_or(&0),
                average_duration_ms: average,
                p95_duration_ms: p95,
            })
        })
    }
}
impl ObservabilityRepository for TelemetryProjection {
    fn metric_points(
        &self,
        name: Option<String>,
        current: usize,
        size: usize,
    ) -> Result<MetricPage, ObservabilityError> {
        if current == 0 || current > 1_000_000 || !(1..=100).contains(&size) {
            return Err(ObservabilityError::InvalidQuery);
        }
        blocking(async {
            let page = self
                .store
                .metric_points(name, (current - 1) * size, size)
                .await
                .map_err(failure)?;
            let records = page
                .records
                .into_iter()
                .map(|p| {
                    serde_json::from_value(serde_json::to_value(p).map_err(failure)?)
                        .map_err(failure)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(MetricPage {
                current,
                size,
                total: page.total,
                records,
            })
        })
    }
    fn runtime_metrics(&self) -> Result<RuntimeMetrics, ObservabilityError> {
        blocking(async {
            let resource = BTreeMap::from([
                ("service.instance.id".into(), self.instance_id.clone()),
                ("process.pid".into(), std::process::id().to_string()),
            ]);
            let cpu = self
                .store
                .latest_metric("process.cpu.utilization".into(), resource.clone())
                .await
                .map_err(failure)?;
            let memory = self
                .store
                .latest_metric("process.memory.usage".into(), resource)
                .await
                .map_err(failure)?;
            let value = |p: &eliteforge_agent_core::TelemetryMetricPoint| {
                p.data.get("asDouble").and_then(|v| v.as_f64()).or_else(|| {
                    p.data
                        .get("asInt")
                        .and_then(|v| v.as_str()?.parse::<f64>().ok())
                })
            };
            Ok(RuntimeMetrics {
                cpu_percent: cpu.as_ref().and_then(value).map(|v| v * 100.),
                memory_bytes: memory.as_ref().and_then(value).map(|v| v.max(0.) as u64),
                sampled_at: cpu
                    .as_ref()
                    .map(|p| p.timestamp)
                    .or(memory.as_ref().map(|p| p.timestamp)),
                metric_point_count: self
                    .store
                    .metric_points(None, 0, 1)
                    .await
                    .map_err(failure)?
                    .total,
            })
        })
    }
}
