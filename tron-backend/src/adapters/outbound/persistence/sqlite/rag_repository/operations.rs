use super::{RagError, SqliteRagRepositorySql};
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

impl SqliteRagRepositorySql<'_> {
    /// Three SQL statements for the catalogue, its schema metadata and all counts.
    /// VALUES avoids SQLite's compound-SELECT limit for large corpus batches.
    pub(super) fn list_corpora_with_counts(
        &self,
    ) -> Result<Vec<(crate::domain::rag::RagCorpusRecord, RagIndexCounts)>, RagError> {
        use std::collections::{BTreeMap, BTreeSet};
        let corpora = self.list_corpora()?;
        if corpora.is_empty() {
            return Ok(Vec::new());
        }
        let tables = corpora
            .iter()
            .flat_map(|corpus| {
                ["source", "textUnit", "fulltext", "vector", "tree"]
                    .into_iter()
                    .filter_map(move |segment| {
                        corpus
                            .storage_binding
                            .sqlite_table_for_segment(segment)
                            .map(str::to_owned)
                    })
            })
            .collect::<BTreeSet<_>>();
        let inspected = corpora
            .iter()
            .flat_map(|corpus| {
                ["source", "textUnit"]
                    .into_iter()
                    .filter_map(move |segment| {
                        corpus
                            .storage_binding
                            .sqlite_table_for_segment(segment)
                            .map(str::to_owned)
                    })
            })
            .collect::<BTreeSet<_>>();
        let mut schema = BTreeMap::<String, BTreeSet<String>>::new();
        let mut vector_tables = BTreeMap::<String, String>::new();
        {
            let mut q = self
                .connection
                .prepare("SELECT name,sql FROM sqlite_schema WHERE type='table'")
                .map_err(super::storage_error)?;
            let names = q
                .query_map([], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
                })
                .map_err(super::storage_error)?
                .collect::<Result<BTreeMap<_, _>, _>>()
                .map_err(super::storage_error)?;
            for (table, definition) in &names {
                let vector = table
                    .rsplit_once("_vec_d")
                    .is_some_and(|(base, dimension)| {
                        tables.contains(base) && dimension.parse::<usize>().is_ok()
                    });
                if !tables.contains(table) && !vector {
                    continue;
                }
                // Preparing a zero-row projection exposes column metadata without
                // stepping a SELECT, traversing table rows or invoking per-table PRAGMAs.
                let columns = if inspected.contains(table) {
                    let quoted = super::sql_plan::quote_identifier(&table)?;
                    let statement = self
                        .connection
                        .prepare(&format!("SELECT * FROM {quoted} LIMIT 0"))
                        .map_err(super::storage_error)?;
                    statement
                        .column_names()
                        .into_iter()
                        .map(str::to_owned)
                        .collect()
                } else {
                    BTreeSet::new()
                };
                if vector {
                    // vec0 scans issue an internal SELECT per virtual table. Its native
                    // row-ID index contains exactly the live vectors; joining it avoids
                    // those hidden queries and never loads embedding payloads.
                    let rowids = format!("{table}_rowids");
                    let native_vec = definition
                        .as_ref()
                        .is_some_and(|sql| sql.to_ascii_lowercase().contains("using vec0"));
                    let storage = if native_vec {
                        if !names.contains_key(&rowids) {
                            return Err(RagError::StorageBindingInvalid);
                        }
                        rowids
                    } else {
                        table.clone()
                    };
                    vector_tables.insert(table.clone(), storage);
                }
                schema.insert(table.clone(), columns);
            }
        }
        // Keep per-source metadata inside SQLite: return only aggregate counts.
        self.connection
            .create_scalar_function(
                "tron_rag_index_settings_changed",
                2,
                rusqlite::functions::FunctionFlags::SQLITE_UTF8
                    | rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC,
                |context| {
                    let saved: String = context.get(0)?;
                    let desired: String = context.get(1)?;
                    let saved: serde_json::Value = serde_json::from_str(&saved)
                        .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
                    let desired: crate::application::rag::RagSettings =
                        serde_json::from_str(&desired)
                            .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
                    let old = serde_json::from_value::<crate::application::rag::RagSettings>(
                        saved["indexSettings"].clone(),
                    )
                    .ok();
                    Ok(old.is_none_or(|old| {
                        old.fulltext != desired.fulltext
                            || old.vector != desired.vector
                            || old.graph != desired.graph
                            || (desired.vector
                                && (old.embedding_model_code != desired.embedding_model_code
                                    || old.embedding_dimension != desired.embedding_dimension))
                            || (desired.graph && old.graph_model_code != desired.graph_model_code)
                            || old.image_description_model_code
                                != desired.image_description_model_code
                    }))
                },
            )
            .map_err(super::storage_error)?;
        let mut values = Vec::new();
        let mut bindings = Vec::<String>::new();
        for (index, corpus) in corpora.iter().enumerate() {
            let config =
                crate::application::rag::RagSettings::from_capabilities(&corpus.capabilities)?;
            bindings.push(corpus.corpus_id.clone());
            bindings.push(serde_json::to_string(&config).map_err(|_| RagError::StorageFailure)?);
            let Some(table) = corpus.storage_binding.sqlite_table_for_segment("textUnit") else {
                values.push(format!(
                    "({index},'{{}}',0*length(?{}),0,0*length(?{}))",
                    index * 2 + 1,
                    index * 2 + 2
                ));
                continue;
            };
            let unit_table = table;
            let source_table = corpus
                .storage_binding
                .sqlite_table_for_segment("source")
                .filter(|source| *source != unit_table);
            let unit_columns = schema
                .get(unit_table)
                .ok_or(RagError::StorageBindingInvalid)?;
            let source_columns = schema
                .get(source_table.unwrap_or(unit_table))
                .ok_or(RagError::StorageBindingInvalid)?;
            let choose = |columns: &BTreeSet<String>, names: &[&str]| {
                names
                    .iter()
                    .find(|name| columns.contains(**name))
                    .map(|s| s.to_string())
                    .ok_or(RagError::StorageBindingInvalid)
            };
            let status = super::sql_plan::quote_identifier(&choose(
                unit_columns,
                &["text_unit_status", "status"],
            )?)?;
            let source_status = super::sql_plan::quote_identifier(&choose(
                source_columns,
                &["source_status", "status"],
            )?)?;
            let table = super::sql_plan::quote_identifier(unit_table)?;
            let (join, alias) = if let Some(source) = source_table {
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
            let active = format!("units.{status}='ACTIVE' AND {alias}.{source_status}='ACTIVE'");
            let base = format!(
                "(SELECT json_object('sourceCount',COUNT(DISTINCT units.source_code),'chunkCount',COUNT(*)) FROM {table} units {join} WHERE {active})"
            );
            let prefix = format!(
                "{}_vec_d",
                corpus
                    .storage_binding
                    .sqlite_table_for_segment("vector")
                    .unwrap_or(unit_table)
            );
            let vectors=vector_tables.iter().filter(|(name,_)|name.strip_prefix(&prefix).is_some_and(|suffix|suffix.parse::<usize>().is_ok())).map(|(_,physical)|super::sql_plan::quote_identifier(physical).map(|name|format!("(SELECT COUNT(*) FROM {name} vec JOIN {table} units ON units.id=vec.rowid {join} WHERE {active})"))).collect::<Result<Vec<_>,_>>()?;
            let vectors = if vectors.is_empty() {
                "0".into()
            } else {
                vectors.join("+")
            };
            let metadata = super::sql_plan::quote_identifier(&choose(
                source_columns,
                &["source_metadata", "metadata"],
            )?)?;
            let graph = if config.graph {
                format!(
                    "(SELECT COUNT(DISTINCT p.entity_id) FROM rag_graph_node_projections p JOIN rag_graph_generations g ON g.generation_id=p.generation_id JOIN rag_graph_active_sources a ON a.generation_id=g.generation_id WHERE g.corpus_id=?{} AND g.state='Active' AND EXISTS(SELECT 1 FROM {table} units {join} WHERE {active} AND units.source_code=g.source_code AND units.source_version=g.source_version AND units.scope_type=g.scope_type AND units.scope_code=g.scope_code AND json_extract({alias}.{metadata},'$.graphGenerationId')=g.generation_id))",
                    index * 2 + 1
                )
            } else {
                format!("0*length(?{})", index * 2 + 1)
            };
            let saved = format!(
                "(SELECT COUNT(*) FROM (SELECT DISTINCT {alias}.{metadata} AS metadata FROM {table} units {join} WHERE {active}) WHERE tron_rag_index_settings_changed(metadata,?{}))",
                index * 2 + 2
            );
            values.push(format!("({index},{base},{vectors},{graph},{saved})"));
        }
        let sql = format!(
            "WITH counts(ordinal,base,vectors,graph,settings) AS (VALUES {}) SELECT ordinal,base,vectors,graph,settings FROM counts ORDER BY ordinal",
            values.join(",")
        );
        let mut q = self
            .connection
            .prepare(&sql)
            .map_err(super::storage_error)?;
        let rows = q
            .query_map(rusqlite::params_from_iter(bindings), |r| {
                Ok((
                    r.get::<_, usize>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, usize>(2)?,
                    r.get::<_, usize>(3)?,
                    r.get::<_, usize>(4)?,
                ))
            })
            .map_err(super::storage_error)?;
        let mut result = Vec::new();
        for row in rows {
            let (index, base, vector_count, graph_count, rebuild_source_count) =
                row.map_err(super::storage_error)?;
            let base: serde_json::Value =
                serde_json::from_str(&base).map_err(|_| RagError::StorageFailure)?;
            result.push((
                corpora[index].clone(),
                RagIndexCounts {
                    source_count: base["sourceCount"].as_u64().unwrap_or(0) as usize,
                    chunk_count: base["chunkCount"].as_u64().unwrap_or(0) as usize,
                    vector_count,
                    graph_count,
                    rebuild_source_count,
                },
            ));
        }
        Ok(result)
    }
}
