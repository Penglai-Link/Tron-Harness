use super::SqliteRagRepositorySql;
use std::collections::{BTreeMap, BTreeSet};

use rusqlite::params;

use crate::{
    application::rag::{
        RagConfigRepository, RagIngestRepository, RagIngestWarning, RagPersistIngestCommand,
        RagTextUnitWrite, RagTreeNodeWrite,
    },
    domain::rag::{RagCorpusRecord, RagError},
};

use super::{
    SqliteRagRepository,
    indexes::{
        decode_embedding, delete_text_unit_indexes_for_source, fts_table_name,
        sync_text_unit_indexes_for_units, table_exists, vec_table_name,
    },
    ingest_values::{
        execute_upsert, json_string, now_millis, push_available_domain_values, push_domain_values,
        push_dynamic_text, push_embedded_source_values, push_integer, push_optional_blob,
        push_optional_text, push_text, validate_domain_field_targets,
    },
    sql_plan::{SqliteRagCorpusPlan, compile_sql_corpus_plan, quote_identifier},
    storage_error,
};

#[derive(Debug, Clone)]
struct SqliteSourceOnlyPlan {
    source_table: String,
    source_columns: BTreeSet<String>,
    field_mapping: BTreeMap<String, String>,
    source_field_mapping: BTreeMap<String, String>,
    source_status_column: String,
    source_content_hash_column: Option<String>,
    source_metadata_column: Option<String>,
    source_created_at_column: Option<String>,
    source_updated_at_column: Option<String>,
}

impl SqliteSourceOnlyPlan {
    fn compile(
        corpus: &RagCorpusRecord,
        source_table: String,
        source_columns: BTreeSet<String>,
    ) -> Result<Self, RagError> {
        Ok(Self {
            source_table,
            source_status_column: first_existing_source_column(
                &source_columns,
                &["source_status", "status"],
            )
            .ok_or(RagError::StorageBindingInvalid)?,
            source_content_hash_column: first_existing_source_column(
                &source_columns,
                &["source_content_hash", "content_hash"],
            ),
            source_metadata_column: first_existing_source_column(
                &source_columns,
                &["source_metadata", "metadata"],
            ),
            source_created_at_column: first_existing_source_column(
                &source_columns,
                &["source_created_at", "created_at"],
            ),
            source_updated_at_column: first_existing_source_column(
                &source_columns,
                &["source_updated_at", "updated_at"],
            ),
            source_columns,
            field_mapping: corpus.field_mapping.clone(),
            source_field_mapping: corpus.source_field_mapping.clone(),
        })
    }

    fn domain_mapping(&self) -> &BTreeMap<String, String> {
        if self.source_field_mapping.is_empty() {
            &self.field_mapping
        } else {
            &self.source_field_mapping
        }
    }
}

impl<'a> SqliteRagRepositorySql<'a> {
    pub(crate) fn persist_source_only(
        &self,
        corpus: &RagCorpusRecord,
        command: &RagPersistIngestCommand,
    ) -> Result<(), RagError> {
        self.ensure_sqlite_runtime_storage_for_corpus(corpus)?;
        let Some(source_table) = corpus.storage_binding.sqlite_table_for_segment("source") else {
            return Ok(());
        };
        let plan = SqliteSourceOnlyPlan::compile(
            corpus,
            source_table.to_string(),
            self.table_columns(source_table)?,
        )?;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&self
            .connection, rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        persist_source_only_row(&tx, &plan, command)?;
        tx.commit().map_err(storage_error)
    }
}

impl RagIngestRepository for SqliteRagRepositorySql<'_> {
    fn persist_ingest(&self, command: RagPersistIngestCommand) -> Result<(), RagError> {
        let corpus = self
            .get_corpus(&command.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        let plan = compile_sql_corpus_plan(self, &corpus)?;
        validate_domain_field_targets(&plan, &command)?;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&self
            .connection, rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        if let Some(task_id) = command
            .metadata
            .get("indexTaskId")
            .and_then(serde_json::Value::as_str)
        {
            let running: i64 = tx.query_row("SELECT COUNT(*) FROM rag_record_tasks current WHERE id=?1 AND state='RUNNING' AND user_code=?2 AND NOT EXISTS (SELECT 1 FROM rag_record_tasks newer WHERE newer.corpus_id=current.corpus_id AND newer.rowid>current.rowid AND json_extract(newer.payload_json,'$.sourceCode')=json_extract(current.payload_json,'$.sourceCode') AND json_extract(newer.payload_json,'$.scopeType')=json_extract(current.payload_json,'$.scopeType') AND json_extract(newer.payload_json,'$.scopeCode')=json_extract(current.payload_json,'$.scopeCode'))",params![task_id,command.user_code],|row|row.get(0)).map_err(storage_error)?;
            if running != 1 {
                return Err(RagError::IngestRequestInvalid);
            }
        }
        delete_text_unit_indexes_for_source(
            &tx,
            &plan,
            &command.source_code,
            &command.source_version,
            &command.scope.scope_type,
            &command.scope.scope_code,
        )?;
        persist_source(&tx, &plan, &command)?;
        persist_text_units(&tx, &plan, &command)?;
        persist_fulltext_units(&tx, &plan, &command)?;
        persist_vector_units(&tx, &plan, &command)?;
        sync_text_unit_indexes_for_units(&tx, &plan, &command.text_units)?;
        persist_tree_nodes(&tx, &plan, &command)?;
        persist_warnings(&tx, &plan, &command)?;
        tx.execute("DELETE FROM rag_graph_evidence WHERE corpus_id=?1 AND source_code=?2 AND source_version=?3 AND scope_type=?4 AND scope_code=?5", params![command.corpus_id,command.source_code,command.source_version,command.scope.scope_type,command.scope.scope_code]).map_err(storage_error)?;
        if let Some(graph) = &command.related_graph {
            // Every derived entity/relation is bound to its original chunk identity.
            for unit in &command.text_units {
                let Some(evidence) = graph.get(&unit.text_unit_code) else {
                    continue;
                };
                let graph_text =
                    serde_json::to_string(evidence).map_err(|_| RagError::StorageFailure)?;
                tx.execute("INSERT INTO rag_graph_evidence(corpus_id,source_code,source_version,scope_type,scope_code,target_code,text,metadata) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",params![command.corpus_id,command.source_code,command.source_version,command.scope.scope_type,command.scope.scope_code,unit.text_unit_code,graph_text,graph_text]).map_err(storage_error)?;
            }
        }
        if let Some(task_id) = command
            .metadata
            .get("indexTaskId")
            .and_then(serde_json::Value::as_str)
        {
            tx.execute(
                "UPDATE rag_record_tasks SET state='COMPLETED' WHERE id=?1 AND state='RUNNING'",
                [task_id],
            )
            .map_err(storage_error)?;
        }
        tx.commit().map_err(storage_error)
    }
}

fn persist_source(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
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
    push_optional_text(
        &mut values,
        &plan.source_columns,
        "body_hash",
        command.body_hash.as_deref(),
    );
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
    let mapping = if plan.source_field_mapping.is_empty() {
        &plan.field_mapping
    } else {
        &plan.source_field_mapping
    };
    push_domain_values(&mut values, &plan.source_columns, mapping, command)?;
    execute_upsert(
        tx,
        table,
        values,
        &["source_code", "source_version", "scope_type", "scope_code"],
        &[plan.source_created_at_column.as_deref()],
    )?;
    Ok(())
}

fn persist_source_only_row(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    plan: &SqliteSourceOnlyPlan,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
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
    push_optional_text(
        &mut values,
        &plan.source_columns,
        "body_hash",
        command.body_hash.as_deref(),
    );
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
    push_available_domain_values(
        &mut values,
        &plan.source_columns,
        plan.domain_mapping(),
        command,
    )?;
    execute_upsert(
        tx,
        &plan.source_table,
        values,
        &["source_code", "source_version", "scope_type", "scope_code"],
        &[plan.source_created_at_column.as_deref()],
    )?;
    Ok(())
}

fn persist_text_units(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    plan: &SqliteRagCorpusPlan,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    if !plan.has_sqlite_text_units() {
        return Ok(());
    }
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

fn first_existing_source_column(columns: &BTreeSet<String>, candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .find(|candidate| columns.contains(**candidate))
        .map(|candidate| (*candidate).to_string())
}

fn persist_text_unit(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
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
    if plan.embedded_source {
        push_embedded_source_values(&mut values, plan, command, now)?;
    }
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
    push_index_values(&mut values, &plan.text_unit_columns, unit);
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
    push_domain_values(
        &mut values,
        &plan.text_unit_columns,
        &plan.text_unit_field_mapping,
        command,
    )?;
    execute_upsert(
        tx,
        &plan.text_unit_table,
        values,
        &plan
            .text_unit_unique_key
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        &[
            plan.text_unit_created_at_column.as_deref(),
            plan.source_created_at_column.as_deref(),
        ],
    )?;
    Ok(())
}

fn persist_vector_units(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    plan: &SqliteRagCorpusPlan,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    if !plan.stores_vector_carrier_rows() {
        return Ok(());
    }
    let now = now_millis();
    for unit in &command.text_units {
        let mut values = Vec::new();
        push_carrier_text_unit_values(&mut values, &plan.vector_columns, plan, command, unit, now)?;
        execute_upsert(
            tx,
            plan.vector_storage_table(),
            values,
            &plan
                .vector_unique_key
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            &[Some("created_at")],
        )?;
    }
    Ok(())
}

impl SqliteRagRepository {
    pub(crate) fn persist_source_only(
        &self,
        corpus: &RagCorpusRecord,
        command: &RagPersistIngestCommand,
    ) -> Result<(), RagError> {
        let corpus = corpus.to_owned();
        let command = command.to_owned();
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.persist_source_only(&corpus, &command)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
impl RagIngestRepository for SqliteRagRepository {
    fn persist_ingest(&self, command: RagPersistIngestCommand) -> Result<(), RagError> {
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.persist_ingest(command),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
