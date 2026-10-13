use super::*;
use rusqlite::{params_from_iter, types::Value};

pub(super) fn append_filters(sql: &mut String, values: &mut Vec<Value>, filters: &TraceFilters, attrs: &str, code: &str, unfinished: &str, abandoned: &str, trace: &str, searchable: &str) {
    if let Some(agent) = &filters.agent_code {
        sql.push_str(&format!(" AND json_extract({attrs},'$.\"tron.agent.code\"')=?"));
        values.push(Value::Text(agent.clone()));
    }
    if let Some(status) = &filters.status {
        sql.push_str(&format!(" AND (CASE WHEN ({unfinished}) OR json_extract({attrs},'$.\"tron.operation.active\"')='true' THEN 'Running' WHEN ({abandoned}) THEN 'Unknown' WHEN json_extract({attrs},'$.\"agent.operation.outcome\"')='unknown' THEN 'Unknown' WHEN json_extract({attrs},'$.\"agent.operation.outcome\"')='cancelled' THEN 'Cancelled' WHEN json_extract({attrs},'$.\"agent.operation.outcome\"') IN ('waiting','awaiting_input') THEN 'Waiting' WHEN json_extract({attrs},'$.\"agent.operation.outcome\"') IN ('interrupted','step_limit','tool_call_limit') THEN 'Interrupted' WHEN json_extract({attrs},'$.\"agent.operation.outcome\"')='error' OR {code}=2 THEN 'Error' ELSE 'Completed' END)=?"));
        values.push(Value::Text(status.clone()));
    }
    if !filters.keyword.is_empty() {
        sql.push_str(&format!(" AND (instr(lower({trace} || ' ' || {searchable}),?)>0"));
        values.push(Value::Text(filters.keyword.clone()));
        for alias in &filters.keyword_aliases {
            sql.push_str(&format!(" OR instr(lower({searchable}),lower(?))>0"));
            values.push(Value::Text(alias.clone()));
        }
        sql.push(')');
    }
}
/// Match an exact span in a page trace, or an execution root containing that span.
/// UNION deduplicates ancestors so malformed cycles cannot recurse forever.
pub(super) fn append_span_filter(sql: &mut String, values: &mut Vec<Value>, id: Option<&str>, trace: &str, root: Option<&str>) {
    let Some(id) = id else { return; };
    if let Some(root) = root {
        sql.push_str(&format!(" AND {root} IN (WITH RECURSIVE ancestors(span_id,parent_id) AS (SELECT p.span_id,json_extract(p.data,'$.parentSpanId') FROM ef_telemetry_spans p WHERE p.trace_id={trace} AND p.span_id=? UNION SELECT p.span_id,json_extract(p.data,'$.parentSpanId') FROM ef_telemetry_spans p JOIN ancestors a ON p.span_id=a.parent_id WHERE p.trace_id={trace}) SELECT span_id FROM ancestors)"));
    } else {
        sql.push_str(&format!(" AND EXISTS (SELECT 1 FROM ef_telemetry_spans p WHERE p.trace_id={trace} AND p.span_id=?)"));
    }
    values.push(Value::Text(id.to_owned()));
}

impl TelemetryProjection {
    pub(super) fn trace_agent_options(&self, workspace: &str) -> Result<Vec<TraceAgentOption>, ObservabilityError> {
        let workspace = workspace.to_owned();
        blocking(self.database.call(move |c| -> rusqlite::Result<_> {
            let mut stmt = c.prepare("SELECT DISTINCT json_extract(data,'$.attributes.\"tron.agent.code\"') FROM ef_telemetry_spans WHERE json_extract(data,'$.name') IN ('tron.session.run','tron.dude.execute') AND (?1='' OR json_extract(data,'$.attributes.\"tron.workspace.code\"')=?1) ORDER BY 1")?;
            let codes = stmt.query_map([workspace], |r| r.get::<_, Option<String>>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(codes.into_iter().flatten().filter(|c| !c.is_empty()).map(|code| TraceAgentOption{name:code.clone(),code}).collect())
        })).map_err(failure)
    }
    pub(super) fn page_triggers(&self, q: &TraceQuery) -> Result<TracePage, ObservabilityError> {
        let mut clause = "json_extract(t.attributes,'$.\"tron.trace.page\"')='true' AND COALESCE(json_extract(t.attributes,'$.\"url.path\"'),'') NOT IN ('/api/v1/profiles/events','/api/v1/system/skins/events')".to_owned();
        let mut values = Vec::<Value>::new();
        for (key,value) in [("tron.workspace.code",Some(q.workspace_code.clone()).filter(|v| !v.is_empty())),("tron.session.code",q.session_code.clone())] {
            if let Some(value)=value { clause.push_str(" AND json_extract(t.attributes,?)=?"); values.push(Value::Text(format!("$.\"{key}\""))); values.push(Value::Text(value)); }
        }
        if let Some(id)=&q.trace_id { clause.push_str(" AND t.trace_id=?"); values.push(Value::Text(id.clone())); }
        append_span_filter(&mut clause, &mut values, q.filters.span_id.as_deref(), "t.trace_id", None);
        append_filters(&mut clause,&mut values,&q.filters,"t.attributes","t.status_code","t.ended_at IS NULL","t.abandoned","t.trace_id","t.attributes");
        let sorts = if q.sorts.is_empty() {vec![TraceSort{field:q.sort_by,direction:q.sort_direction}]} else {q.sorts.clone()};
        let default_order=sorts.len()==1 && sorts[0].field==TraceSortField::StartedAt && sorts[0].direction==TraceSortDirection::Desc;
        let offset=(q.current-1)*q.size;
        let size=q.size;
        let (total,traces)=blocking(self.database.call(move |c| -> rusqlite::Result<_> {
            let total=c.query_row(&format!("SELECT COUNT(*) FROM ef_telemetry_traces t WHERE {clause}"),params_from_iter(&values),|r|r.get(0))?;
            let mut sql=format!("SELECT trace_id,started_at,ended_at,status_code,attributes,abandoned FROM ef_telemetry_traces t WHERE {clause} ORDER BY started_at DESC,trace_id DESC");
            if default_order {sql.push_str(" LIMIT ? OFFSET ?");values.push(Value::Integer(size as i64));values.push(Value::Integer(offset as i64));}
            let mut stmt=c.prepare(&sql)?;
            let records=stmt.query_map(params_from_iter(values),|r| {
                let attrs:String=r.get(4)?;
                Ok(TelemetryTrace{trace_id:r.get(0)?,started_at:r.get(1)?,ended_at:r.get(2)?,status_code:r.get(3)?,attributes:serde_json::from_str(&attrs).map_err(|_|rusqlite::Error::InvalidQuery)?,abandoned:r.get(5)?})
            })?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok((total,records))
        })).map_err(failure)?;
        let mut records=Vec::new();
        for trace in traces {
            let spans=blocking(self.store.spans(trace.trace_id.clone())).map_err(failure)?.into_iter().map(span).collect::<Vec<_>>();
            records.push(summary(trace,&spans));
        }
        if !default_order {sort_trace_summaries(&mut records,&sorts);records=records.into_iter().skip(offset).take(size).collect();}
        Ok(TracePage{agent_options:Vec::new(),current:q.current,size:q.size,total,records})
    }
}
