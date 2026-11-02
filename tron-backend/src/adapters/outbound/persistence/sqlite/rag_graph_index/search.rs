//! Native node recall. Authorization and vector-space predicates precede ranking.
use super::*;

const KNN_CANDIDATE_LIMIT: usize = 4096;

pub(super) fn search_nodes(
    connection: &Connection,
    query: GraphNodeSearchQuery,
) -> Result<GraphNodeSearchPage, String> {
    if !(1..=100).contains(&query.limit) {
        return Err(err("invalid node search budget"));
    }
    if query.generation_ids.is_empty() {
        return Ok(GraphNodeSearchPage::default());
    }
    let generations = serde_json::to_string(&query.generation_ids).map_err(err)?;
    // Recheck the SQL publication pointer in the same query as the actual recall.
    let eligible = "SELECT p.id FROM rag_graph_node_projections p \
        JOIN rag_graph_generations g ON g.generation_id=p.generation_id \
        JOIN rag_graph_active_sources a ON a.generation_id=g.generation_id \
        WHERE g.state='Active' AND g.generation_id IN (SELECT value FROM json_each(?1))";
    let (sql, parameters, vector) = match query.matching {
        GraphNodeMatch::Browse { excluded_ids } => (
            format!(
                "WITH ranked AS ( \
                SELECT p.entity_id,p.generation_id,p.chunk_codes_json, \
                CAST(json_array_length(p.chunk_codes_json) AS REAL) AS score, \
                row_number() OVER(PARTITION BY p.entity_id ORDER BY json_array_length(p.chunk_codes_json) DESC,p.generation_id) AS position \
                FROM rag_graph_node_projections p WHERE p.id IN ({eligible})), balanced AS ( \
                SELECT entity_id,generation_id,chunk_codes_json,score, \
                row_number() OVER(PARTITION BY generation_id ORDER BY score DESC,entity_id) AS source_position \
                FROM ranked WHERE position=1 AND entity_id NOT IN (SELECT value FROM json_each(?2))) \
                SELECT entity_id,generation_id,chunk_codes_json,score,0 FROM balanced \
                ORDER BY source_position,generation_id,entity_id LIMIT ?3"
            ),
            vec![
                generations.into(),
                serde_json::to_string(&excluded_ids).map_err(err)?.into(),
                ((query.limit + 1) as i64).into(),
            ],
            false,
        ),
        GraphNodeMatch::Fulltext(text) => {
            if text.trim().is_empty() {
                return Err(err("node fulltext query is empty"));
            }
            (
                format!(
                    "WITH candidates AS MATERIALIZED ( \
                    SELECT p.entity_id,p.generation_id,p.chunk_codes_json,-bm25(rag_graph_nodes_fts) AS score \
                    FROM rag_graph_nodes_fts JOIN rag_graph_node_projections p ON p.id=rag_graph_nodes_fts.rowid \
                    WHERE rag_graph_nodes_fts MATCH REPLACE(jieba_query(?2,0),' AND ',' OR ') \
                    AND p.id IN ({eligible})), ranked AS ( \
                    SELECT *,row_number() OVER(PARTITION BY entity_id ORDER BY score DESC,generation_id) AS position FROM candidates) \
                    SELECT entity_id,generation_id,chunk_codes_json,score,0 FROM ranked WHERE position=1 \
                    ORDER BY score DESC,entity_id LIMIT ?3"
                ),
                vec![
                    rusqlite::types::Value::Text(generations),
                    text.into(),
                    ((query.limit + 1) as i64).into(),
                ],
                false,
            )
        }
        GraphNodeMatch::Vector {
            embedding,
            model_code,
            model_fingerprint,
            provider_model,
        } => {
            if embedding.is_empty()
                || embedding.iter().any(|v| !v.is_finite())
                || model_code.trim().is_empty()
                || model_fingerprint.trim().is_empty()
                || embedding.iter().all(|v| *v == 0.0)
            {
                return Err(err("invalid graph query vector or model space"));
            }
            let dimension = embedding.len();
            let table = vector_table(dimension)?;
            let eligible = format!(
                "{eligible} AND g.model_code=?3 AND g.model_fingerprint=?4 \
                AND g.provider_model IS ?5 AND g.dimension=?6"
            );
            // vec0's rowid IN restriction runs before KNN; a private or differently
            // configured near-neighbour cannot displace an authorized candidate.
            (
                format!(
                    "WITH nearest AS MATERIALIZED ( \
                    SELECT rowid,distance FROM {table} WHERE embedding MATCH ?2 AND k={KNN_CANDIDATE_LIMIT} \
                    AND rowid IN ({eligible})), ranked AS ( \
                    SELECT p.entity_id,p.generation_id,p.chunk_codes_json,1.0-n.distance AS score, \
                    row_number() OVER(PARTITION BY p.entity_id ORDER BY n.distance,p.generation_id) AS position \
                    FROM nearest n JOIN rag_graph_node_projections p ON p.id=n.rowid) \
                    SELECT entity_id,generation_id,chunk_codes_json,score,(SELECT count(*) FROM nearest) \
                    FROM ranked WHERE position=1 ORDER BY score DESC,entity_id LIMIT ?7"
                ),
                vec![
                    generations.into(),
                    serde_json::to_string(&embedding).map_err(err)?.into(),
                    model_code.into(),
                    model_fingerprint.into(),
                    provider_model
                        .map(rusqlite::types::Value::Text)
                        .unwrap_or(rusqlite::types::Value::Null),
                    (dimension as i64).into(),
                    ((query.limit + 1) as i64).into(),
                ],
                true,
            )
        }
    };
    let mut statement = connection.prepare(&sql).map_err(err)?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(parameters), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, f64>(3)?,
                r.get::<_, usize>(4)?,
            ))
        })
        .map_err(err)?;
    let mut result = GraphNodeSearchPage::default();
    for row in rows {
        let (entity_id, generation_id, chunks, score, count) = row.map_err(err)?;
        if !score.is_finite() {
            return Err(err("nonfinite node recall score"));
        }
        result.candidate_limit_reached |= vector && count >= KNN_CANDIDATE_LIMIT;
        result.hits.push(GraphNodeSearchHit {
            entity_id,
            generation_id,
            chunk_codes: serde_json::from_str(&chunks).map_err(err)?,
            score,
        });
    }
    result.has_more = result.hits.len() > query.limit || result.candidate_limit_reached;
    result.hits.truncate(query.limit);
    Ok(result)
}
