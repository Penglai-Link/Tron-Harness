use super::*;
use rusqlite::{params_from_iter, types::Value};

pub(super) fn append_filters(
    sql: &mut String,
    values: &mut Vec<Value>,
    filters: &TraceFilters,
    attrs: &str,
    code: &str,
    unfinished: &str,
    abandoned: &str,
    trace: &str,
    searchable: &str,
) {
    if let Some(agent) = &filters.agent_code {
        sql.push_str(&format!(
            " AND json_extract({attrs},'$.\"tron.agent.code\"')=?"
        ));
        values.push(Value::Text(agent.clone()));
    }
    if let Some(status) = &filters.status {
        let expression = status_sql(attrs, code, unfinished, abandoned);
        sql.push_str(&format!(" AND ({expression})=?"));
        values.push(Value::Text(status.clone()));
    }
    if !filters.keyword.is_empty() {
        sql.push_str(&format!(
            " AND (instr(lower({trace} || ' ' || {searchable}),?)>0"
        ));
        values.push(Value::Text(filters.keyword.clone()));
        for alias in &filters.keyword_aliases {
            sql.push_str(&format!(" OR instr(lower({searchable}),lower(?))>0"));
            values.push(Value::Text(alias.clone()));
        }
        sql.push(')');
    }
}
pub(super) fn status_sql(attrs: &str, code: &str, unfinished: &str, abandoned: &str) -> String {
    format!(
        "CASE WHEN ({unfinished}) OR json_extract({attrs},'$.\"tron.operation.active\"')='true' THEN 'Running' WHEN ({abandoned}) THEN 'Unknown' WHEN json_extract({attrs},'$.\"agent.operation.outcome\"')='unknown' THEN 'Unknown' WHEN json_extract({attrs},'$.\"agent.operation.outcome\"')='cancelled' THEN 'Cancelled' WHEN json_extract({attrs},'$.\"agent.operation.outcome\"') IN ('waiting','awaiting_input') THEN 'Waiting' WHEN json_extract({attrs},'$.\"agent.operation.outcome\"') IN ('interrupted','step_limit','tool_call_limit') THEN 'Interrupted' WHEN json_extract({attrs},'$.\"agent.operation.outcome\"')='error' OR {code}=2 THEN 'Error' ELSE 'Completed' END"
    )
}

/// Match an exact span in a page trace, or an execution root containing that span.
/// UNION deduplicates ancestors so malformed cycles cannot recurse forever.
pub(super) fn append_span_filter(
    sql: &mut String,
    values: &mut Vec<Value>,
    id: Option<&str>,
    trace: &str,
    root: Option<&str>,
) {
    let Some(id) = id else {
        return;
    };
    if let Some(root) = root {
        sql.push_str(&format!(" AND {root} IN (WITH RECURSIVE ancestors(span_id,parent_id) AS (SELECT p.span_id,json_extract(p.data,'$.parentSpanId') FROM ef_telemetry_spans p WHERE p.trace_id={trace} AND p.span_id=? UNION SELECT p.span_id,json_extract(p.data,'$.parentSpanId') FROM ef_telemetry_spans p JOIN ancestors a ON p.span_id=a.parent_id WHERE p.trace_id={trace}) SELECT span_id FROM ancestors)"));
    } else {
        sql.push_str(&format!(" AND EXISTS (SELECT 1 FROM ef_telemetry_spans p WHERE p.trace_id={trace} AND p.span_id=?)"));
    }
    values.push(Value::Text(id.to_owned()));
}

impl TelemetryProjection {
    pub(super) fn trace_agent_options(
        &self,
        workspace: &str,
    ) -> Result<Vec<TraceAgentOption>, ObservabilityError> {
        let workspace = workspace.to_owned();
        blocking(self.database.call_read(move |c| -> rusqlite::Result<_> {
            let mut stmt = c.prepare("SELECT DISTINCT json_extract(data,'$.attributes.\"tron.agent.code\"') FROM ef_telemetry_spans WHERE json_extract(data,'$.name') IN ('tron.session.run','tron.assistant.execute') AND (?1='' OR json_extract(data,'$.attributes.\"tron.workspace.code\"')=?1) ORDER BY 1")?;
            let codes = stmt.query_map([workspace], |r| r.get::<_, Option<String>>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(codes.into_iter().flatten().filter(|c| !c.is_empty()).map(|code| TraceAgentOption{name:code.clone(),code}).collect())
        })).map_err(failure)
    }
    pub(super) fn page_triggers(&self, q: &TraceQuery) -> Result<TracePage, ObservabilityError> {
        let mut clause = "json_extract(t.attributes,'$.\"tron.trace.page\"')='true' AND COALESCE(json_extract(t.attributes,'$.\"url.path\"'),'') NOT IN ('/api/v1/profiles/events','/api/v1/system/skins/events')".to_owned();
        if q.category.is_none() {
            clause = "1=1".into();
        }
        let mut values = Vec::<Value>::new();
        for (key, value) in [
            (
                "tron.workspace.code",
                Some(q.workspace_code.clone()).filter(|v| !v.is_empty()),
            ),
            ("tron.session.code", q.session_code.clone()),
        ] {
            if let Some(value) = value {
                clause.push_str(&format!(" AND json_extract(t.attributes,'$.\"{key}\"')=?"));
                values.push(Value::Text(value));
            }
        }
        if let Some(id) = &q.trace_id {
            clause.push_str(" AND t.trace_id=?");
            values.push(Value::Text(id.clone()));
        }
        append_span_filter(
            &mut clause,
            &mut values,
            q.filters.span_id.as_deref(),
            "t.trace_id",
            None,
        );
        append_filters(
            &mut clause,
            &mut values,
            &q.filters,
            "t.attributes",
            "t.status_code",
            "t.ended_at IS NULL",
            "t.abandoned",
            "t.trace_id",
            "t.attributes",
        );
        let sorts = list_query::sorts(q);
        let metric_sort = list_query::needs_metrics(&sorts);
        let order = list_query::order(&sorts);
        let limit = format!("LIMIT {} OFFSET {}", q.size, (q.current - 1) * q.size);
        let timestamp = now();
        let raw_roots = format!(
            "SELECT t.*, '' AS root_span_id,MAX(0,COALESCE(t.ended_at,{timestamp})-t.started_at) AS duration_ms FROM ef_telemetry_traces t WHERE {clause}"
        );
        let roots = if metric_sort {
            raw_roots
        } else {
            format!("SELECT * FROM ({raw_roots}) r ORDER BY {order} {limit}")
        };
        let columns = list_query::METRIC_COLUMNS;
        let sql=format!("WITH roots AS MATERIALIZED ({roots}) SELECT r.trace_id,r.started_at,r.ended_at,r.status_code,r.attributes,r.abandoned,{columns},
            (SELECT p.data FROM ef_telemetry_spans p WHERE p.trace_id=r.trace_id
                AND (json_extract(p.data,'$.name') LIKE 'page %' OR json_extract(p.data,'$.parentSpanId') IS NULL)
                ORDER BY CASE WHEN json_extract(p.data,'$.name') LIKE 'page %' THEN 0 ELSE 1 END,json_extract(p.data,'$.startedAt'),p.span_id LIMIT 1)
        FROM roots r LEFT JOIN ef_telemetry_trace_rollups m ON m.trace_id=r.trace_id ORDER BY {order} {}",if metric_sort {limit} else {String::new()});
        let (total,rows)=blocking(self.database.call_read(move |c| -> rusqlite::Result<_> {
            let transaction=crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(c,rusqlite::TransactionBehavior::Deferred)?;
            let total:usize=transaction.query_row(&format!("SELECT COUNT(*) FROM ef_telemetry_traces t WHERE {clause}"),params_from_iter(&values),|r|r.get(0))?;
            let rows=if total==0 {Vec::new()} else {
                let mut statement=transaction.prepare(&sql)?;
                statement.query_map(params_from_iter(&values),|r|Ok((
                    (r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,Option<i64>>(2)?,r.get::<_,i32>(3)?,r.get::<_,String>(4)?,r.get::<_,bool>(5)?),
                    list_query::Metrics::read(r,6)?,r.get::<_,Option<String>>(13)?
                )))?.collect::<rusqlite::Result<Vec<_>>>()?
            };
            transaction.commit()?;
            Ok((total,rows))
        })).map_err(failure)?;
        let records = rows
            .into_iter()
            .map(
                |(
                    (trace_id, started_at, ended_at, status_code, attributes, abandoned),
                    metrics,
                    root,
                )| {
                    let trace = TelemetryTrace {
                        trace_id,
                        started_at,
                        ended_at,
                        status_code,
                        abandoned,
                        attributes: serde_json::from_str(&attributes).map_err(failure)?,
                    };
                    let root = root
                        .map(|data| {
                            serde_json::from_str::<TelemetrySpan>(&data)
                                .map(span)
                                .map_err(failure)
                        })
                        .transpose()?;
                    let mut record = summary(trace, root.as_slice());
                    metrics.apply(&mut record);
                    Ok(record)
                },
            )
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
