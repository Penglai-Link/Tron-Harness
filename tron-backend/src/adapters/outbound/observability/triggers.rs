//! Agent entries are execution spans, not the last agent attached to a trace.
use super::*;
use rusqlite::{params_from_iter, types::Value};

impl TelemetryProjection {
    pub(super) fn agent_triggers(&self, q: &TraceQuery) -> Result<TracePage, ObservabilityError> {
        let mut clause =
            "json_extract(s.data,'$.name') IN ('tron.session.run','tron.assistant.execute')"
                .to_owned();
        let mut values = Vec::<Value>::new();
        clause.push_str(" AND NOT (json_extract(s.data,'$.name')='tron.session.run' AND EXISTS (SELECT 1 FROM ef_telemetry_spans p WHERE p.trace_id=s.trace_id AND p.span_id=json_extract(s.data,'$.parentSpanId') AND json_extract(p.data,'$.name')='tron.assistant.execute'))");
        for (key, value) in [
            (
                "tron.workspace.code",
                Some(q.workspace_code.clone()).filter(|s| !s.is_empty()),
            ),
            ("tron.session.code", q.session_code.clone()),
        ] {
            if let Some(value) = value {
                clause.push_str(&format!(
                    " AND json_extract(s.data,'$.attributes.\"{key}\"')=?"
                ));
                values.push(Value::Text(value));
            }
        }
        if let Some(id) = &q.trace_id {
            clause.push_str(" AND s.trace_id=?");
            values.push(Value::Text(id.clone()));
        }
        query_filters::append_span_filter(
            &mut clause,
            &mut values,
            q.filters.span_id.as_deref(),
            "s.trace_id",
            Some("s.span_id"),
        );
        query_filters::append_filters(
            &mut clause,
            &mut values,
            &q.filters,
            "json_extract(s.data,'$.attributes')",
            "json_extract(s.data,'$.statusCode')",
            "0",
            "0",
            "s.trace_id",
            "s.data",
        );
        let sorts = list_query::sorts(q);
        let metric_sort = list_query::needs_metrics(&sorts);
        let order = list_query::order(&sorts);
        let offset = (q.current - 1) * q.size;
        let limit = format!("LIMIT {} OFFSET {offset}", q.size);
        let timestamp = now();
        let raw_roots = format!("SELECT s.trace_id,s.span_id AS root_span_id,s.data,
            json_extract(s.data,'$.startedAt') AS started_at,
            MAX(0,(CASE WHEN json_extract(s.data,'$.attributes.\"tron.operation.active\"')='true' THEN {timestamp} ELSE s.ended_at END)-json_extract(s.data,'$.startedAt')) AS duration_ms
            FROM ef_telemetry_spans s WHERE {clause}");
        // Stable counters are updated during span ingestion; lists never traverse descendants.
        let roots = if metric_sort {
            raw_roots
        } else {
            format!("SELECT * FROM ({raw_roots}) r ORDER BY {order} {limit}")
        };
        let columns = list_query::METRIC_COLUMNS;
        let sql = format!("WITH roots AS MATERIALIZED ({roots})
            SELECT r.data,{columns} FROM roots r
            LEFT JOIN ef_telemetry_span_rollups m ON m.trace_id=r.trace_id AND m.span_id=r.root_span_id
            ORDER BY {order} {}", if metric_sort { limit } else { String::new() });
        let (total, roots) = blocking(self.database.call_read(move |c| -> rusqlite::Result<_> {
            let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(c,rusqlite::TransactionBehavior::Deferred)?;
            let total: usize = transaction.query_row(&format!("SELECT COUNT(*) FROM ef_telemetry_spans s WHERE {clause}"),params_from_iter(&values),|r|r.get(0))?;
            let records = if total==0 { Vec::new() } else {
                let mut statement = transaction.prepare(&sql)?;
                statement.query_map(params_from_iter(&values),|row| Ok((row.get::<_,String>(0)?,list_query::Metrics::read(row,1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?
            };
            transaction.commit()?;
            Ok((total,records))
        })).map_err(failure)?;
        let records = roots
            .into_iter()
            .map(|(payload, metrics)| {
                // Decode only the small execution root, outside the database worker.
                let root: TelemetrySpan = serde_json::from_str(&payload).map_err(failure)?;
                let running = root
                    .attributes
                    .get("tron.operation.active")
                    .is_some_and(|v| v == "true");
                let mut record = summary(
                    TelemetryTrace {
                        trace_id: root.trace_id,
                        started_at: root.started_at,
                        ended_at: if running { None } else { Some(root.ended_at) },
                        status_code: root.status_code,
                        attributes: root.attributes.clone(),
                        abandoned: false,
                    },
                    &[],
                );
                record.root_span_id = Some(root.span_id);
                record.trigger = if root.name == "tron.assistant.execute" {
                    root.attributes
                        .get("tron.assistant.code")
                        .cloned()
                        .unwrap_or_else(|| record.agent_code.clone())
                } else {
                    record.agent_code.clone()
                };
                metrics.apply(&mut record);
                Ok(record)
            })
            .collect::<Result<Vec<_>, ObservabilityError>>()?;
        Ok(TracePage {
            agent_options: Vec::new(),
            current: q.current,
            size: q.size,
            total,
            records,
        })
    }
}
