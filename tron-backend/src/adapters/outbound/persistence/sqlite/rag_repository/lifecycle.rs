use rusqlite::{Transaction, params};

use crate::{
    application::rag::{
        RagConfigRepository, RagDeactivateSourceCommand, RagSourceLifecycleRepository,
    },
    domain::rag::RagError,
};

use super::{
    SqliteRagRepository,
    indexes::delete_text_unit_indexes_for_source,
    sql_plan::{SqliteRagCorpusPlan, compile_sql_corpus_plan, quote_identifier},
    storage_error,
};

impl RagSourceLifecycleRepository for SqliteRagRepository {
    fn deactivate_source(&self, command: RagDeactivateSourceCommand) -> Result<(), RagError> {
        let corpus = self
            .get_corpus(&command.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        let plan = compile_sql_corpus_plan(self, &corpus)?;
        let tx = self
            .connection
            .unchecked_transaction()
            .map_err(storage_error)?;
        mark_source_disabled(&tx, &plan, &command)?;
        delete_text_unit_indexes_for_source(
            &tx,
            &plan,
            &command.source_code,
            &command.source_version,
            &command.scope.scope_type,
            &command.scope.scope_code,
        )?;
        tx.commit().map_err(storage_error)
    }
}

fn mark_source_disabled(
    tx: &Transaction<'_>,
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
    tx: &Transaction<'_>,
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
