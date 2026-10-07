use super::SqliteRagRepositorySql;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, params};
use serde_json::Value;

use crate::{
    application::rag::{
        RagConfigRepository, RagRetryIndexRepository, RagRetryIndexRequest, RagRetryTextUnit,
        RagRetryVectorUpdate,
    },
    domain::rag::RagError,
};

use super::{
    SqliteRagRepository,
    indexes::{
        fts_table_name, rebuild_fulltext_indexes_for_source, table_exists,
        upsert_vector_index_for_code,
    },
    sql_plan::{compile_sql_corpus_plan, quote_identifier},
    storage_error,
};

impl RagRetryIndexRepository for SqliteRagRepositorySql<'_> {
    fn retry_text_units(
        &self,
        request: &RagRetryIndexRequest,
    ) -> Result<Vec<RagRetryTextUnit>, RagError> {
        let plan = retry_plan(self, &request.corpus_id)?;
        let table = quote_identifier(&plan.text_unit_table)?;
        let (fulltext_join, has_fulltext_carrier_join, has_fts_join) =
            retry_fulltext_join(&self.connection, &plan)?;
        let index_text_expr =
            retry_index_text_expr(&plan, has_fulltext_carrier_join, has_fts_join)?;
        let sql = format!(
            "SELECT tu.text_unit_code, tu.text, {index_text_expr}
             FROM {table} tu
             {fulltext_join}
             WHERE tu.source_code = ?1 AND tu.source_version = ?2
               AND tu.scope_type = ?3 AND tu.scope_code = ?4
               AND tu.{} = 'ACTIVE'
             ORDER BY tu.ordinal",
            quote_identifier(&plan.text_unit_status_column)?
        );
        let mut statement = self.connection.prepare(&sql).map_err(storage_error)?;
        let rows = statement
            .query_map(
                params![
                    request.source_code,
                    request.source_version,
                    request.scope_type,
                    request.scope_code
                ],
                |row| {
                    Ok(RagRetryTextUnit {
                        text_unit_code: row.get(0)?,
                        text: row.get(1)?,
                        index_text: row.get(2)?,
                    })
                },
            )
            .map_err(storage_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
    }

    fn rebuild_fulltext_index(&self, request: &RagRetryIndexRequest) -> Result<usize, RagError> {
        let plan = retry_plan(self, &request.corpus_id)?;
        let table = quote_identifier(plan.fulltext_storage_table())?;
        let updated_at_column = if plan.has_distinct_fulltext_table() {
            plan.fulltext_updated_at_column.as_deref()
        } else {
            plan.text_unit_updated_at_column.as_deref()
        }
        .ok_or(RagError::StorageBindingInvalid)?;
        let status_column = if plan.has_distinct_fulltext_table() {
            "status"
        } else {
            &plan.text_unit_status_column
        };
        let sql = format!(
            "UPDATE {table}
             SET {} = ?1
             WHERE source_code = ?2 AND source_version = ?3 AND scope_type = ?4 AND scope_code = ?5
               AND {} = 'ACTIVE'",
            quote_identifier(updated_at_column)?,
            quote_identifier(status_column)?
        );
        let updated = self
            .connection
            .execute(
                &sql,
                params![
                    now_millis(),
                    request.source_code,
                    request.source_version,
                    request.scope_type,
                    request.scope_code
                ],
            )
            .map_err(storage_error)?;
        rebuild_fulltext_indexes_for_source(
            &self.connection,
            &plan,
            &request.source_code,
            &request.source_version,
            &request.scope_type,
            &request.scope_code,
        )?;
        Ok(updated)
    }

    fn update_vector_index(&self, update: RagRetryVectorUpdate) -> Result<(), RagError> {
        let plan = retry_plan(self, &update.corpus_id)?;
        let table = quote_identifier(plan.vector_storage_table())?;
        let updated_at_column = plan
            .vector_updated_at_column
            .as_deref()
            .ok_or(RagError::StorageBindingInvalid)?;
        let sql = format!(
            "UPDATE {table}
             SET {} = ?1
             WHERE text_unit_code = ?2",
            quote_identifier(updated_at_column)?
        );
        self.connection
            .execute(&sql, params![now_millis(), update.text_unit_code])
            .map_err(storage_error)?;
        upsert_vector_index_for_code(
            &self.connection,
            &plan,
            &update.text_unit_code,
            update.embedding_dimension,
            &update.embedding,
        )?;
        Ok(())
    }

    fn mark_vector_retry_failed(&self, request: &RagRetryIndexRequest) -> Result<(), RagError> {
        let plan = retry_plan(self, &request.corpus_id)?;
        let table = quote_identifier(plan.vector_storage_table())?;
        let updated_at_column = plan
            .vector_updated_at_column
            .as_deref()
            .ok_or(RagError::StorageBindingInvalid)?;
        let sql = format!(
            "UPDATE {table}
             SET {} = ?1
             WHERE source_code = ?2 AND source_version = ?3 AND scope_type = ?4 AND scope_code = ?5"
            ,
            quote_identifier(updated_at_column)?
        );
        self.connection
            .execute(
                &sql,
                params![
                    now_millis(),
                    request.source_code,
                    request.source_version,
                    request.scope_type,
                    request.scope_code
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn persist_retry_warning(
        &self,
        request: &RagRetryIndexRequest,
        stage_id: &str,
        code: &str,
        message: &str,
    ) -> Result<(), RagError> {
        let plan = retry_plan(self, &request.corpus_id)?;
        let diagnostics_table = quote_identifier(
            plan.diagnostics_table
                .as_deref()
                .ok_or(RagError::StorageBindingInvalid)?,
        )?;
        let now = now_millis();
        let record_code = format!(
            "{}:{}:{}:{}:{}",
            request.corpus_id,
            request.source_code,
            request.source_version,
            request.scope_code,
            stage_id
        );
        let sql = format!(
            "INSERT INTO {diagnostics_table} (
                record_type, record_code, corpus_id, source_code, source_version,
                scope_type, scope_code, workspace_code, user_code, agent_code, stage_id,
                message, payload_json, created_at
            )
            VALUES ('INDEXING_WARNING', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT (record_code) DO UPDATE SET
                message = excluded.message,
                payload_json = excluded.payload_json,
                created_at = excluded.created_at"
        );
        self.connection
            .execute(
                &sql,
                params![
                    record_code,
                    request.corpus_id,
                    request.source_code,
                    request.source_version,
                    request.scope_type,
                    request.scope_code,
                    request.workspace_code,
                    request.user_code,
                    request.agent_code,
                    stage_id,
                    message,
                    json_string(&serde_json::json!({ "code": code, "stageId": stage_id }))?,
                    now,
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

fn retry_plan(
    repository: &SqliteRagRepositorySql<'_>,
    corpus_id: &str,
) -> Result<super::sql_plan::SqliteRagCorpusPlan, RagError> {
    let corpus = repository
        .get_corpus(corpus_id)?
        .ok_or(RagError::CorpusNotFound)?;
    compile_sql_corpus_plan(repository, &corpus)
}

fn retry_fulltext_join(
    connection: &Connection,
    plan: &super::sql_plan::SqliteRagCorpusPlan,
) -> Result<(String, bool, bool), RagError> {
    if !plan.has_sqlite_fulltext() {
        return Ok((String::new(), false, false));
    }
    let fts_table_name = fts_table_name(plan.fulltext_storage_table());
    let has_fts_table = table_exists(connection, &fts_table_name)?;
    if plan.has_distinct_fulltext_table() {
        if !table_exists(connection, plan.fulltext_storage_table())? {
            return Ok((String::new(), false, false));
        }
        let fulltext_table = quote_identifier(plan.fulltext_storage_table())?;
        let mut join =
            format!("LEFT JOIN {fulltext_table} ft ON ft.text_unit_code = tu.text_unit_code");
        if has_fts_table {
            join.push_str(&format!(
                "\n             LEFT JOIN {} fts ON fts.rowid = ft.id",
                quote_identifier(&fts_table_name)?
            ));
        }
        return Ok((join, true, has_fts_table));
    }
    if has_fts_table {
        return Ok((
            format!(
                "LEFT JOIN {} fts ON fts.rowid = tu.id",
                quote_identifier(&fts_table_name)?
            ),
            false,
            true,
        ));
    }
    Ok((String::new(), false, false))
}

fn retry_index_text_expr(
    plan: &super::sql_plan::SqliteRagCorpusPlan,
    has_fulltext_carrier_join: bool,
    has_fts_join: bool,
) -> Result<String, RagError> {
    // Search/index text may live in a carrier column, the FTS sidecar,
    // or only as raw text in legacy layouts.
    let mut candidates = Vec::new();
    if has_fulltext_carrier_join && plan.fulltext_columns.contains("search_text") {
        candidates.push(format!("ft.{}", quote_identifier("search_text")?));
    }
    if plan.text_unit_columns.contains("search_text") {
        candidates.push(format!("tu.{}", quote_identifier("search_text")?));
    }
    if has_fts_join {
        candidates.push(format!("fts.{}", quote_identifier("search_text")?));
    }
    if has_fulltext_carrier_join && plan.fulltext_columns.contains("text") {
        candidates.push(format!("ft.{}", quote_identifier("text")?));
    }
    candidates.push(format!("tu.{}", quote_identifier("text")?));
    Ok(format!("COALESCE({})", candidates.join(", ")))
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

impl RagRetryIndexRepository for SqliteRagRepository {
    fn retry_text_units(
        &self,
        request: &RagRetryIndexRequest,
    ) -> Result<Vec<RagRetryTextUnit>, RagError> {
        let request = request.to_owned();
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.retry_text_units(&request),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn rebuild_fulltext_index(&self, request: &RagRetryIndexRequest) -> Result<usize, RagError> {
        let request = request.to_owned();
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.rebuild_fulltext_index(&request)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn update_vector_index(&self, update: RagRetryVectorUpdate) -> Result<(), RagError> {
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.update_vector_index(update),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn mark_vector_retry_failed(&self, request: &RagRetryIndexRequest) -> Result<(), RagError> {
        let request = request.to_owned();
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.mark_vector_retry_failed(&request)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn persist_retry_warning(
        &self,
        request: &RagRetryIndexRequest,
        stage_id: &str,
        code: &str,
        message: &str,
    ) -> Result<(), RagError> {
        let request = request.to_owned();
        let stage_id = stage_id.to_owned();
        let code = code.to_owned();
        let message = message.to_owned();
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }
                    .persist_retry_warning(&request, &stage_id, &code, &message)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
