use super::*;
use GraphSnapshotError as Failure;

fn storage(e: impl std::fmt::Display) -> Failure {
    Failure::Storage(e.to_string())
}

fn has_private_payload(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(fields) => fields.iter().any(|(key, value)| {
            let key = key.replace('_', "").to_ascii_lowercase();
            matches!(
                key.as_str(),
                "embedding"
                    | "embeddings"
                    | "vectors"
                    | "queryembedding"
                    | "queryvector"
                    | "apikey"
            ) || has_private_payload(value)
        }),
        serde_json::Value::Array(values) => values.iter().any(has_private_payload),
        _ => false,
    }
}

fn check_generations(connection: &Connection, ids: &BTreeSet<String>) -> Result<(), Failure> {
    let count: usize = connection
        .query_row(
            "SELECT count(*) FROM rag_graph_generations g \
        JOIN rag_graph_active_sources a ON a.generation_id=g.generation_id \
        WHERE g.state='Active' AND g.generation_id IN (SELECT value FROM json_each(?1))",
            [serde_json::to_string(ids).map_err(storage)?],
            |r| r.get(0),
        )
        .map_err(storage)?;
    if count != ids.len() {
        return Err(Failure::PublicationChanged);
    }
    Ok(())
}

pub(super) fn save(
    connection: &Connection,
    draft: GraphSnapshotDraft,
    now: i64,
) -> Result<GraphQuerySnapshot, Failure> {
    if draft.identity_hash.is_empty()
        || draft.owner_hash.is_empty()
        || draft.generation_ids.is_empty()
        || draft.generation_ids.len() > 10_000
        || !draft.request.is_object()
        || !draft.traversal.is_object()
        || draft.expires_at <= now
        || draft.expires_at > now.saturating_add(GRAPH_SNAPSHOT_TTL_MS)
    {
        return Err(Failure::Invalid);
    }
    let request = serde_json::to_string(&draft.request).map_err(storage)?;
    let traversal = serde_json::to_string(&draft.traversal).map_err(storage)?;
    if has_private_payload(&draft.request) || has_private_payload(&draft.traversal) {
        return Err(Failure::Invalid);
    }
    if request.len() > 65_536 || traversal.len() > 2 * 1024 * 1024 {
        return Err(Failure::Invalid);
    }
    let cursor = rand::random::<[u8; 32]>()
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect::<String>();
    let tx = business_transaction(connection, rusqlite::TransactionBehavior::Immediate)
        .map_err(storage)?;
    check_generations(&tx, &draft.generation_ids)?;
    tx.execute(
        "DELETE FROM rag_graph_query_snapshots WHERE expires_at<=?1",
        [now],
    )
    .map_err(storage)?;
    tx.execute("INSERT INTO rag_graph_query_snapshots(snapshot_id,owner_hash,identity_hash,request_json,traversal_json,created_at,expires_at) \
        VALUES (?1,?2,?3,?4,?5,?6,?7)",params![cursor,draft.owner_hash,draft.identity_hash,request,traversal,now,draft.expires_at]).map_err(storage)?;
    for generation in &draft.generation_ids {
        tx.execute(
            "INSERT INTO rag_graph_snapshot_generations(snapshot_id,generation_id) VALUES (?1,?2)",
            params![cursor, generation],
        )
        .map_err(storage)?;
    }
    // The cap belongs to the user, across Agents and workspaces. Keep the just
    // created cursor even if several pages share the same millisecond timestamp.
    tx.execute(
        "DELETE FROM rag_graph_query_snapshots WHERE snapshot_id IN ( \
        SELECT snapshot_id FROM rag_graph_query_snapshots WHERE owner_hash=?1 AND snapshot_id<>?2 \
        ORDER BY created_at DESC,snapshot_id DESC LIMIT -1 OFFSET ?3)",
        params![
            draft.owner_hash,
            cursor,
            (GRAPH_SNAPSHOTS_PER_USER - 1) as i64
        ],
    )
    .map_err(storage)?;
    tx.commit().map_err(storage)?;
    Ok(GraphQuerySnapshot {
        cursor,
        owner_hash: draft.owner_hash,
        identity_hash: draft.identity_hash,
        request: draft.request,
        traversal: draft.traversal,
        generation_ids: draft.generation_ids,
        created_at: now,
        expires_at: draft.expires_at,
    })
}

pub(super) fn load(
    connection: &Connection,
    identity: &str,
    cursor: &str,
    now: i64,
) -> Result<GraphQuerySnapshot, Failure> {
    if identity.is_empty() || cursor.len() != 64 || !cursor.bytes().all(|v| v.is_ascii_hexdigit()) {
        return Err(Failure::Unavailable);
    }
    let tx = business_transaction(connection, rusqlite::TransactionBehavior::Deferred)
        .map_err(storage)?;
    let row = tx
        .query_row(
            "SELECT owner_hash,request_json,traversal_json,created_at,expires_at \
        FROM rag_graph_query_snapshots WHERE snapshot_id=?1 AND identity_hash=?2",
            params![cursor, identity],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                ))
            },
        )
        .optional()
        .map_err(storage)?
        .ok_or(Failure::Unavailable)?;
    if row.4 <= now {
        return Err(Failure::Unavailable);
    }
    let generations = {
        let mut statement=tx.prepare("SELECT generation_id FROM rag_graph_snapshot_generations WHERE snapshot_id=?1 ORDER BY generation_id").map_err(storage)?;
        statement
            .query_map([cursor], |r| r.get::<_, String>(0))
            .map_err(storage)?
            .collect::<Result<BTreeSet<_>, _>>()
            .map_err(storage)?
    };
    if generations.is_empty() {
        return Err(Failure::Unavailable);
    }
    check_generations(&tx, &generations)?;
    Ok(GraphQuerySnapshot {
        cursor: cursor.into(),
        identity_hash: identity.into(),
        owner_hash: row.0,
        request: serde_json::from_str(&row.1).map_err(storage)?,
        traversal: serde_json::from_str(&row.2).map_err(storage)?,
        generation_ids: generations,
        created_at: row.3,
        expires_at: row.4,
    })
}
