use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, params};
use serde_json::{Value, json};

use crate::{
    application::rag::{
        RagConfigRepository, RagCorpusManagementRecord, RagManagementRepository,
        RagProfileListQuery,
    },
    domain::rag::{
        RagCorpusRecord, RagError, RagProfileRecord, RagProfileType, RagStatus, RagStorageBackend,
        RagStorageBinding, RagStorageSegmentTarget,
    },
    shared::sqlite_extensions::default_sqlite_extension_context,
};

use self::field_mapping::parse_field_mapping;
use super::migrations::{
    RAG_DOMAIN_FIELDS_MIGRATION, RAG_FOUNDATION_MIGRATION, RAG_INGEST_PROFILES_MIGRATION,
    RAG_MEMORY_INGEST_VECTOR_DEGRADE_MIGRATION, RAG_MEMORY_QUERY_PROFILE_MIGRATION,
    RAG_MEMORY_TARGETED_STORAGE_MIGRATION, RAG_MEMORY_TIMESTAMP_RETURN_FIELDS_MIGRATION,
    RAG_OPTIONAL_MODEL_CODES_MIGRATION, RAG_PROJECT_DOC_MINERU_INGEST_PROFILE_MIGRATION,
    RAG_SEARCH_PROFILES_MIGRATION, RAG_SKILL_CORPUS_MIGRATION, RAG_SKILL_GRAPH_CORPUS_MIGRATION,
    RAG_STORAGE_BACKEND_FUSION_MIGRATION, RAG_STORAGE_BINDING_LAYOUT_MIGRATION,
};

struct SqliteRagRepositorySql<'a> {
    connection: &'a mut Connection,
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
            .map_err(storage_error)?;
        let repository = Self { connection };
        repository.apply_rag_migrations()?;
        Ok(repository)
    }

    pub fn apply_rag_migrations(&self) -> Result<(), RagError> {
        self.apply_rag_foundation_migration()?;
        self.apply_rag_domain_fields_migration()?;
        self.apply_rag_ingest_profiles_migration()?;
        self.apply_rag_search_profiles_migration()?;
        self.apply_rag_optional_model_codes_migration()?;
        self.apply_rag_storage_binding_layout_migration()?;
        self.apply_rag_memory_targeted_storage_migration()?;
        self.apply_rag_skill_corpus_migration()?;
        self.apply_rag_skill_graph_corpus_migration()?;
        self.apply_rag_storage_backend_fusion_migration()?;
        self.apply_rag_memory_query_profile_migration()?;
        self.apply_rag_memory_timestamp_return_fields_migration()?;
        self.apply_rag_memory_ingest_vector_degrade_migration()?;
        self.apply_rag_project_doc_mineru_ingest_profile_migration()?;
        self.apply_rag_source_type_removal_migration()?;
        self.apply_rag_sqlite_extension_index_migration()
    }

    pub fn apply_rag_foundation_migration(&self) -> Result<(), RagError> {
        self.connection
            .execute_batch(RAG_FOUNDATION_MIGRATION)
            .map_err(storage_error)
    }

    pub fn apply_rag_domain_fields_migration(&self) -> Result<(), RagError> {
        ensure_legacy_memory_source_domain_columns(&self.connection)?;
        self.connection
            .execute_batch(RAG_DOMAIN_FIELDS_MIGRATION)
            .map_err(storage_error)
    }

    pub fn apply_rag_ingest_profiles_migration(&self) -> Result<(), RagError> {
        self.connection
            .execute_batch(RAG_INGEST_PROFILES_MIGRATION)
            .map_err(storage_error)
    }

    pub fn apply_rag_search_profiles_migration(&self) -> Result<(), RagError> {
        self.connection
            .execute_batch(RAG_SEARCH_PROFILES_MIGRATION)
            .map_err(storage_error)
    }

    pub fn apply_rag_optional_model_codes_migration(&self) -> Result<(), RagError> {
        self.connection
            .execute_batch(RAG_OPTIONAL_MODEL_CODES_MIGRATION)
            .map_err(storage_error)
    }

    pub fn apply_rag_storage_binding_layout_migration(&self) -> Result<(), RagError> {
        self.connection
            .execute_batch(RAG_STORAGE_BINDING_LAYOUT_MIGRATION)
            .map_err(storage_error)?;
        backfill_storage_binding_from_storage_topology(&self.connection)
    }

    pub fn apply_rag_memory_targeted_storage_migration(&self) -> Result<(), RagError> {
        self.connection
            .execute_batch(RAG_MEMORY_TARGETED_STORAGE_MIGRATION)
            .map_err(storage_error)
    }

    pub fn apply_rag_skill_corpus_migration(&self) -> Result<(), RagError> {
        self.connection
            .execute_batch(RAG_SKILL_CORPUS_MIGRATION)
            .map_err(storage_error)
    }

    pub fn apply_rag_skill_graph_corpus_migration(&self) -> Result<(), RagError> {
        self.connection
            .execute_batch(RAG_SKILL_GRAPH_CORPUS_MIGRATION)
            .map_err(storage_error)
    }

    pub fn apply_rag_storage_backend_fusion_migration(&self) -> Result<(), RagError> {
        self.connection
            .execute_batch(RAG_STORAGE_BACKEND_FUSION_MIGRATION)
            .map_err(storage_error)
    }

    pub fn apply_rag_memory_query_profile_migration(&self) -> Result<(), RagError> {
        self.connection
            .execute_batch(RAG_MEMORY_QUERY_PROFILE_MIGRATION)
            .map_err(storage_error)
    }

    pub fn apply_rag_memory_timestamp_return_fields_migration(&self) -> Result<(), RagError> {
        self.connection
            .execute_batch(RAG_MEMORY_TIMESTAMP_RETURN_FIELDS_MIGRATION)
            .map_err(storage_error)
    }

    pub fn apply_rag_memory_ingest_vector_degrade_migration(&self) -> Result<(), RagError> {
        self.connection
            .execute_batch(RAG_MEMORY_INGEST_VECTOR_DEGRADE_MIGRATION)
            .map_err(storage_error)
    }

    pub fn apply_rag_project_doc_mineru_ingest_profile_migration(&self) -> Result<(), RagError> {
        self.connection
            .execute_batch(RAG_PROJECT_DOC_MINERU_INGEST_PROFILE_MIGRATION)
            .map_err(storage_error)
    }

    pub fn apply_rag_source_type_removal_migration(&self) -> Result<(), RagError> {
        source_type_removal::apply(&self.connection)
    }

    pub fn apply_rag_sqlite_extension_index_migration(&self) -> Result<(), RagError> {
        indexes::apply_extension_index_migration(self)
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

    fn list_profiles(&self, query: RagProfileListQuery) -> Result<Vec<RagProfileRecord>, RagError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT profile_id,
                        profile_type,
                        status,
                        primary_corpus_id,
                        corpus_ids_json,
                        required_source_version,
                        config_json,
                        filters_json,
                        degrade_policy
                 FROM rag_profiles
                 ORDER BY profile_id",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([], map_profile_row)
            .map_err(storage_error)?;
        Ok(collect_rows(rows)?
            .into_iter()
            .filter(|profile: &RagProfileRecord| {
                query
                    .profile_type
                    .as_ref()
                    .is_none_or(|profile_type| profile.profile_type == *profile_type)
                    && query
                        .status
                        .as_ref()
                        .is_none_or(|status| profile.status == *status)
                    && query.corpus_id.as_ref().is_none_or(|corpus_id| {
                        profile.primary_corpus_id.as_ref() == Some(corpus_id)
                            || profile.corpus_ids.contains(corpus_id)
                    })
            })
            .collect())
    }

    fn get_profile(&self, profile_id: &str) -> Result<Option<RagProfileRecord>, RagError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT profile_id,
                        profile_type,
                        status,
                        primary_corpus_id,
                        corpus_ids_json,
                        required_source_version,
                        config_json,
                        filters_json,
                        degrade_policy
                 FROM rag_profiles
                 WHERE profile_id = ?1",
            )
            .map_err(storage_error)?;
        match statement.query_row([profile_id], map_profile_row) {
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

    fn save_profile(&self, record: &RagProfileRecord) -> Result<(), RagError> {
        let now = current_timestamp_millis();
        self.connection
            .execute(
                "INSERT INTO rag_profiles (
                    profile_id,
                    profile_type,
                    status,
                    primary_corpus_id,
                    corpus_ids_json,
                    required_source_version,
                    config_json,
                    filters_json,
                    degrade_policy,
                    created_at,
                    updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
                ON CONFLICT (profile_id) DO UPDATE SET
                    profile_type = excluded.profile_type,
                    status = excluded.status,
                    primary_corpus_id = excluded.primary_corpus_id,
                    corpus_ids_json = excluded.corpus_ids_json,
                    required_source_version = excluded.required_source_version,
                    config_json = excluded.config_json,
                    filters_json = excluded.filters_json,
                    degrade_policy = excluded.degrade_policy,
                    updated_at = excluded.updated_at",
                params![
                    record.profile_id,
                    record.profile_type.as_str(),
                    record.status.as_str(),
                    record.primary_corpus_id,
                    serialize_json_value(&record.corpus_ids)?,
                    record.required_source_version,
                    serialize_json_value(&record.config)?,
                    serialize_json_value(&record.filters)?,
                    record.degrade_policy,
                    now,
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn delete_profile(&self, profile_id: &str) -> Result<bool, RagError> {
        let deleted_rows = self
            .connection
            .execute(
                "DELETE FROM rag_profiles WHERE profile_id = ?1",
                [profile_id],
            )
            .map_err(storage_error)?;
        Ok(deleted_rows > 0)
    }
}

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> Result<Vec<T>, RagError> {
    rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
}

fn backfill_storage_binding_from_storage_topology(connection: &Connection) -> Result<(), RagError> {
    if !rag_corpora_has_column(connection, "storage_topology_json")? {
        return Ok(());
    }
    let mut statement = connection
        .prepare(
            "SELECT corpus_id, storage_binding_json, storage_topology_json
             FROM rag_corpora
             WHERE storage_topology_json IS NOT NULL
               AND trim(storage_topology_json) <> ''",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    drop(statement);

    for (corpus_id, storage_binding_json, storage_topology_json) in rows {
        if parse_storage_binding(&storage_binding_json).is_ok()
            || parse_storage_binding(&storage_topology_json).is_err()
        {
            continue;
        }
        connection
            .execute(
                "UPDATE rag_corpora
                 SET storage_binding_json = ?1
                 WHERE corpus_id = ?2",
                params![storage_topology_json, corpus_id],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn rag_corpora_has_column(connection: &Connection, column_name: &str) -> Result<bool, RagError> {
    let mut statement = connection
        .prepare("PRAGMA table_info(rag_corpora)")
        .map_err(storage_error)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(columns.iter().any(|column| column == column_name))
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
    pub fn apply_rag_migrations(&self) -> Result<(), RagError> {
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.apply_rag_migrations(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn apply_rag_foundation_migration(&self) -> Result<(), RagError> {
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.apply_rag_foundation_migration()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn apply_rag_domain_fields_migration(&self) -> Result<(), RagError> {
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.apply_rag_domain_fields_migration()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn apply_rag_ingest_profiles_migration(&self) -> Result<(), RagError> {
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.apply_rag_ingest_profiles_migration()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn apply_rag_search_profiles_migration(&self) -> Result<(), RagError> {
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.apply_rag_search_profiles_migration()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn apply_rag_optional_model_codes_migration(&self) -> Result<(), RagError> {
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.apply_rag_optional_model_codes_migration()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn apply_rag_storage_binding_layout_migration(&self) -> Result<(), RagError> {
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.apply_rag_storage_binding_layout_migration()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn apply_rag_memory_targeted_storage_migration(&self) -> Result<(), RagError> {
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.apply_rag_memory_targeted_storage_migration()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn apply_rag_skill_corpus_migration(&self) -> Result<(), RagError> {
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.apply_rag_skill_corpus_migration()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn apply_rag_skill_graph_corpus_migration(&self) -> Result<(), RagError> {
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.apply_rag_skill_graph_corpus_migration()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn apply_rag_storage_backend_fusion_migration(&self) -> Result<(), RagError> {
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.apply_rag_storage_backend_fusion_migration()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn apply_rag_memory_query_profile_migration(&self) -> Result<(), RagError> {
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.apply_rag_memory_query_profile_migration()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn apply_rag_memory_timestamp_return_fields_migration(&self) -> Result<(), RagError> {
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }
                    .apply_rag_memory_timestamp_return_fields_migration()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn apply_rag_memory_ingest_vector_degrade_migration(&self) -> Result<(), RagError> {
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }
                    .apply_rag_memory_ingest_vector_degrade_migration()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn apply_rag_project_doc_mineru_ingest_profile_migration(&self) -> Result<(), RagError> {
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }
                    .apply_rag_project_doc_mineru_ingest_profile_migration()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn apply_rag_source_type_removal_migration(&self) -> Result<(), RagError> {
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.apply_rag_source_type_removal_migration()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub fn apply_rag_sqlite_extension_index_migration(&self) -> Result<(), RagError> {
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }.apply_rag_sqlite_extension_index_migration()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    pub(crate) fn ensure_sqlite_runtime_storage_for_corpus(
        &self,
        corpus: &RagCorpusRecord,
    ) -> Result<(), RagError> {
        let corpus = corpus.to_owned();
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }
                    .ensure_sqlite_runtime_storage_for_corpus(&corpus)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
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
    fn list_profiles(&self, query: RagProfileListQuery) -> Result<Vec<RagProfileRecord>, RagError> {
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.list_profiles(query),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn get_profile(&self, profile_id: &str) -> Result<Option<RagProfileRecord>, RagError> {
        let profile_id = profile_id.to_owned();
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.get_profile(&profile_id),
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
    fn save_profile(&self, record: &RagProfileRecord) -> Result<(), RagError> {
        let record = record.to_owned();
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.save_profile(&record),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_profile(&self, profile_id: &str) -> Result<bool, RagError> {
        let profile_id = profile_id.to_owned();
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.delete_profile(&profile_id),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
