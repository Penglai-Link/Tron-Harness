use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Transaction, params, params_from_iter, types::Value as SqlValue};
use serde_json::Value;

use crate::{
    application::rag::{
        RagConfigRepository, RagIngestRepository, RagPersistIngestCommand, RagTextUnitWrite,
    },
    domain::rag::RagError,
};

use super::{
    SqliteRagRepository,
    sql_plan::{SqliteRagCorpusPlan, compile_sql_corpus_plan, quote_identifier},
    storage_error,
};

impl RagIngestRepository for SqliteRagRepository {
    fn persist_ingest(&self, command: RagPersistIngestCommand) -> Result<(), RagError> {
        let corpus = self
            .get_corpus(&command.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        let plan = compile_sql_corpus_plan(self, &corpus)?;
        let tx = self
            .connection
            .unchecked_transaction()
            .map_err(storage_error)?;
        persist_source(&tx, &plan, &command)?;
        persist_text_units(&tx, &plan, &command)?;
        persist_warnings(&tx, &plan, &command)?;
        tx.commit().map_err(storage_error)
    }
}

fn persist_source(
    tx: &Transaction<'_>,
    plan: &SqliteRagCorpusPlan,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    let Some(table) = plan.source_table.as_deref() else {
        return Ok(());
    };
    let now = now_millis();
    let mut values = Vec::new();
    push_text(
        &mut values,
        &plan.source_columns,
        "source_code",
        &command.source_code,
    );
    push_text(
        &mut values,
        &plan.source_columns,
        "source_version",
        &command.source_version,
    );
    push_text(
        &mut values,
        &plan.source_columns,
        "source_type",
        &command.source_type,
    );
    push_text(
        &mut values,
        &plan.source_columns,
        "scope_type",
        &command.scope.scope_type,
    );
    push_text(
        &mut values,
        &plan.source_columns,
        "scope_code",
        &command.scope.scope_code,
    );
    push_dynamic_text(
        &mut values,
        &plan.source_columns,
        &plan.source_status_column,
        &command.source_status,
    );
    push_optional_text(
        &mut values,
        &plan.source_columns,
        "title",
        command.title.as_deref(),
    );
    if let Some(column) = plan.source_content_hash_column.as_deref() {
        push_dynamic_text(
            &mut values,
            &plan.source_columns,
            column,
            &command.content_hash,
        );
    }
    if let Some(column) = plan.source_metadata_column.as_deref() {
        push_dynamic_text(
            &mut values,
            &plan.source_columns,
            column,
            &json_string(&command.metadata)?,
        );
    }
    if let Some(column) = plan.source_created_at_column.as_deref() {
        push_integer(&mut values, &plan.source_columns, column, now);
    }
    if let Some(column) = plan.source_updated_at_column.as_deref() {
        push_integer(&mut values, &plan.source_columns, column, now);
    }
    push_source_domain_values(&mut values, &plan.source_columns, command);
    execute_upsert(
        tx,
        table,
        values,
        &["source_code", "source_version", "scope_type", "scope_code"],
        &[plan.source_created_at_column.as_deref()],
    )?;
    Ok(())
}

fn push_source_domain_values(
    values: &mut Vec<(String, SqlValue)>,
    columns: &std::collections::BTreeSet<String>,
    command: &RagPersistIngestCommand,
) {
    if let Some(memory) = &command.memory {
        push_optional_text(
            values,
            columns,
            "memory_type",
            memory.memory_type.as_deref(),
        );
        push_optional_text(
            values,
            columns,
            "memory_status",
            memory.memory_status.as_deref(),
        );
        push_optional_text(
            values,
            columns,
            "review_status",
            memory.review_status.as_deref(),
        );
        push_optional_text(values, columns, "version", memory.version.as_deref());
        push_optional_integer(values, columns, "importance_score", memory.importance_score);
    }
    if let Some(project_doc) = &command.project_doc {
        push_optional_text(
            values,
            columns,
            "document_code",
            project_doc.document_code.as_deref(),
        );
        push_optional_text(
            values,
            columns,
            "document_type",
            project_doc.document_type.as_deref(),
        );
        push_optional_text(values, columns, "path", project_doc.path.as_deref());
        push_text(values, columns, "body_hash", &project_doc.body_hash);
        push_optional_text(
            values,
            columns,
            "repo_code",
            project_doc.repo_code.as_deref(),
        );
        push_optional_text(
            values,
            columns,
            "module_code",
            project_doc.module_code.as_deref(),
        );
        push_optional_text(
            values,
            columns,
            "module_name",
            project_doc.module_name.as_deref(),
        );
    }
}

fn persist_text_units(
    tx: &Transaction<'_>,
    plan: &SqliteRagCorpusPlan,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    let now = now_millis();
    for unit in &command.text_units {
        persist_text_unit(tx, plan, command, unit, now)?;
    }
    let table = quote_identifier(&plan.text_unit_table)?;
    let delete_sql = format!(
        "DELETE FROM {table}
         WHERE source_code = ?1
           AND source_version = ?2
           AND scope_type = ?3
           AND scope_code = ?4
           AND ordinal >= ?5"
    );
    tx.execute(
        &delete_sql,
        params![
            command.source_code,
            command.source_version,
            command.scope.scope_type,
            command.scope.scope_code,
            command.text_units.len() as i64,
        ],
    )
    .map_err(storage_error)?;
    Ok(())
}

fn persist_text_unit(
    tx: &Transaction<'_>,
    plan: &SqliteRagCorpusPlan,
    command: &RagPersistIngestCommand,
    unit: &RagTextUnitWrite,
    now: i64,
) -> Result<(), RagError> {
    let mut values = Vec::new();
    push_text(
        &mut values,
        &plan.text_unit_columns,
        "text_unit_code",
        &unit.text_unit_code,
    );
    push_text(
        &mut values,
        &plan.text_unit_columns,
        "source_code",
        &command.source_code,
    );
    push_text(
        &mut values,
        &plan.text_unit_columns,
        "source_version",
        &command.source_version,
    );
    push_optional_text(
        &mut values,
        &plan.text_unit_columns,
        "document_code",
        unit.document_code.as_deref(),
    );
    push_text(
        &mut values,
        &plan.text_unit_columns,
        "scope_type",
        &command.scope.scope_type,
    );
    push_text(
        &mut values,
        &plan.text_unit_columns,
        "scope_code",
        &command.scope.scope_code,
    );
    push_text(
        &mut values,
        &plan.text_unit_columns,
        "unit_type",
        &unit.unit_type,
    );
    push_text(&mut values, &plan.text_unit_columns, "text", &unit.text);
    push_text(
        &mut values,
        &plan.text_unit_columns,
        "search_text",
        &unit.search_text,
    );
    push_text(
        &mut values,
        &plan.text_unit_columns,
        "heading_path",
        &unit.heading_path,
    );
    push_integer(
        &mut values,
        &plan.text_unit_columns,
        "ordinal",
        unit.ordinal as i64,
    );
    push_integer(
        &mut values,
        &plan.text_unit_columns,
        "token_count",
        unit.token_count,
    );
    if let Some(column) = plan.text_unit_content_hash_column.as_deref() {
        push_dynamic_text(
            &mut values,
            &plan.text_unit_columns,
            column,
            &unit.content_hash,
        );
    }
    push_dynamic_text(
        &mut values,
        &plan.text_unit_columns,
        &plan.text_unit_status_column,
        &unit.status,
    );
    push_text(
        &mut values,
        &plan.text_unit_columns,
        "fulltext_status",
        &unit.fulltext_status,
    );
    push_optional_text(
        &mut values,
        &plan.text_unit_columns,
        "embedding_model_code",
        unit.embedding_model_code.as_deref(),
    );
    push_optional_text(
        &mut values,
        &plan.text_unit_columns,
        "embedding_version",
        unit.embedding_version.as_deref(),
    );
    push_optional_text(
        &mut values,
        &plan.text_unit_columns,
        "embedding_text_hash",
        unit.embedding_text_hash.as_deref(),
    );
    push_optional_integer(
        &mut values,
        &plan.text_unit_columns,
        "embedding_dimension",
        unit.embedding_dimension,
    );
    push_optional_blob(
        &mut values,
        &plan.text_unit_columns,
        "embedding",
        unit.embedding.as_deref(),
    );
    push_text(
        &mut values,
        &plan.text_unit_columns,
        "embedding_status",
        &unit.embedding_status,
    );
    if let Some(column) = plan.text_unit_metadata_column.as_deref() {
        push_dynamic_text(
            &mut values,
            &plan.text_unit_columns,
            column,
            &json_string(&unit.metadata)?,
        );
    }
    if let Some(column) = plan.text_unit_created_at_column.as_deref() {
        push_integer(&mut values, &plan.text_unit_columns, column, now);
    }
    if let Some(column) = plan.text_unit_updated_at_column.as_deref() {
        push_integer(&mut values, &plan.text_unit_columns, column, now);
    }
    execute_upsert(
        tx,
        &plan.text_unit_table,
        values,
        &plan
            .text_unit_unique_key
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        &[plan.text_unit_created_at_column.as_deref()],
    )?;
    Ok(())
}

fn persist_warnings(
    tx: &Transaction<'_>,
    plan: &SqliteRagCorpusPlan,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    let diagnostics_table = plan
        .diagnostics_table
        .as_deref()
        .ok_or(RagError::StorageBindingInvalid)?;
    let diagnostics_table = quote_identifier(diagnostics_table)?;
    let now = now_millis();
    for warning in &command.warnings {
        let record_code = format!(
            "{}:{}:{}:{}:{}:{}",
            command.corpus_id,
            command.profile_id,
            command.source_code,
            command.source_version,
            command.scope.scope_type,
            warning.stage_id
        );
        let sql = format!(
            "INSERT INTO {diagnostics_table} (
                record_type, record_code, corpus_id, profile_id, source_code, source_version,
                scope_type, scope_code, workspace_code, user_code, agent_code, stage_id,
                message, payload_json, created_at
            )
            VALUES ('INDEXING_WARNING', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            ON CONFLICT (record_code) DO UPDATE SET
                message = excluded.message,
                payload_json = excluded.payload_json,
                created_at = excluded.created_at"
        );
        tx.execute(
            &sql,
            params![
                record_code,
                command.corpus_id,
                command.profile_id,
                command.source_code,
                command.source_version,
                command.scope.scope_type,
                command.scope.scope_code,
                command.workspace_code,
                command.user_code,
                command.agent_code,
                warning.stage_id,
                warning.message,
                json_string(&serde_json::json!({ "code": warning.code }))?,
                now,
            ],
        )
        .map_err(storage_error)?;
    }
    Ok(())
}

fn execute_upsert(
    tx: &Transaction<'_>,
    table_name: &str,
    values: Vec<(String, SqlValue)>,
    conflict_columns: &[&str],
    skip_update_columns: &[Option<&str>],
) -> Result<(), RagError> {
    if values.is_empty() {
        return Err(RagError::StorageBindingInvalid);
    }
    let table = quote_identifier(table_name)?;
    let columns = values
        .iter()
        .map(|(column, _)| quote_identifier(column))
        .collect::<Result<Vec<_>, _>>()?;
    let placeholders = (1..=values.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>();
    let conflict = conflict_columns
        .iter()
        .map(|column| quote_identifier(column))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    let skipped = skip_update_columns
        .iter()
        .flatten()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let update_columns = values
        .iter()
        .map(|(column, _)| column.as_str())
        .filter(|column| !conflict_columns.contains(column) && !skipped.contains(column))
        .collect::<Vec<_>>();
    let update_sql = if update_columns.is_empty() {
        "DO NOTHING".to_string()
    } else {
        format!(
            "DO UPDATE SET {}",
            update_columns
                .iter()
                .map(|column| {
                    let column = quote_identifier(column)?;
                    Ok(format!("{column} = excluded.{column}"))
                })
                .collect::<Result<Vec<_>, RagError>>()?
                .join(", ")
        )
    };
    let sql = format!(
        "INSERT INTO {table} ({}) VALUES ({}) ON CONFLICT ({conflict}) {update_sql}",
        columns.join(", "),
        placeholders.join(", ")
    );
    tx.execute(
        &sql,
        params_from_iter(values.into_iter().map(|(_, value)| value)),
    )
    .map_err(storage_error)?;
    Ok(())
}

fn push_text(
    values: &mut Vec<(String, SqlValue)>,
    columns: &std::collections::BTreeSet<String>,
    column: &str,
    value: &str,
) {
    if columns.contains(column) {
        values.push((column.to_string(), SqlValue::Text(value.to_string())));
    }
}

fn push_dynamic_text(
    values: &mut Vec<(String, SqlValue)>,
    columns: &std::collections::BTreeSet<String>,
    column: &str,
    value: &str,
) {
    push_text(values, columns, column, value);
}

fn push_optional_text(
    values: &mut Vec<(String, SqlValue)>,
    columns: &std::collections::BTreeSet<String>,
    column: &str,
    value: Option<&str>,
) {
    if columns.contains(column) {
        values.push((
            column.to_string(),
            value
                .map(|value| SqlValue::Text(value.to_string()))
                .unwrap_or(SqlValue::Null),
        ));
    }
}

fn push_integer(
    values: &mut Vec<(String, SqlValue)>,
    columns: &std::collections::BTreeSet<String>,
    column: &str,
    value: i64,
) {
    if columns.contains(column) {
        values.push((column.to_string(), SqlValue::Integer(value)));
    }
}

fn push_optional_integer(
    values: &mut Vec<(String, SqlValue)>,
    columns: &std::collections::BTreeSet<String>,
    column: &str,
    value: Option<i64>,
) {
    if columns.contains(column) {
        values.push((
            column.to_string(),
            value.map(SqlValue::Integer).unwrap_or(SqlValue::Null),
        ));
    }
}

fn push_optional_blob(
    values: &mut Vec<(String, SqlValue)>,
    columns: &std::collections::BTreeSet<String>,
    column: &str,
    value: Option<&[u8]>,
) {
    if columns.contains(column) {
        values.push((
            column.to_string(),
            value
                .map(|value| SqlValue::Blob(value.to_vec()))
                .unwrap_or(SqlValue::Null),
        ));
    }
}

fn json_string(value: &Value) -> Result<String, RagError> {
    serde_json::to_string(value).map_err(|_| RagError::StorageFailure)
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
