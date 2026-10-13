//! Agent entries are execution spans, not the last agent attached to a trace.
use super::*;
use rusqlite::{params_from_iter, types::Value};

impl TelemetryProjection {
    pub(super) fn agent_triggers(&self, q: &TraceQuery) -> Result<TracePage, ObservabilityError> {
        let mut clause = "json_extract(s.data,'$.name') IN ('tron.session.run','tron.dude.execute')".to_owned();
        let mut values = Vec::<Value>::new();
        // A Dude's immediate run belongs to its execution entry; spawned agents retain their own entries.
        clause.push_str(" AND NOT (json_extract(s.data,'$.name')='tron.session.run' AND EXISTS (SELECT 1 FROM ef_telemetry_spans p WHERE p.trace_id=s.trace_id AND p.span_id=json_extract(s.data,'$.parentSpanId') AND json_extract(p.data,'$.name')='tron.dude.execute'))");
        for (key, value) in [("tron.workspace.code", Some(q.workspace_code.clone()).filter(|s| !s.is_empty())), ("tron.session.code", q.session_code.clone())] {
            if let Some(value) = value {
                clause.push_str(" AND json_extract(s.data,?)=?");
                values.push(Value::Text(format!("$.attributes.\"{key}\"")));
                values.push(Value::Text(value));
            }
        }
        if let Some(id) = &q.trace_id { clause.push_str(" AND s.trace_id=?"); values.push(Value::Text(id.clone())); }
        query_filters::append_span_filter(&mut clause, &mut values, q.filters.span_id.as_deref(), "s.trace_id", Some("s.span_id"));
        query_filters::append_filters(&mut clause, &mut values, &q.filters, "json_extract(s.data,'$.attributes')", "json_extract(s.data,'$.statusCode')", "0", "0", "s.trace_id", "s.data");
        let sorts = if q.sorts.is_empty() { vec![TraceSort { field:q.sort_by, direction:q.sort_direction }] } else { q.sorts.clone() };
        let default_order = sorts.len()==1 && sorts[0].field==TraceSortField::StartedAt && sorts[0].direction==TraceSortDirection::Desc;
        let offset = (q.current-1)*q.size;
        let size = q.size;
        let (total, roots) = blocking(self.database.call(move |c| -> rusqlite::Result<_> {
            let total: usize = c.query_row(&format!("SELECT COUNT(*) FROM ef_telemetry_spans s WHERE {clause}"), params_from_iter(&values), |r| r.get(0))?;
            let mut sql = format!("SELECT s.data FROM ef_telemetry_spans s WHERE {clause} ORDER BY json_extract(s.data,'$.startedAt') DESC,s.span_id DESC");
            if default_order { sql.push_str(" LIMIT ? OFFSET ?"); values.push(Value::Integer(size as i64)); values.push(Value::Integer(offset as i64)); }
            let mut stmt = c.prepare(&sql)?;
            let roots = stmt.query_map(params_from_iter(values), |r| {
                let data: String = r.get(0)?;
                serde_json::from_str::<TelemetrySpan>(&data).map_err(|_| rusqlite::Error::InvalidQuery)
            })?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok((total, roots))
        })).map_err(failure)?;
        let mut records = Vec::new();
        for root in roots {
            let all = blocking(self.store.spans(root.trace_id.clone())).map_err(failure)?;
            let mut ids = std::collections::HashSet::from([root.span_id.clone()]);
            loop {
                let previous = ids.len();
                for s in &all { if s.parent_span_id.as_ref().is_some_and(|p| ids.contains(p)) { ids.insert(s.span_id.clone()); } }
                if previous==ids.len() { break; }
            }
            let children = all.into_iter().filter(|s| ids.contains(&s.span_id)).map(span).collect::<Vec<_>>();
            let running = root.attributes.get("tron.operation.active").is_some_and(|v| v=="true");
            let mut record = summary(TelemetryTrace {
                trace_id:root.trace_id.clone(), started_at:root.started_at,
                ended_at:if running { None } else { Some(root.ended_at) }, status_code:root.status_code,
                attributes:root.attributes.clone(), abandoned:false,
            }, &children);
            record.root_span_id = Some(root.span_id);
            record.trigger = if root.name == "tron.dude.execute" {
                root.attributes.get("tron.dude.code").cloned().unwrap_or_else(|| record.agent_code.clone())
            } else { record.agent_code.clone() };
            records.push(record);
        }
        if !default_order { sort_trace_summaries(&mut records,&sorts); records=records.into_iter().skip(offset).take(size).collect(); }
        Ok(TracePage { agent_options:Vec::new(), current:q.current,size:q.size,total,records })
    }
}
