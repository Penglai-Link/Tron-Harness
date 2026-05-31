use rusqlite::params;
use serde_json::{Value, json};

use crate::{
    application::rag::{RagSearchRepository, RagSearchRow, RagTextUnitSearchQuery},
    domain::rag::RagError,
};

use super::{SqliteRagRepository, storage_error};

impl RagSearchRepository for SqliteRagRepository {
    fn search_text_units(
        &self,
        query: RagTextUnitSearchQuery,
    ) -> Result<Vec<RagSearchRow>, RagError> {
        match query.corpus_id.as_str() {
            "memory-card" => search_memory(self, &query),
            "project-doc" => search_project_doc(self, &query),
            _ => Err(RagError::CorpusReserved),
        }
    }
}

fn search_memory(
    repository: &SqliteRagRepository,
    query: &RagTextUnitSearchQuery,
) -> Result<Vec<RagSearchRow>, RagError> {
    let mut statement = repository
        .connection
        .prepare(
            "SELECT tu.text_unit_code, tu.source_code, tu.source_version, tu.scope_type,
                    tu.scope_code, tu.text, tu.search_text, tu.heading_path, tu.ordinal,
                    tu.token_count, tu.embedding, src.title, src.metadata, tu.metadata,
                    src.memory_type, src.memory_status, src.review_status, src.version,
                    src.injectable, src.importance_score
             FROM rag_memory_text_units tu
             JOIN rag_memory_sources src
               ON src.source_code = tu.source_code
              AND src.source_version = tu.source_version
              AND src.scope_type = tu.scope_type
              AND src.scope_code = tu.scope_code
             WHERE tu.source_version = ?1
               AND tu.status = 'ACTIVE'
               AND src.status = 'ACTIVE'
               AND tu.fulltext_status = 'INDEXED'
             ORDER BY tu.source_code, tu.ordinal",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![query.source_version], |row| {
            let source_metadata = json!({
                "title": row.get::<_, Option<String>>(11)?,
                "memoryType": row.get::<_, Option<String>>(14)?,
                "memoryStatus": row.get::<_, Option<String>>(15)?,
                "reviewStatus": row.get::<_, Option<String>>(16)?,
                "version": row.get::<_, Option<String>>(17)?,
                "injectable": row.get::<_, i64>(18)? != 0,
                "importanceScore": row.get::<_, Option<i64>>(19)?,
                "payload": parse_json(row.get::<_, String>(12)?.as_str()),
            });
            Ok(RagSearchRow {
                corpus_id: "memory-card".into(),
                source_code: row.get(1)?,
                source_version: row.get(2)?,
                target_code: row.get(0)?,
                scope_type: row.get(3)?,
                scope_code: row.get(4)?,
                text: row.get(5)?,
                search_text: row.get(6)?,
                heading_path: split_heading_path(&row.get::<_, String>(7)?),
                ordinal: row.get(8)?,
                token_count: row.get(9)?,
                embedding: row
                    .get::<_, Option<Vec<u8>>>(10)?
                    .and_then(decode_embedding),
                source_metadata,
                text_metadata: parse_json(row.get::<_, String>(13)?.as_str()),
            })
        })
        .map_err(storage_error)?;
    filter_rows(
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)?,
        query,
    )
}

fn search_project_doc(
    repository: &SqliteRagRepository,
    query: &RagTextUnitSearchQuery,
) -> Result<Vec<RagSearchRow>, RagError> {
    let mut statement = repository
        .connection
        .prepare(
            "SELECT tu.text_unit_code, tu.source_code, tu.source_version, tu.scope_type,
                    tu.scope_code, tu.text, tu.search_text, tu.heading_path, tu.ordinal,
                    tu.token_count, tu.embedding, src.title, src.metadata, tu.metadata,
                    src.document_code, src.document_type, src.path, src.repo_code,
                    src.module_code, src.module_name
             FROM rag_project_doc_text_units tu
             JOIN rag_project_doc_sources src
               ON src.source_code = tu.source_code
              AND src.source_version = tu.source_version
              AND src.scope_type = tu.scope_type
              AND src.scope_code = tu.scope_code
             WHERE tu.source_version = ?1
               AND tu.status = 'ACTIVE'
               AND src.status = 'ACTIVE'
               AND tu.fulltext_status = 'INDEXED'
             ORDER BY tu.source_code, tu.ordinal",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![query.source_version], |row| {
            let heading_path = split_heading_path(&row.get::<_, String>(7)?);
            let source_metadata = json!({
                "title": row.get::<_, Option<String>>(11)?,
                "documentCode": row.get::<_, Option<String>>(14)?,
                "documentType": row.get::<_, Option<String>>(15)?,
                "path": row.get::<_, Option<String>>(16)?,
                "repoCode": row.get::<_, Option<String>>(17)?,
                "moduleCode": row.get::<_, Option<String>>(18)?,
                "moduleName": row.get::<_, Option<String>>(19)?,
                "headingPath": heading_path,
                "payload": parse_json(row.get::<_, String>(12)?.as_str()),
            });
            Ok(RagSearchRow {
                corpus_id: "project-doc".into(),
                source_code: row.get(1)?,
                source_version: row.get(2)?,
                target_code: row.get(0)?,
                scope_type: row.get(3)?,
                scope_code: row.get(4)?,
                text: row.get(5)?,
                search_text: row.get(6)?,
                heading_path,
                ordinal: row.get(8)?,
                token_count: row.get(9)?,
                embedding: row
                    .get::<_, Option<Vec<u8>>>(10)?
                    .and_then(decode_embedding),
                source_metadata,
                text_metadata: parse_json(row.get::<_, String>(13)?.as_str()),
            })
        })
        .map_err(storage_error)?;
    filter_rows(
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)?,
        query,
    )
}

fn filter_rows(
    rows: Vec<RagSearchRow>,
    query: &RagTextUnitSearchQuery,
) -> Result<Vec<RagSearchRow>, RagError> {
    let mut visible = rows
        .into_iter()
        .filter(|row| {
            query.scopes.iter().any(|scope| {
                scope.scope_type == row.scope_type && scope.scope_code == row.scope_code
            })
        })
        .filter(|row| row_matches_filters(row, &query.filters))
        .take(query.limit)
        .collect::<Vec<_>>();
    visible.sort_by(|left, right| {
        left.source_code
            .cmp(&right.source_code)
            .then_with(|| left.ordinal.cmp(&right.ordinal))
    });
    Ok(visible)
}

fn row_matches_filters(row: &RagSearchRow, filters: &Value) -> bool {
    let Some(object) = filters.as_object() else {
        return false;
    };
    object.iter().all(|(field, filter)| {
        let Some(expected) = filter.get("value") else {
            return false;
        };
        let actual = filter_value(row, field);
        if let Some(items) = expected.as_array() {
            items.iter().any(|item| Some(item) == actual.as_ref())
        } else {
            Some(expected) == actual.as_ref()
        }
    })
}

fn filter_value(row: &RagSearchRow, field: &str) -> Option<Value> {
    match field {
        "sourceCode" => Some(json!(row.source_code)),
        "sourceVersion" => Some(json!(row.source_version)),
        "scopeType" => Some(json!(row.scope_type)),
        "scopeCode" => Some(json!(row.scope_code)),
        _ => row.source_metadata.get(field).cloned(),
    }
}

fn split_heading_path(raw: &str) -> Vec<String> {
    raw.split('>')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn decode_embedding(raw: Vec<u8>) -> Option<Vec<f32>> {
    if !raw.len().is_multiple_of(4) {
        return None;
    }
    Some(
        raw.chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect(),
    )
}

fn parse_json(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| json!({}))
}
