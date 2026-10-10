//! One-time import of Tron's previous telemetry schema. Normal reads/writes use Framework ports.
use crate::{
    adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
    application::observability::{ObservabilityError, TraceSpan},
};
use eliteforge_agent_core::{
    TelemetryMetricPoint, TelemetrySpan, TelemetrySpanEvent, TelemetrySpanLink, TelemetryStore,
    TelemetryTrace,
};
use std::{collections::BTreeMap, path::Path, sync::Arc};
fn failure(_: impl std::fmt::Display) -> ObservabilityError {
    ObservabilityError::Storage
}
pub(super) async fn import(
    path: &Path,
    store: Arc<dyn TelemetryStore>,
) -> Result<(), ObservabilityError> {
    let db = SharedDatabase::open(&path.to_string_lossy()).map_err(failure)?;
    let (traces, metrics) = db
        .call(
            |c| -> rusqlite::Result<_> {
                let has = |name: &str| {
                    c.query_row(
                        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                        [name],
                        |r| r.get::<_, bool>(0),
                    )
                };
                Ok((has("traces")?, has("metric_points")?))
            },
            || rusqlite::Error::InvalidQuery,
        )
        .map_err(failure)?;
    if !traces && !metrics {
        return Ok(());
    }
    if traces {
        let mut offset = 0;
        loop {
            let batch = db.call(
                move |c| -> rusqlite::Result<Vec<TelemetryTrace>> {
                    let mut stmt = c.prepare(
                        "SELECT trace_id,workspace_code,session_code,run_code,agent_code,started_at,ended_at,status FROM traces ORDER BY trace_id LIMIT 1000 OFFSET ?1",
                    )?;
                    stmt.query_map([offset], |r| {
                        let status: String = r.get(7)?;
                        let start: i64 = r.get(5)?;
                        let end: Option<i64> = r.get(6)?;
                        let outcome = match status.as_str() {
                            "Error" => "error",
                            "Cancelled" => "cancelled",
                            "Waiting" => "waiting",
                            "Interrupted" => "interrupted",
                            "Completed" => "completed",
                            _ => "unknown",
                        };
                        Ok(TelemetryTrace {
                            trace_id: r.get(0)?,
                            started_at: start,
                            ended_at: Some(end.unwrap_or(start)),
                            status_code: if status == "Error" { 2 } else { 0 },
                            attributes: BTreeMap::from([
                                ("tron.workspace.code".into(), r.get::<_, String>(1)?),
                                ("tron.session.code".into(), r.get(2)?),
                                ("tron.run.code".into(), r.get(3)?),
                                ("tron.agent.code".into(), r.get(4)?),
                                ("agent.operation.outcome".into(), outcome.into()),
                            ]),
                            abandoned: status == "Running",
                        })
                    })?.collect()
                },
                || rusqlite::Error::InvalidQuery,
            ).map_err(failure)?;
            let count = batch.len();
            for trace in batch {
                store.register_trace(trace.clone()).await.map_err(failure)?;
                let id = trace.trace_id.clone();
                let spans = db
                    .call(
                        move |c| -> rusqlite::Result<Vec<TraceSpan>> {
                            let mut stmt = c.prepare("SELECT data FROM spans WHERE trace_id=?1")?;
                            stmt.query_map([id], |r| {
                                let json: String = r.get(0)?;
                                serde_json::from_str(&json)
                                    .map_err(|_| rusqlite::Error::InvalidQuery)
                            })?
                            .collect()
                        },
                        || rusqlite::Error::InvalidQuery,
                    )
                    .map_err(failure)?;
                let spans = spans
                    .into_iter()
                    .map(|s| {
                        let mut attrs = trace.attributes.clone();
                        attrs.extend(s.attributes);
                        TelemetrySpan {
                            trace_id: trace.trace_id.clone(),
                            span_id: s.span_id,
                            parent_span_id: s.parent_span_id,
                            name: s.name,
                            started_at: s.started_at,
                            ended_at: s.started_at.saturating_add(s.duration_ms.max(0.) as i64),
                            status_code: if s.status == "Error" { 2 } else { 0 },
                            attributes: attrs,
                            events: s
                                .events
                                .into_iter()
                                .map(|e| TelemetrySpanEvent {
                                    name: e.name,
                                    timestamp: e.timestamp,
                                    attributes: e.attributes,
                                })
                                .collect(),
                            links: s
                                .links
                                .into_iter()
                                .map(|l| TelemetrySpanLink {
                                    trace_id: l.trace_id,
                                    span_id: l.span_id,
                                })
                                .collect(),
                            data: serde_json::json!({"legacy":true,"durationMs":s.duration_ms}),
                        }
                    })
                    .collect();
                store.write_spans(spans).await.map_err(failure)?;
            }
            offset += count;
            if count < 1000 {
                break;
            }
        }
    }
    if metrics {
        let mut offset = 0;
        loop {
            let points = db
                .call(
                    move |c| -> rusqlite::Result<Vec<TelemetryMetricPoint>> {
                        let mut stmt = c.prepare(
                            "SELECT data FROM metric_points ORDER BY point_id LIMIT 1000 OFFSET ?1",
                        )?;
                        stmt.query_map([offset], |r| {
                            let json: String = r.get(0)?;
                            serde_json::from_str(&json).map_err(|_| rusqlite::Error::InvalidQuery)
                        })?
                        .collect()
                    },
                    || rusqlite::Error::InvalidQuery,
                )
                .map_err(failure)?;
            let count = points.len();
            store.write_metrics(points).await.map_err(failure)?;
            offset += count;
            if count < 1000 {
                break;
            }
        }
    }
    // Remove old copies only after every imported batch committed successfully.
    db.call(
        move |c| -> rusqlite::Result<()> {
            let tx = c.transaction()?;
            if traces {
                tx.execute_batch("DROP TABLE spans;DROP TABLE traces;")?;
            }
            if metrics {
                tx.execute_batch("DROP TABLE metric_points;")?;
            }
            tx.commit()
        },
        || rusqlite::Error::InvalidQuery,
    )
    .map_err(failure)?;
    store.cleanup().await.map_err(failure)
}
