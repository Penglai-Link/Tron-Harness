use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, params};
use serde_json::{Value, json};

use crate::{
    application::rag::{RagConfigRepository, RagCorpusManagementRecord, RagManagementRepository},
    domain::rag::{
        RagCorpusRecord, RagError, RagStatus, RagStorageBackend, RagStorageBinding,
        RagStorageSegmentTarget,
    },
    shared::sqlite_extensions::default_sqlite_extension_context,
};

use self::field_mapping::parse_field_mapping;

struct SqliteRagRepositorySql<'a> {
    connection: &'a Connection,
}
pub struct SqliteRagRepository {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}

impl<'a> SqliteRagRepositorySql<'a> {
    pub fn initialize(
        database_url: &str,
        connection: &'a mut Connection,
    ) -> Result<Self, RagError> {
        ensure_database_parent(database_url)?;

        default_sqlite_extension_context()
            .prepare_connection(&connection)
            .map_err(|error| {
                tracing::error!("RAG SQLite extension error: {error}");
                RagError::StorageFailure
            })?;
        let repository = Self { connection };
        for corpus in repository.list_corpora()? {
            repository.ensure_sqlite_runtime_storage_for_corpus(&corpus)?;
        }
        Ok(repository)
    }

    pub(crate) fn ensure_sqlite_runtime_storage_for_corpus(
        &self,
        corpus: &RagCorpusRecord,
    ) -> Result<(), RagError> {
        index_schema::ensure_runtime_storage_for_corpus(self, corpus)
    }

    pub fn rag_table_names(&self) -> Result<BTreeSet<String>, RagError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT name
                 FROM sqlite_schema
                 WHERE type = 'table'
                   AND name LIKE 'rag_%'
                 ORDER BY name",
            )
            .map_err(storage_error)?;
        let names = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(storage_error)?
            .collect::<Result<BTreeSet<_>, _>>()
            .map_err(storage_error)?;
        Ok(names
            .into_iter()
            .filter(|name| !is_rag_extension_shadow_table(name))
            .collect())
    }

    pub fn table_columns(&self, table_name: &str) -> Result<BTreeSet<String>, RagError> {
        let pragma = format!("PRAGMA table_info({table_name})");
        let mut statement = self.connection.prepare(&pragma).map_err(storage_error)?;
        statement
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(storage_error)?
            .collect::<Result<BTreeSet<_>, _>>()
            .map_err(storage_error)
    }

    pub fn seeded_corpus_statuses(&self) -> Result<BTreeMap<String, String>, RagError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT corpus_id, status
                 FROM rag_corpora
                 ORDER BY corpus_id",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(storage_error)?;

        let mut statuses = BTreeMap::new();
        for row in rows {
            let (corpus_id, status) = row.map_err(storage_error)?;
            statuses.insert(corpus_id, status);
        }
        Ok(statuses)
    }
}

fn is_rag_extension_shadow_table(name: &str) -> bool {
    if name.contains("_fts_") {
        return true;
    }
    if let Some((_, suffix)) = name.rsplit_once("_vec_d") {
        return suffix.contains('_');
    }
    false
}

impl RagConfigRepository for SqliteRagRepositorySql<'_> {
    fn list_corpora(&self) -> Result<Vec<RagCorpusRecord>, RagError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT corpus_id,
                        display_name,
                        status,
                        description,
                        capabilities_json,
                        filter_schema_json,
                        field_mapping_json,
                        storage_binding_json
                 FROM rag_corpora
                 ORDER BY corpus_id",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([], map_corpus_row)
            .map_err(storage_error)?;
        collect_rows(rows)
    }

    fn get_corpus(&self, corpus_id: &str) -> Result<Option<RagCorpusRecord>, RagError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT corpus_id,
                        display_name,
                        status,
                        description,
                        capabilities_json,
                        filter_schema_json,
                        field_mapping_json,
                        storage_binding_json
                 FROM rag_corpora
                 WHERE corpus_id = ?1",
            )
            .map_err(storage_error)?;
        match statement.query_row([corpus_id], map_corpus_row) {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(storage_error(error)),
        }
    }
}

impl RagManagementRepository for SqliteRagRepositorySql<'_> {
    fn get_managed_corpus(
        &self,
        corpus_id: &str,
    ) -> Result<Option<RagCorpusManagementRecord>, RagError> {
        let sql = "SELECT corpus_id,
                    display_name,
                    status,
                    description,
                    capabilities_json,
                    filter_schema_json,
                    field_mapping_json,
                    storage_binding_json
             FROM rag_corpora
             WHERE corpus_id = ?1";
        let mut statement = self.connection.prepare(sql).map_err(storage_error)?;
        match statement.query_row([corpus_id], map_corpus_management_row) {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(storage_error(error)),
        }
    }

    fn save_managed_corpus(&self, record: &RagCorpusRecord) -> Result<(), RagError> {
        let now = current_timestamp_millis();
        let field_mapping_json = serialize_field_mapping(record)?;
        let storage_binding_json = serialize_storage_binding_value(&record.storage_binding)?;
        self.connection
            .execute(
                "INSERT INTO rag_corpora (
                    corpus_id,
                    display_name,
                    status,
                    description,
                    capabilities_json,
                    filter_schema_json,
                    field_mapping_json,
                    storage_binding_json,
                    created_at,
                    updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
                ON CONFLICT (corpus_id) DO UPDATE SET
                    display_name = excluded.display_name,
                    status = excluded.status,
                    description = excluded.description,
                    capabilities_json = excluded.capabilities_json,
                    filter_schema_json = excluded.filter_schema_json,
                    field_mapping_json = excluded.field_mapping_json,
                    storage_binding_json = excluded.storage_binding_json,
                    updated_at = excluded.updated_at",
                params![
                    record.corpus_id,
                    record.display_name,
                    record.status.as_str(),
                    record.description,
                    serialize_json_value(&record.capabilities)?,
                    serialize_json_value(&record.filter_schema)?,
                    field_mapping_json,
                    storage_binding_json,
                    now,
                ],
            )
            .map_err(storage_error)?;
        self.ensure_sqlite_runtime_storage_for_corpus(record)?;
        Ok(())
    }

    fn delete_managed_corpus(&self, corpus_id: &str) -> Result<bool, RagError> {
        let Some(managed) = self.get_managed_corpus(corpus_id)? else {
            return Ok(false);
        };
        let runtime_tables = self.unshared_sqlite_runtime_tables(corpus_id, &managed.record)?;

        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(self.connection,rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
        let result = (|| {
            let deleted_rows = self
                .connection
                .execute("DELETE FROM rag_corpora WHERE corpus_id = ?1", [corpus_id])
                .map_err(storage_error)?;
            if deleted_rows == 0 {
                return Ok(false);
            }
            self.connection
                .execute(
                    "DELETE FROM rag_graph_evidence WHERE corpus_id=?1",
                    [corpus_id],
                )
                .map_err(storage_error)?;
            self.connection
                .execute(
                    "DELETE FROM rag_record_tasks WHERE corpus_id=?1",
                    [corpus_id],
                )
                .map_err(storage_error)?;
            for table in &runtime_tables {
                drop_sqlite_runtime_table_family(&self.connection, table)?;
            }
            Ok(true)
        })();
        match result {
            Ok(deleted) => {
                transaction.commit().map_err(storage_error)?;
                Ok(deleted)
            }
            Err(error) => {
                let _ = transaction.rollback();
                Err(error)
            }
        }
    }
}

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> Result<Vec<T>, RagError> {
    rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
}

impl SqliteRagRepository {
    pub fn connect(database_url: &str) -> Result<Self, RagError> {
        let connection = crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(database_url).map_err(storage_error)?;
        let database_url = database_url.to_owned();
        connection.call(
            move |connection| {
                SqliteRagRepositorySql::initialize(&database_url, connection).map(|_| ())
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )?;
        Ok(Self { connection })
    }
}
impl SqliteRagRepository {
    pub fn rag_table_names(&self) -> Result<BTreeSet<String>, RagError> {
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.rag_table_names(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn table_columns(&self, table_name: &str) -> Result<BTreeSet<String>, RagError> {
        let table_name = table_name.to_owned();
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.table_columns(&table_name),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn seeded_corpus_statuses(&self) -> Result<BTreeMap<String, String>, RagError> {
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.seeded_corpus_statuses(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
impl RagConfigRepository for SqliteRagRepository {
    fn record_task_is_latest(
        &self,
        task: &crate::application::rag::RagRecordTask,
    ) -> Result<bool, RagError> {
        let task = task.clone();
        self.connection.call(move |connection| {
            connection.query_row("SELECT NOT EXISTS (SELECT 1 FROM rag_record_tasks newer WHERE newer.user_code=?1 AND newer.corpus_id=?2 AND json_extract(newer.payload_json,'$.sourceCode')=?3 AND json_extract(newer.payload_json,'$.sourceVersion')=?4 AND json_extract(newer.payload_json,'$.scopeType')=?5 AND json_extract(newer.payload_json,'$.scopeCode')=?6 AND newer.rowid>(SELECT rowid FROM rag_record_tasks WHERE id=?7))",
            rusqlite::params![task.user_code,task.corpus_id,task.source_code,task.source_version,task.scope_type,task.scope_code,task.id], |row| row.get(0)).map_err(storage_error)
        }, || RagError::StorageFailure)
    }
    fn claim_record_task(&self, user: &str, id: &str) -> Result<bool, RagError> {
        let user = user.to_owned();
        let id = id.to_owned();
        self.connection.call(move|connection| {
            let count=connection.execute("UPDATE rag_record_tasks SET state='RUNNING',payload_json=json_set(payload_json,'$.state','RUNNING') WHERE user_code=?1 AND id=?2 AND state='QUEUED'",rusqlite::params![user,id]).map_err(storage_error)?;
            Ok(count==1)
        },||RagError::StorageFailure)
    }
    fn recover_record_tasks(&self) -> Result<(), RagError> {
        self.connection.call(move|connection| {
            connection.execute("UPDATE rag_record_tasks SET state='FAILED',payload_json=json_set(payload_json,'$.state','FAILED','$.error','服务重启中断入库，可重试并复用检查点') WHERE state IN ('RUNNING','QUEUED')",[]).map_err(storage_error)?;Ok(())
        },||RagError::StorageFailure)
    }
    fn save_record_task(
        &self,
        task: &crate::application::rag::RagRecordTask,
    ) -> Result<(), RagError> {
        let task = task.clone();
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.save_record_task(&task),
            || RagError::StorageFailure,
        )
    }
    fn record_tasks(
        &self,
        user: &str,
    ) -> Result<Vec<crate::application::rag::RagRecordTask>, RagError> {
        let user = user.to_owned();
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.record_tasks(&user),
            || RagError::StorageFailure,
        )
    }
    fn record_task(
        &self,
        user: &str,
        id: &str,
    ) -> Result<Option<crate::application::rag::RagRecordTask>, RagError> {
        let user = user.to_owned();
        let id = id.to_owned();
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.record_task(&user, &id),
            || RagError::StorageFailure,
        )
    }
    fn index_counts(
        &self,
        corpus: &str,
    ) -> Result<crate::application::rag::RagIndexCounts, RagError> {
        let corpus = corpus.to_owned();
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.index_counts(&corpus),
            || RagError::StorageFailure,
        )
    }
    fn list_corpora(&self) -> Result<Vec<RagCorpusRecord>, RagError> {
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.list_corpora(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn get_corpus(&self, corpus_id: &str) -> Result<Option<RagCorpusRecord>, RagError> {
        let corpus_id = corpus_id.to_owned();
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.get_corpus(&corpus_id),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
impl RagManagementRepository for SqliteRagRepository {
    fn get_managed_corpus(
        &self,
        corpus_id: &str,
    ) -> Result<Option<RagCorpusManagementRecord>, RagError> {
        let corpus_id = corpus_id.to_owned();
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.get_managed_corpus(&corpus_id),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn save_managed_corpus(&self, record: &RagCorpusRecord) -> Result<(), RagError> {
        let record = record.to_owned();
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.save_managed_corpus(&record),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_managed_corpus(&self, corpus_id: &str) -> Result<bool, RagError> {
        let corpus_id = corpus_id.to_owned();
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.delete_managed_corpus(&corpus_id)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
