//! Read only report metadata for the requested history steps, never full prompts.
use crate::application::{
    agent::FrameworkHost,
    short_term_memory::{CompactionBlock, compaction_blocks},
};
use eliteforge_agent_core as fw;
use std::collections::BTreeMap;

pub(crate) async fn read_compaction_details(
    host: &FrameworkHost,
    session: &str,
    steps: &[fw::HistoryStep],
) -> Result<BTreeMap<String, Vec<CompactionBlock>>, fw::StateError> {
    let windows: Vec<_> = steps
        .iter()
        .filter(|step| step.name == "framework.context.compaction" && step.finished_at_ms.is_some())
        .map(|step| {
            serde_json::json!({
                "id": step.id, "runId": step.run_id,
                "start": step.started_at_ms, "end": step.finished_at_ms,
            })
        })
        .collect();
    if windows.is_empty() {
        return Ok(BTreeMap::new());
    }
    let windows = serde_json::to_string(&windows)
        .map_err(|error| fw::StateError::Backend(error.to_string()))?;
    let session = session.to_owned();
    let rows = host
        .shared_database()
        .await?
        .call_read(
            move |connection| -> rusqlite::Result<Vec<(String, String)>> {
                let mut statement = connection.prepare(
            "SELECT json_extract(w.value, '$.id'), json_extract(e.record, '$.payload.contextReport')
             FROM json_each(?2) w JOIN ef_events e
               ON e.session_id=?1 AND e.run_id=json_extract(w.value, '$.runId')
              AND e.kind='tron.context-view'
             WHERE json_extract(e.record, '$.created_at_ms') BETWEEN
                   json_extract(w.value, '$.start') AND json_extract(w.value, '$.end')
             ORDER BY e.sequence"
        )?;
                statement
                    .query_map(rusqlite::params![session, windows], |row| {
                        Ok((row.get(0)?, row.get(1)?))
                    })?
                    .collect()
            },
        )
        .await
        .map_err(|error| fw::StateError::Backend(error.to_string()))?;
    let mut result = BTreeMap::new();
    for (id, report) in rows {
        let report = serde_json::from_str(&report)
            .map_err(|error| fw::StateError::Backend(error.to_string()))?;
        result.insert(id, compaction_blocks(&report));
    }
    Ok(result)
}
