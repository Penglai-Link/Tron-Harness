//! Tron projection of Framework telemetry records. No transport or physical storage.
mod dashboard;
mod triggers;
mod query_filters;
pub mod logs;
use crate::adapters::outbound::persistence::sqlite::framework_storage::blocking;
use crate::application::observability::*;
use eliteforge_agent_core::{TelemetrySpan, TelemetryStore, TelemetryTrace, TelemetryTraceFilter};
use std::{collections::BTreeMap, sync::Arc};
pub struct TelemetryProjection {
    pub database: eliteforge_agent_official_plugin_storage_local::SqliteDatabase,
    pub store: Arc<dyn TelemetryStore>,
    pub instance_id: String,
}
fn failure(_: impl std::fmt::Display) -> ObservabilityError {
    ObservabilityError::Storage
}
fn filter(q: &TraceQuery) -> TelemetryTraceFilter {
    let mut attributes = BTreeMap::new();
    if let Some(category) = q.category {
        let key = match category { TraceCategory::Page => "tron.trace.page", TraceCategory::Agent => "tron.trace.agent" };
        attributes.insert(key.into(), "true".into());
    }
    if !q.workspace_code.is_empty() { attributes.insert("tron.workspace.code".into(), q.workspace_code.clone()); }
    if let Some(session) = &q.session_code {
        attributes.insert("tron.session.code".into(), session.clone());
    }
    TelemetryTraceFilter {
        trace_id: q.trace_id.clone(),
        attributes,
    }
}
fn status(attrs: &BTreeMap<String, String>, code: i32) -> String {
    if attrs.get("tron.operation.active").is_some_and(|v| v == "true") { return "Running".into(); }
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
    if t.ended_at.is_none() || t.attributes.get("tron.operation.active").is_some_and(|v| v == "true") {
        "Running".into()
    } else if t.abandoned {
        "Unknown".into()
    } else {
        status(&t.attributes, t.status_code)
    }
}
fn span(mut s: TelemetrySpan) -> TraceSpan {
    if s.name == "db.sqlite.business" {
        if let Some(file) = s.attributes.get("code.file.path") {
            let module = file.split("/sqlite/").last().unwrap_or(file).trim_end_matches(".rs").trim_end_matches(".inc");
            s.name = format!("db.sqlite.{module}:{}", s.attributes.get("code.line.number").map(String::as_str).unwrap_or("?"));
        }
    }
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
    let root = spans.iter().find(|s| s.name.starts_with("page "))
        .or_else(|| spans.iter().find(|s| s.parent_span_id.is_none()));
    let mut record = TraceSummary {
        trigger: root.map(|s| s.attributes.get("url.path").cloned().unwrap_or_else(|| s.name.clone())).unwrap_or_default(),
        root_span_id: root.map(|s| s.span_id.clone()),
        workspace_code: attr("tron.workspace.code"),
        trace_id: t.trace_id.clone(),
        session_code: attr("tron.session.code"),
        session_name: None,
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
        cached_input_tokens: 0,
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
            record.cached_input_tokens += s
                .attributes
                .get("gen_ai.usage.cached_input_tokens")
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
fn sort_trace_summaries(records: &mut [TraceSummary], sorts: &[crate::application::observability::TraceSort]) {
    records.sort_by(|a, b| {
        for sort in sorts {
            let order = match sort.field {
                TraceSortField::StartedAt => a.started_at.cmp(&b.started_at),
                TraceSortField::Duration => a.duration_ms.total_cmp(&b.duration_ms),
                TraceSortField::Spans => a.span_count.cmp(&b.span_count),
                TraceSortField::Tokens => a.input_tokens.saturating_add(a.output_tokens).cmp(&b.input_tokens.saturating_add(b.output_tokens)),
                TraceSortField::Errors => a.error_count.cmp(&b.error_count),
            };
            if !order.is_eq() {
                return if matches!(sort.direction, TraceSortDirection::Desc) { order.reverse() } else { order };
            }
        }
        b.started_at.cmp(&a.started_at).then_with(|| a.trace_id.cmp(&b.trace_id))
    });
}
impl TraceRepository for TelemetryProjection {
    fn agent_options(&self, workspace: &str) -> Result<Vec<TraceAgentOption>, ObservabilityError> { self.trace_agent_options(workspace) }
    fn query(&self, q: &TraceQuery) -> Result<TracePage, ObservabilityError> {
        if matches!(q.category, Some(TraceCategory::Agent)) { return self.agent_triggers(q); }
        if matches!(q.category, Some(TraceCategory::Page)) { return self.page_triggers(q); }
        blocking(async {
            let sorts = if q.sorts.is_empty() { vec![crate::application::observability::TraceSort { field: q.sort_by, direction: q.sort_direction }] } else { q.sorts.clone() };
            let default_order = sorts.len() == 1 && sorts[0].field == TraceSortField::StartedAt && sorts[0].direction == TraceSortDirection::Desc;
            let mut offset = if default_order { (q.current - 1) * q.size } else { 0 };
            let mut records = Vec::new();
            let total = loop {
                let page = self.store.query_traces(filter(q), offset, if default_order { q.size } else { 5000 })
                    .await.map_err(failure)?;
                let total = page.total;
                let count = page.records.len();
                for t in page.records {
                    let spans = self.store.spans(t.trace_id.clone()).await.map_err(failure)?
                        .into_iter().map(span).collect::<Vec<_>>();
                    records.push(summary(t, &spans));
                }
                offset += count;
                if default_order || offset >= total || count == 0 { break total; }
            };
            if !default_order {
                sort_trace_summaries(&mut records, &sorts);
                records = records.into_iter().skip((q.current - 1) * q.size).take(q.size).collect();
            }
            Ok(TracePage {
                agent_options: Vec::new(),
                current: q.current,
                size: q.size,
                total,
                records,
            })
        })
    }
    fn detail(&self, workspace: &str, id: &str) -> Result<TraceDetail, ObservabilityError> {
        // Keep inspection reads off the telemetry export connection. Decode large
        // payloads after releasing the database worker so other queries can proceed.
        let workspace = workspace.to_owned();
        let id = id.to_owned();
        let (trace, payloads) = blocking(self.database.call(move |c| -> rusqlite::Result<_> {
            use rusqlite::OptionalExtension;
            let trace = c.query_row(
                "SELECT trace_id,started_at,ended_at,status_code,attributes,abandoned FROM ef_telemetry_traces WHERE trace_id=?1 AND (?2='' OR json_extract(attributes,'$.\"tron.workspace.code\"')=?2)",
                rusqlite::params![id, workspace],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, Option<i64>>(2)?, r.get::<_, i32>(3)?, r.get::<_, String>(4)?, r.get::<_, bool>(5)?)),
            ).optional()?;
            let payloads = if trace.is_some() {
                let mut statement = c.prepare("SELECT data FROM ef_telemetry_spans WHERE trace_id=?1")?;
                statement.query_map([id], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?
            } else { Vec::new() };
            Ok((trace, payloads))
        })).map_err(failure)?;
        let (trace_id, started_at, ended_at, status_code, attributes, abandoned) = trace.ok_or(ObservabilityError::NotFound)?;
        let trace = TelemetryTrace { trace_id, started_at, ended_at, status_code, abandoned,
            attributes: serde_json::from_str(&attributes).map_err(failure)?,
        };
        let mut spans = payloads.into_iter().map(|data| serde_json::from_str::<TelemetrySpan>(&data).map(span).map_err(failure)).collect::<Result<Vec<_>, _>>()?;
        spans.sort_by(|a, b| a.started_at.cmp(&b.started_at).then_with(|| a.span_id.cmp(&b.span_id)));
        Ok(TraceDetail { trace: summary(trace, &spans), spans })
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
    fn scoped_metric_points(&self, workspace: Option<String>, name: Option<String>, current: usize, size: usize) -> Result<MetricPage, ObservabilityError> {
        let Some(workspace) = workspace.filter(|code| !code.is_empty()) else { return self.metric_points(name, current, size); };
        if current == 0 || current > 1_000_000 || !(1..=100).contains(&size) { return Err(ObservabilityError::InvalidQuery); }
        let start = (current - 1) * size;
        let mut total = 0;
        let mut records = Vec::new();
        let mut page_number = 1;
        const SCAN_PAGE_SIZE: usize = 50_000;
        loop {
            let page = blocking(async {
                let page = self
                    .store
                    .metric_points(
                        name.clone(),
                        (page_number - 1) * SCAN_PAGE_SIZE,
                        SCAN_PAGE_SIZE,
                    )
                    .await
                    .map_err(failure)?;
                let records = page
                    .records
                    .into_iter()
                    .map(|point| {
                        serde_json::from_value(serde_json::to_value(point).map_err(failure)?)
                            .map_err(failure)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok::<_, ObservabilityError>(MetricPage {
                    current: page_number,
                    size: SCAN_PAGE_SIZE,
                    total: page.total,
                    records,
                })
            })?;
            let count = page.records.len();
            for point in page.records {
                let belongs = point.resource.get("tron.workspace.code") == Some(&workspace)
                    || point.data.get("attributes").and_then(|v| v.as_array()).is_some_and(|attrs| attrs.iter().any(|attr| attr.get("key").and_then(|v| v.as_str()) == Some("tron.workspace.code") && attr.pointer("/value/stringValue").and_then(|v| v.as_str()) == Some(workspace.as_str())));
                if belongs { if total >= start && records.len() < size { records.push(point); } total += 1; }
            }
            if page_number * SCAN_PAGE_SIZE >= page.total || count == 0 { break; }
            page_number += 1;
        }
        Ok(MetricPage { current, size, total, records })
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

#[cfg(test)]
mod sorting_tests {
    use super::*;
    fn row(id: &str, start: i64, duration: f64, spans: usize, tokens: u64, errors: usize) -> TraceSummary {
        TraceSummary { trigger: String::new(), root_span_id: None, workspace_code: "w".into(), trace_id: id.into(), session_code: "s".into(), session_name: None,
            run_code: "r".into(), agent_code: "a".into(), started_at: start, ended_at: Some(start + duration as i64),
            duration_ms: duration, status: "Completed".into(), span_count: spans, error_count: errors,
            model_count: 0, tool_count: 0, input_tokens: tokens, cached_input_tokens: 0, output_tokens: 1 }
    }
    #[test]
    fn trace_sorting_uses_numeric_values_and_deterministic_ties() {
        let rows = vec![row("a", 10, 1200., 2, 1000, 1), row("b", 20, 50., 10, 20, 3), row("c", 30, 300., 1, 200, 2)];
        for (field, expected) in [
            (TraceSortField::StartedAt, ["a", "b", "c"]),
            (TraceSortField::Duration, ["b", "c", "a"]),
            (TraceSortField::Spans, ["c", "a", "b"]),
            (TraceSortField::Tokens, ["b", "c", "a"]),
            (TraceSortField::Errors, ["a", "c", "b"]),
        ] {
            let mut sorted = rows.clone();
            sort_trace_summaries(&mut sorted, &[crate::application::observability::TraceSort { field: field, direction: TraceSortDirection::Asc }]);
            assert_eq!(sorted.iter().map(|r| r.trace_id.as_str()).collect::<Vec<_>>(), expected);
            sort_trace_summaries(&mut sorted, &[crate::application::observability::TraceSort { field: field, direction: TraceSortDirection::Desc }]);
            assert_eq!(sorted.iter().map(|r| r.trace_id.as_str()).collect::<Vec<_>>(), expected.into_iter().rev().collect::<Vec<_>>());
        }
        let mut tied = vec![row("b", 1, 1., 1, 1, 1), row("a", 1, 1., 1, 1, 1)];
        sort_trace_summaries(&mut tied, &[crate::application::observability::TraceSort { field: TraceSortField::Tokens, direction: TraceSortDirection::Desc }]);
        assert_eq!(tied[0].trace_id, "a");
        let mut combined = vec![row("a", 30, 100., 2, 1, 1), row("b", 10, 100., 5, 1, 1), row("c", 20, 50., 9, 1, 1)];
        sort_trace_summaries(&mut combined, &[
            crate::application::observability::TraceSort { field: TraceSortField::Duration, direction: TraceSortDirection::Desc },
            crate::application::observability::TraceSort { field: TraceSortField::Spans, direction: TraceSortDirection::Desc },
        ]);
        assert_eq!(combined.iter().map(|r| r.trace_id.as_str()).collect::<Vec<_>>(), ["b", "a", "c"]);

    }
}
