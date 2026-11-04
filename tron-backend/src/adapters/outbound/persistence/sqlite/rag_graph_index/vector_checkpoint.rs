//! Atomic append/adoption of immutable, compact node-vector batches.
use super::*;

fn manifest(raw: &str, task: &str) -> Result<GraphNodeVectorManifest, String> {
    let value: GraphNodeVectorManifest = serde_json::from_str(raw)
        .map_err(|_| err("node-vector checkpoint requires verified format conversion"))?;
    value.validate()?;
    if value.storage_task_id != task {
        return Err(err("node-vector checkpoint owner differs"));
    }
    Ok(value)
}
fn validate_key(key: &str) -> Result<(), String> {
    if !key.starts_with("NodeVectors_") || key.contains(':') || key.len() > 256 {
        return Err(err("invalid node-vector checkpoint key"));
    }
    Ok(())
}
fn prefix(key: &str) -> String {
    format!("{key}:Batch:")
}

pub(super) fn load(
    connection: &mut Connection,
    task: &str,
    scope: &str,
    key: &str,
) -> Result<Option<GraphNodeVectorCheckpoint>, String> {
    validate_key(key)?;
    let tx =
        business_transaction(connection, rusqlite::TransactionBehavior::Immediate).map_err(err)?;
    let row: Option<(String, String)> = tx.query_row(
        "SELECT c.task_id,c.payload_json FROM rag_graph_task_checkpoints c \
         WHERE c.cache_scope=?2 AND c.checkpoint_key=?3 AND \
         (c.task_id=?1 OR EXISTS(SELECT 1 FROM rag_graph_active_sources a JOIN rag_graph_generations g \
         ON g.generation_id=a.generation_id WHERE g.task_id=c.task_id AND g.state='Active' AND g.vector_cache_invalidated=0)) \
         ORDER BY (c.task_id=?1) DESC,c.updated_at DESC LIMIT 1",
        params![task,scope,key], |r| Ok((r.get(0)?,r.get(1)?)),
    ).optional().map_err(err)?;
    let Some((owner, raw)) = row else {
        return Ok(None);
    };
    let mut saved = manifest(&raw, &owner)?;
    let lower = prefix(key);
    let upper = format!("{lower}\u{10ffff}");
    let mut batches = Vec::new();
    {
        let mut statement = tx
            .prepare(
                "SELECT checkpoint_key,payload_json FROM rag_graph_task_checkpoints \
             WHERE task_id=?1 AND cache_scope=?2 AND checkpoint_key>=?3 AND checkpoint_key<?4 \
             ORDER BY checkpoint_key",
            )
            .map_err(err)?;
        let mut rows = statement
            .query(params![owner, scope, lower, upper])
            .map_err(err)?;
        let mut offset = 0usize;
        while let Some(row) = rows.next().map_err(err)? {
            let batch_key: String = row.get(0).map_err(err)?;
            if batches.len() >= saved.batch_count
                || batch_key != graph_node_vector_batch_key(key, batches.len())
            {
                return Err(err("node-vector batch ordinal differs"));
            }
            let raw: String = row.get(1).map_err(err)?;
            let batch: GraphNodeVectorBatch = serde_json::from_str(&raw).map_err(err)?;
            if batch.count == 0 || batch.offset != offset {
                return Err(err("node-vector batches are not contiguous"));
            }
            offset = offset
                .checked_add(batch.count)
                .ok_or("node-vector checkpoint too large")?;
            if offset > saved.completed_units {
                return Err(err("node-vector batch exceeds manifest"));
            }
            batches.push(batch);
        }
        if batches.len() != saved.batch_count || offset != saved.completed_units {
            return Err(err("node-vector checkpoint is incomplete"));
        }
    }
    if owner != task {
        // Copy exact encoded bytes, without deserializing historical floats.
        // The new task owns its prefix before the old generation can be retired.
        let now = chrono::Utc::now().timestamp_millis();
        tx.execute(
            "INSERT INTO rag_graph_task_checkpoints(task_id,cache_scope,checkpoint_key,payload_json,updated_at) \
             SELECT ?1,cache_scope,checkpoint_key,payload_json,?6 FROM rag_graph_task_checkpoints \
             WHERE task_id=?2 AND cache_scope=?3 AND checkpoint_key>=?4 AND checkpoint_key<?5",
            params![task,owner,scope,lower,upper,now],
        ).map_err(err)?;
        saved.storage_task_id = task.into();
        tx.execute(
            "INSERT INTO rag_graph_task_checkpoints(task_id,cache_scope,checkpoint_key,payload_json,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![task,scope,key,serde_json::to_string(&saved).map_err(err)?,now],
        ).map_err(err)?;
    }
    tx.commit().map_err(err)?;
    Ok(Some(GraphNodeVectorCheckpoint {
        manifest: saved,
        batches,
    }))
}

pub(super) fn save(
    connection: &mut Connection,
    task: &str,
    scope: &str,
    key: &str,
    expected: usize,
    next: &GraphNodeVectorManifest,
    batch: Option<&GraphNodeVectorBatch>,
) -> Result<(), String> {
    validate_key(key)?;
    next.validate()?;
    if next.storage_task_id != task {
        return Err(err("node-vector checkpoint owner differs"));
    }
    if let Some(batch) = batch {
        // Bounded to one provider batch; never decode or rewrite the historical prefix.
        batch.decode(next.dimension)?;
        if batch.offset != expected
            || batch.offset.checked_add(batch.count) != Some(next.completed_units)
        {
            return Err(err("node-vector batch offset differs"));
        }
    }
    let tx =
        business_transaction(connection, rusqlite::TransactionBehavior::Immediate).map_err(err)?;
    let raw: Option<String> = tx.query_row(
        "SELECT payload_json FROM rag_graph_task_checkpoints WHERE task_id=?1 AND cache_scope=?2 AND checkpoint_key=?3",
        params![task,scope,key], |r| r.get(0),
    ).optional().map_err(err)?;
    let previous = raw.as_deref().map(|s| manifest(s, task)).transpose()?;
    if let Some(old) = &previous {
        if old == next {
            if let Some(batch) = batch {
                let stored: Option<String> = tx.query_row(
                    "SELECT payload_json FROM rag_graph_task_checkpoints WHERE task_id=?1 AND cache_scope=?2 AND checkpoint_key=?3",
                    params![task,scope,graph_node_vector_batch_key(key,next.batch_count-1)], |r| r.get(0),
                ).optional().map_err(err)?;
                if stored
                    .as_deref()
                    .map(serde_json::from_str::<GraphNodeVectorBatch>)
                    .transpose()
                    .map_err(err)?
                    .as_ref()
                    != Some(batch)
                {
                    return Err(err("node-vector checkpoint replay differs"));
                }
            } else if expected != old.completed_units {
                return Err(err("node-vector manifest offset differs"));
            }
            return Ok(());
        }
        if old.model_code != next.model_code
            || old.fingerprint != next.fingerprint
            || old.dimension != next.dimension
            || (old.completed_units > 0 && old.provider_model != next.provider_model)
        {
            return Err(err("node-vector model space differs"));
        }
    }
    let old_completed = previous.as_ref().map_or(0, |m| m.completed_units);
    let old_batches = previous.as_ref().map_or(0, |m| m.batch_count);
    if old_completed != expected
        || next.batch_count != old_batches + usize::from(batch.is_some())
        || (batch.is_none() && next.completed_units != old_completed)
    {
        return Err(err("node-vector checkpoint prefix changed"));
    }
    let now = chrono::Utc::now().timestamp_millis();
    if let Some(batch) = batch {
        tx.execute(
            "INSERT INTO rag_graph_task_checkpoints(task_id,cache_scope,checkpoint_key,payload_json,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![task,scope,graph_node_vector_batch_key(key,old_batches),serde_json::to_string(batch).map_err(err)?,now],
        ).map_err(err)?;
    }
    tx.execute(
        "INSERT INTO rag_graph_task_checkpoints(task_id,cache_scope,checkpoint_key,payload_json,updated_at) \
         VALUES (?1,?2,?3,?4,?5) ON CONFLICT(task_id,cache_scope,checkpoint_key) \
         DO UPDATE SET payload_json=excluded.payload_json,updated_at=excluded.updated_at",
        params![task,scope,key,serde_json::to_string(next).map_err(err)?,now],
    ).map_err(err)?;
    tx.commit().map_err(err)
}
