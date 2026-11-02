//! Session-owned telemetry cleanup; shared traces preserve other executions.
use rusqlite::{Connection, params};
use std::collections::{HashMap, HashSet, VecDeque};

pub(crate) fn initialize_session_telemetry_indexes(c: &Connection) -> rusqlite::Result<()> {
    // The host owns this attribute. Keep its projection index out of Framework
    // and build it once at telemetry startup, never during a deletion transaction.
    c.execute_batch(
        "CREATE INDEX IF NOT EXISTS tron_telemetry_spans_session
         ON ef_telemetry_spans(json_extract(data,'$.attributes.\"tron.session.code\"'),trace_id)
         WHERE json_extract(data,'$.attributes.\"tron.session.code\"') IS NOT NULL;",
    )
}

pub(super) const SESSION_TRACE_LOOKUP_SQL: &str =
    "SELECT trace_id FROM telemetry.ef_telemetry_trace_attributes
     WHERE attribute='tron.session.code' AND value=?1
     UNION SELECT trace_id FROM telemetry.ef_telemetry_spans
     WHERE json_extract(data,'$.attributes.\"tron.session.code\"')=?1";

/// Remove the execution spans owned by a deleted session. A trace can also
/// contain an HTTP span or another session, so deleting the trace unconditionally
/// would erase unrelated observability data.
pub(super) fn delete_session_telemetry(c: &Connection, code: &str) -> rusqlite::Result<()> {
    let exists: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM telemetry.sqlite_master WHERE type='table' AND name='ef_telemetry_traces')",
        [], |row| row.get(0),
    )?;
    if !exists {
        return Ok(());
    }
    let trace_ids = c
        .prepare(SESSION_TRACE_LOOKUP_SQL)?
        .query_map([code], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for trace_id in trace_ids {
        let spans = c.prepare(
            "SELECT span_id,json_extract(data,'$.parentSpanId'),json_extract(data,'$.attributes.\"tron.session.code\"')
             FROM telemetry.ef_telemetry_spans WHERE trace_id=?1",
        )?.query_map([&trace_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, Option<String>>(2)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut children: HashMap<String, Vec<String>> = HashMap::new();
        let mut owners = HashMap::new();
        let mut doomed = HashSet::new();
        let mut queue = VecDeque::new();
        for (id, parent, owner) in &spans {
            if let Some(parent) = parent {
                children.entry(parent.clone()).or_default().push(id.clone());
            }
            owners.insert(id.clone(), owner.clone());
            if owner.as_deref() == Some(code) {
                doomed.insert(id.clone());
                queue.push_back(id.clone());
            }
        }
        while let Some(parent) = queue.pop_front() {
            for child in children.get(&parent).into_iter().flatten() {
                // A child execution for a different session must survive even
                // when its parent belongs to the deleted session.
                if owners
                    .get(child)
                    .and_then(Option::as_deref)
                    .is_some_and(|owner| owner != code)
                {
                    continue;
                }
                if doomed.insert(child.clone()) {
                    queue.push_back(child.clone());
                }
            }
        }
        if !spans.is_empty() && doomed.len() == spans.len() {
            c.execute(
                "DELETE FROM telemetry.ef_telemetry_traces WHERE trace_id=?1",
                [&trace_id],
            )?;
            continue;
        }
        let doomed_json =
            serde_json::to_string(&doomed).map_err(|_| rusqlite::Error::InvalidQuery)?;
        c.execute("DELETE FROM telemetry.ef_telemetry_spans WHERE trace_id=?1 AND span_id IN (SELECT value FROM json_each(?2))", params![trace_id, doomed_json])?;
        c.execute("DELETE FROM telemetry.ef_telemetry_span_rollups WHERE trace_id=?1 AND span_id IN (SELECT value FROM json_each(?2))", params![trace_id, doomed_json])?;
        // Preserve surviving execution roots without dangling parent links.
        let detached = spans
            .iter()
            .filter(|(id, parent, _)| {
                !doomed.contains(id)
                    && parent
                        .as_ref()
                        .is_some_and(|parent| doomed.contains(parent))
            })
            .map(|(id, _, _)| id)
            .collect::<Vec<_>>();
        let detached =
            serde_json::to_string(&detached).map_err(|_| rusqlite::Error::InvalidQuery)?;
        c.execute("UPDATE telemetry.ef_telemetry_spans SET data=json_set(data,'$.parentSpanId',NULL) WHERE trace_id=?1 AND span_id IN (SELECT value FROM json_each(?2))", params![trace_id, detached])?;
        c.execute("UPDATE telemetry.ef_telemetry_span_rollups SET parent_id=NULL WHERE trace_id=?1 AND span_id IN (SELECT value FROM json_each(?2))", params![trace_id, detached])?;
        if spans.is_empty() {
            c.execute(
                "DELETE FROM telemetry.ef_telemetry_traces WHERE trace_id=?1",
                [&trace_id],
            )?;
            continue;
        }
        // A shared trace may have been registered with the deleted session's
        // metadata. Remove only that association.
        c.execute("UPDATE telemetry.ef_telemetry_traces SET attributes=json_remove(attributes,'$.\"tron.session.code\"','$.\"tron.run.code\"','$.\"tron.agent.code\"','$.\"tron.trace.agent\"') WHERE trace_id=?1 AND json_extract(attributes,'$.\"tron.session.code\"')=?2", params![trace_id, code])?;
        if !doomed.is_empty() {
            rebuild_trace_rollups(c, &trace_id)?;
        }
    }
    Ok(())
}

fn rebuild_trace_rollups(c: &Connection, trace_id: &str) -> rusqlite::Result<()> {
    type Counts = [i64; 7];
    let nodes = c.prepare("SELECT span_id,parent_id,own_span_count,own_error_count,own_model_count,own_tool_count,own_input_tokens,own_cached_input_tokens,own_output_tokens,span_count,error_count,model_count,tool_count,input_tokens,cached_input_tokens,output_tokens FROM telemetry.ef_telemetry_span_rollups WHERE trace_id=?1")?
        .query_map([trace_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, [row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?,row.get(8)?], [row.get::<_, i64>(9)?,row.get(10)?,row.get(11)?,row.get(12)?,row.get(13)?,row.get(14)?,row.get(15)?])))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let parents: HashMap<_, _> = nodes
        .iter()
        .map(|(id, parent, _, _)| (id.clone(), parent.clone()))
        .collect();
    let mut totals: HashMap<String, Counts> = nodes
        .iter()
        .map(|(id, _, _, _)| (id.clone(), [0; 7]))
        .collect();
    let mut trace_total = [0; 7];
    for (id, _, own, _) in &nodes {
        for (sum, value) in trace_total.iter_mut().zip(own) {
            *sum += value;
        }
        let mut current = Some(id.as_str());
        let mut seen = HashSet::new();
        while let Some(node) = current {
            if !seen.insert(node) {
                break;
            }
            let Some(total) = totals.get_mut(node) else {
                break;
            };
            for (sum, value) in total.iter_mut().zip(own) {
                *sum += value;
            }
            current = parents.get(node).and_then(Option::as_deref);
        }
    }
    let changed = nodes
        .iter()
        .filter_map(|(id, _, _, previous)| {
            let counts = totals.get(id)?;
            (counts != previous).then_some((id, counts))
        })
        .collect::<Vec<_>>();
    if !changed.is_empty() {
        let changed = serde_json::to_string(&changed).map_err(|_| rusqlite::Error::InvalidQuery)?;
        c.execute("UPDATE telemetry.ef_telemetry_span_rollups AS r SET
            span_count=json_extract(change.value,'$[1][0]'),error_count=json_extract(change.value,'$[1][1]'),
            model_count=json_extract(change.value,'$[1][2]'),tool_count=json_extract(change.value,'$[1][3]'),
            input_tokens=json_extract(change.value,'$[1][4]'),cached_input_tokens=json_extract(change.value,'$[1][5]'),
            output_tokens=json_extract(change.value,'$[1][6]')
            FROM json_each(?2) AS change
            WHERE r.trace_id=?1 AND r.span_id=json_extract(change.value,'$[0]')", params![trace_id, changed])?;
    }
    c.execute("UPDATE telemetry.ef_telemetry_trace_rollups SET span_count=?2,error_count=?3,model_count=?4,tool_count=?5,input_tokens=?6,cached_input_tokens=?7,output_tokens=?8 WHERE trace_id=?1",
        params![trace_id,trace_total[0],trace_total[1],trace_total[2],trace_total[3],trace_total[4],trace_total[5],trace_total[6]])?;
    Ok(())
}
