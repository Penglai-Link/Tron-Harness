use super::{RagConfigRepository, RagError, SqliteRagRepositorySql};
use crate::application::rag::{RagIndexCounts, RagRecordTask};
use rusqlite::{OptionalExtension, params};

impl SqliteRagRepositorySql<'_> {
    pub(super) fn record_task(
        &self,
        user: &str,
        id: &str,
    ) -> Result<Option<RagRecordTask>, RagError> {
        self.record_task_inner(user, id, false)
    }
    pub(super) fn record_task_summary(&self, user: &str, id: &str) -> Result<Option<RagRecordTask>, RagError> {
        self.record_task_inner(user, id, true)
    }
    fn record_task_inner(&self, user: &str, id: &str, summary: bool) -> Result<Option<RagRecordTask>, RagError> {
        let row: Option<(String, String)> = self
            .connection
            .query_row(
                "SELECT CASE WHEN ?3 THEN json_set(payload_json,'$.payload',json('{}'),'$.checkpoint',json('null')) ELSE payload_json END,state FROM rag_record_tasks WHERE user_code=?1 AND id=?2",
                params![user, id, summary],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(super::storage_error)?;
        row.map(|(payload, state)| {
            let mut task: RagRecordTask =
                serde_json::from_str(&payload).map_err(|_| RagError::StorageFailure)?;
            task.state = state;
            if task.state == "COMPLETED" {
                task.stage = "PUBLISHING".into();
                task.completed_units = task.total_units;
            }
            Ok(task)
        })
        .transpose()
    }
    pub(super) fn save_record_task(&self, task: &RagRecordTask) -> Result<(), RagError> {
        let payload = serde_json::to_string(task).map_err(|_| RagError::StorageFailure)?;
        self.connection.execute(
            "INSERT INTO rag_record_tasks(id,user_code,corpus_id,state,payload_json,updated_at) VALUES (?1,?2,?3,?4,?5,?6)
            ON CONFLICT(id) DO UPDATE SET state=excluded.state,payload_json=excluded.payload_json,updated_at=excluded.updated_at WHERE rag_record_tasks.state NOT IN ('CANCELLED','COMPLETED') OR rag_record_tasks.state=excluded.state",
            params![task.id,task.user_code,task.corpus_id,task.state,payload,task.updated_at],
        ).map_err(super::storage_error)?;
        Ok(())
    }
    pub(super) fn record_tasks(&self, user: &str) -> Result<Vec<RagRecordTask>, RagError> {
        self.record_tasks_inner(user, false)
    }
    pub(super) fn record_task_summaries(&self, user: &str) -> Result<Vec<RagRecordTask>, RagError> {
        self.record_tasks_inner(user, true)
    }
    fn record_tasks_inner(&self, user: &str, summary: bool) -> Result<Vec<RagRecordTask>, RagError> {
        let mut statement = self.connection.prepare(
            "WITH ranked AS (
                SELECT payload_json,state,rowid AS sequence,
                    ROW_NUMBER() OVER (PARTITION BY corpus_id,
                        json_extract(payload_json,'$.sourceCode'),
                        json_extract(payload_json,'$.workspaceCode'),
                        json_extract(payload_json,'$.scopeType'),
                        json_extract(payload_json,'$.scopeCode') ORDER BY rowid DESC) AS attempt,
                    ROW_NUMBER() OVER (ORDER BY rowid DESC) AS recency
                FROM rag_record_tasks WHERE user_code=?1
            ) SELECT CASE WHEN ?2 THEN json_set(payload_json,
                '$.payload',json_object('title',json_extract(payload_json,'$.payload.title')),
                '$.checkpoint.vectors.vectors',json('[]')) ELSE payload_json END,state
              FROM ranked WHERE attempt=1 OR recency<=200 ORDER BY sequence DESC"
        ).map_err(super::storage_error)?;
        let rows = statement
            .query_map(params![user, summary], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(super::storage_error)?;
        rows.map(|row| {
            let (payload, state) = row.map_err(super::storage_error)?;
            let mut task: RagRecordTask =
                serde_json::from_str(&payload).map_err(|_| RagError::StorageFailure)?;
            task.state = state;
            if task.state == "COMPLETED" {
                task.stage = "PUBLISHING".into();
                task.completed_units = task.total_units;
            }
            Ok(task)
        })
        .collect()
    }
    pub(super) fn index_counts(&self, corpus_id: &str) -> Result<RagIndexCounts, RagError> {
        let corpus = self
            .get_corpus(corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        self.index_counts_for(&corpus)
    }
    pub(super) fn index_counts_for(&self, corpus: &crate::domain::rag::RagCorpusRecord) -> Result<RagIndexCounts, RagError> {
        let Some(table) = corpus.storage_binding.sqlite_table_for_segment("textUnit") else {
            return Ok(RagIndexCounts::default());
        };
        let table = super::sql_plan::quote_identifier(table)?;
        let plan = super::sql_plan::compile_sql_corpus_plan(self, &corpus)?;
        let status = super::sql_plan::quote_identifier(&plan.text_unit_status_column)?;
        let source_status = super::sql_plan::quote_identifier(&plan.source_status_column)?;
        let (join, source_alias) = if let Some(source) = &plan.source_table {
            (
                format!(
                    " JOIN {} src ON src.source_code=units.source_code AND src.source_version=units.source_version AND src.scope_type=units.scope_type AND src.scope_code=units.scope_code",
                    super::sql_plan::quote_identifier(source)?
                ),
                "src",
            )
        } else {
            (String::new(), "units")
        };
        let active = format!("units.{status}='ACTIVE' AND {source_alias}.{source_status}='ACTIVE'");
        let (source_count,chunk_count) = self.connection.query_row(&format!(
            "SELECT COUNT(DISTINCT units.source_code), COUNT(*) FROM {table} units {join} WHERE {active}"), [],
            |row| Ok((row.get::<_,usize>(0)?,row.get::<_,usize>(1)?))
        ).map_err(super::storage_error)?;
        let mut vector_count = 0;
        for vec_name in
            super::indexes::vector_index_tables(self.connection, plan.vector_storage_table())?
        {
            vector_count+=self.connection.query_row(&format!("SELECT COUNT(*) FROM {} vec JOIN {table} units ON units.id=vec.rowid {join} WHERE {active}",super::sql_plan::quote_identifier(&vec_name)?),[],|row|row.get::<_,usize>(0)).map_err(super::storage_error)?;
        }
        let settings = crate::application::rag::RagSettings::from_capabilities(&corpus.capabilities)?;
        let metadata_column = super::sql_plan::quote_identifier(
            plan.source_metadata_column.as_deref().unwrap_or("metadata"),
        )?;
        let graph_count = if settings.graph {
            self.connection.query_row(&format!("SELECT COUNT(DISTINCT p.entity_id) FROM rag_graph_node_projections p \
                JOIN rag_graph_generations g ON g.generation_id=p.generation_id \
                JOIN rag_graph_active_sources a ON a.generation_id=g.generation_id \
                WHERE g.corpus_id=?1 AND g.state='Active' AND EXISTS (SELECT 1 FROM {table} units {join} WHERE {active} \
                AND units.source_code=g.source_code AND units.source_version=g.source_version \
                AND units.scope_type=g.scope_type AND units.scope_code=g.scope_code \
                AND json_extract({source_alias}.{metadata_column},'$.graphGenerationId')=g.generation_id)"),
                [&corpus.corpus_id],|row|row.get(0)).map_err(super::storage_error)?
        } else {0};
        let mut statement = self.connection.prepare(&format!("SELECT DISTINCT {source_alias}.{metadata_column} FROM {table} units {join} WHERE {active}")).map_err(super::storage_error)?;
        let saved = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(super::storage_error)?;
        let mut rebuild_source_count = 0;
        for saved in saved {
            let value: serde_json::Value =
                serde_json::from_str(&saved.map_err(super::storage_error)?)
                    .map_err(|_| RagError::StorageFailure)?;
            let old: Option<crate::application::rag::RagSettings> =
                serde_json::from_value(value["indexSettings"].clone()).ok();
            if old.is_none_or(|old| {
                old.fulltext != settings.fulltext
                    || old.vector != settings.vector
                    || old.graph != settings.graph
                    || (settings.vector
                        && (old.embedding_model_code != settings.embedding_model_code
                            || old.embedding_dimension != settings.embedding_dimension))
                    || (settings.graph && old.graph_model_code != settings.graph_model_code)
                    || old.image_description_model_code != settings.image_description_model_code
            }) {
                rebuild_source_count += 1;
            }
        }
        Ok(RagIndexCounts {
            source_count,
            rebuild_source_count,
            chunk_count,
            vector_count,
            graph_count,
        })
    }
}
