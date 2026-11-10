use super::SqliteRagRepositorySql;
use std::collections::BTreeSet;

use rusqlite::params;

use crate::{
    application::rag::{
        RagIngestRepository, RagIngestWarning, RagPersistIngestCommand,
        RagTextUnitWrite, RagTreeNodeWrite,
    },
    domain::rag::RagError,
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

impl RagIngestRepository for SqliteRagRepositorySql<'_> {
    fn cached_vectors(
        &self,
        corpus: &str,
        source: &str,
        workspace: &str,
        fingerprint: &str,
        dimension: usize,
    ) -> Result<std::collections::BTreeMap<String, Vec<f32>>, RagError> {
        let corpus = self.get_corpus(corpus)?.ok_or(RagError::CorpusNotFound)?;
        let plan = compile_sql_corpus_plan(self, &corpus)?;
        // Other storage implementations may provide their own cache. Never guess a binding.
        if plan.has_distinct_vector_table()
            || plan.has_distinct_source_table()
            || !plan.has_sqlite_vector()
        {
            return Ok(Default::default());
        }
        let table = quote_identifier(&plan.text_unit_table)?;
        let Some(metadata) = plan.source_metadata_column.as_deref() else {
            return Ok(Default::default());
        };
        let metadata = quote_identifier(metadata)?;
        let status = quote_identifier(&plan.source_status_column)?;
        let sql = format!(
            "SELECT embedding_text_hash,embedding FROM {table} WHERE source_code=?1 AND scope_type='SPACE' AND scope_code=?2 AND {status}='ACTIVE' AND embedding_status='INDEXED' AND embedding_dimension=?3 AND json_extract({metadata},'$.embeddingFingerprint')=?4 AND coalesce(json_extract({metadata},'$.embeddingCacheInvalidated'),0)=0"
        );
        let mut statement = self.connection.prepare(&sql).map_err(storage_error)?;
        let rows = statement
            .query_map(
                params![source, workspace, dimension as i64, fingerprint],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<Vec<u8>>>(1)?,
                    ))
                },
            )
            .map_err(storage_error)?;
        let mut result = std::collections::BTreeMap::new();
        for row in rows {
            let (Some(key), Some(bytes)) = row.map_err(storage_error)? else {
                continue;
            };
            if let Some(vector) = decode_embedding(&bytes)
                .filter(|v| v.len() == dimension && v.iter().all(|n| n.is_finite()))
            {
                result.insert(key, vector);
            }
        }
        Ok(result)
    }
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
            let capabilities:String=tx.query_row("SELECT capabilities_json FROM rag_corpora WHERE corpus_id=?1",[&command.corpus_id],|r|r.get(0)).map_err(storage_error)?;
            let current=crate::application::rag::RagSettings::from_capabilities(&serde_json::from_str(&capabilities).map_err(|_|RagError::StorageFailure)?)?;
            let expected:crate::application::rag::RagSettings=serde_json::from_value(command.metadata["indexSettings"].clone()).map_err(|_|RagError::IngestRequestInvalid)?;
            if current!=expected{return Err(RagError::IngestRequestInvalid);}
            let running: i64 = tx.query_row("SELECT COUNT(*) FROM rag_record_tasks current WHERE id=?1 AND state='RUNNING' AND user_code=?2 AND NOT EXISTS (SELECT 1 FROM rag_record_tasks newer WHERE newer.corpus_id=current.corpus_id AND newer.rowid>current.rowid AND json_extract(newer.payload_json,'$.sourceCode')=json_extract(current.payload_json,'$.sourceCode') AND json_extract(newer.payload_json,'$.scopeType')=json_extract(current.payload_json,'$.scopeType') AND json_extract(newer.payload_json,'$.scopeCode')=json_extract(current.payload_json,'$.scopeCode'))",params![task_id,command.user_code],|row|row.get(0)).map_err(storage_error)?;
            if running != 1 {
                return Err(RagError::IngestRequestInvalid);
            }
        }
        if command.corpus_id == "memory-card" {
            let valid: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM memory_records WHERE memory_code=?1 AND user_code=?2 AND enabled=1 AND version=?3 AND (expires_at IS NULL OR expires_at>?4) AND scope_type=?5 AND scope_code=?6 AND text=(SELECT json_extract(payload_json,'$.payload.text') FROM rag_record_tasks WHERE id=?7))",params![command.source_code,command.user_code,command.metadata["recordVersion"].as_u64(),chrono::Utc::now().timestamp_millis(),command.scope.scope_type,command.scope.scope_code,command.task_id],|r|r.get(0)).map_err(storage_error)?;
            if !valid {
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
        if command.source_status == "ACTIVE" {
            let partition=crate::domain::rag::graph::GraphPartition{corpus_id:command.corpus_id.clone(),scope_type:command.scope.scope_type.clone(),scope_code:command.scope.scope_code.clone()};
            let logical=command.metadata["logicalSourceCode"].as_str().unwrap_or(&command.source_code);
            if command.metadata["indexSettings"]["graph"]==true {
                let id=command.metadata["graphGenerationId"].as_str().ok_or(RagError::IngestRequestInvalid)?;
                let config=command.metadata["graphConfigurationHash"].as_str().ok_or(RagError::IngestRequestInvalid)?;
                let graphs=&crate::adapters::outbound::persistence::sqlite::rag_graph_index::validate_graph_generation_binding;
                graphs(&tx,id,&partition,logical,&command.source_code,&command.task_id,config).map_err(|_|RagError::IngestRequestInvalid)?;
                crate::adapters::outbound::persistence::sqlite::rag_graph_index::activate_graph_generation(&tx,id).map_err(|_|RagError::IngestRequestInvalid)?;
            }else{
                crate::adapters::outbound::persistence::sqlite::rag_graph_index::retire_graph_source(&tx,&partition,logical).map_err(|_|RagError::StorageFailure)?;
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

impl RagIngestRepository for SqliteRagRepository {
    fn cached_vectors(
        &self,
        corpus: &str,
        source: &str,
        workspace: &str,
        fingerprint: &str,
        dimension: usize,
    ) -> Result<std::collections::BTreeMap<String, Vec<f32>>, RagError> {
        let (corpus, source, workspace, fingerprint) = (
            corpus.to_owned(),
            source.to_owned(),
            workspace.to_owned(),
            fingerprint.to_owned(),
        );
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.cached_vectors(
                    &corpus,
                    &source,
                    &workspace,
                    &fingerprint,
                    dimension,
                )
            },
            || RagError::StorageFailure,
        )
    }
    fn persist_ingest(&self, command: RagPersistIngestCommand) -> Result<(), RagError> {
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.persist_ingest(command),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
