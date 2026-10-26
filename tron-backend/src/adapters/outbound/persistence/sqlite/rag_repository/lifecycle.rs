use super::SqliteRagRepositorySql;
use rusqlite::params;

use crate::{
    application::rag::{
        RagDeactivateSourceCommand, RagSourceLifecycleRepository,
    },
    domain::rag::RagError,
};

use super::{
    SqliteRagRepository,
    indexes::delete_text_unit_indexes_for_source,
    sql_plan::{SqliteRagCorpusPlan, compile_sql_corpus_plan, quote_identifier},
    storage_error,
};

impl RagSourceLifecycleRepository for SqliteRagRepositorySql<'_> {
    fn deactivate_source(&self, command: RagDeactivateSourceCommand) -> Result<(), RagError> {
        let corpus = self
            .get_corpus(&command.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        let plan = compile_sql_corpus_plan(self, &corpus)?;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&self
            .connection, rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        tx.execute("UPDATE rag_record_tasks SET state='CANCELLED' WHERE corpus_id=?1 AND state IN ('RUNNING','QUEUED') AND json_extract(payload_json,'$.sourceCode')=?2 AND json_extract(payload_json,'$.sourceVersion')=?3 AND json_extract(payload_json,'$.scopeType')=?4 AND json_extract(payload_json,'$.scopeCode')=?5",params![command.corpus_id,command.source_code,command.source_version,command.scope.scope_type,command.scope.scope_code]).map_err(storage_error)?;
        mark_source_disabled(&tx, &plan, &command)?;
        delete_text_unit_indexes_for_source(
            &tx,
            &plan,
            &command.source_code,
            &command.source_version,
            &command.scope.scope_type,
            &command.scope.scope_code,
        )?;
        let partition=crate::domain::rag::graph::GraphPartition{corpus_id:command.corpus_id.clone(),scope_type:command.scope.scope_type.clone(),scope_code:command.scope.scope_code.clone()};
        super::super::rag_graph_index::retire_graph_source_version(&tx,&partition,&command.source_code,&command.source_version).map_err(|_|RagError::StorageFailure)?;
        tx.commit().map_err(storage_error)
    }
}

fn mark_source_disabled(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    plan: &SqliteRagCorpusPlan,
    command: &RagDeactivateSourceCommand,
) -> Result<(), RagError> {
    if let Some(source_table) = plan.source_table.as_deref() {
        update_status(tx, source_table, &plan.source_status_column, command)?;
    }
    update_status(
        tx,
        &plan.text_unit_table,
        &plan.text_unit_status_column,
        command,
    )?;
    if plan.embedded_source {
        update_status(
            tx,
            &plan.text_unit_table,
            &plan.source_status_column,
            command,
        )?;
    }
    Ok(())
}

fn update_status(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    table_name: &str,
    status_column: &str,
    command: &RagDeactivateSourceCommand,
) -> Result<(), RagError> {
    let table = quote_identifier(table_name)?;
    let status_column = quote_identifier(status_column)?;
    let sql = format!(
        "UPDATE {table}
         SET {status_column} = 'DISABLED'
         WHERE source_code = ?1
           AND source_version = ?2
           AND scope_type = ?3
           AND scope_code = ?4"
    );
    tx.execute(
        &sql,
        params![
            command.source_code,
            command.source_version,
            command.scope.scope_type,
            command.scope.scope_code,
        ],
    )
    .map_err(storage_error)?;
    Ok(())
}

impl RagSourceLifecycleRepository for SqliteRagRepository {
    fn deactivate_source(&self, command: RagDeactivateSourceCommand) -> Result<(), RagError> {
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.deactivate_source(command),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
