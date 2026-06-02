mod diagnostics;
mod indexes;
mod ingest;
mod ingest_values;
mod retry;
mod search;
mod sql_plan;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use rusqlite::Connection;
use serde_json::Value;

use crate::{
    application::rag::{RagConfigRepository, RagProfileListQuery},
    domain::rag::{
        RagCorpusRecord, RagError, RagProfileRecord, RagProfileType, RagStatus, RagStorageBinding,
        RagStorageLayout,
    },
    shared::sqlite_extensions::{SqliteExtensionCapabilities, default_sqlite_extension_context},
};

use super::migrations::{
    RAG_DOMAIN_FIELDS_MIGRATION, RAG_FOUNDATION_MIGRATION, RAG_INGEST_PROFILES_MIGRATION,
    RAG_MEMORY_EMBEDDED_SOURCE_MIGRATION, RAG_OPTIONAL_MODEL_CODES_MIGRATION,
    RAG_SEARCH_PROFILES_MIGRATION, RAG_SKILL_CORPUS_MIGRATION,
    RAG_STORAGE_BINDING_LAYOUT_MIGRATION,
};

pub struct SqliteRagRepository {
    connection: Connection,
    sqlite_extensions: SqliteExtensionCapabilities,
}

impl SqliteRagRepository {
    pub fn connect(database_url: &str) -> Result<Self, RagError> {
        ensure_database_parent(database_url)?;
        let connection = if database_url == ":memory:" || database_url.starts_with("file:") {
            Connection::open_with_flags(
                database_url,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                    | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                    | rusqlite::OpenFlags::SQLITE_OPEN_URI,
            )
        } else {
            Connection::open(database_url)
        }
        .map_err(storage_error)?;

        let sqlite_extensions = default_sqlite_extension_context()
            .prepare_connection(&connection)
            .map_err(storage_error)?;
        let repository = Self {
            connection,
            sqlite_extensions,
        };
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
        self.apply_rag_memory_embedded_source_migration()?;
        self.apply_rag_skill_corpus_migration()?;
        self.apply_rag_sqlite_extension_index_migration()
    }

    pub fn apply_rag_foundation_migration(&self) -> Result<(), RagError> {
        self.connection
            .execute_batch(RAG_FOUNDATION_MIGRATION)
            .map_err(storage_error)
    }

    pub fn apply_rag_domain_fields_migration(&self) -> Result<(), RagError> {
        self.ensure_columns(
            "rag_memory_sources",
            &[
                ("memory_type", "memory_type TEXT"),
                ("memory_status", "memory_status TEXT"),
                ("review_status", "review_status TEXT"),
                ("version", "version TEXT"),
                ("superseded_by", "superseded_by TEXT"),
                ("conflict_group", "conflict_group TEXT"),
                ("duplicate_group", "duplicate_group TEXT"),
                ("injectable", "injectable INTEGER NOT NULL DEFAULT 1"),
                ("importance_score", "importance_score INTEGER"),
                ("confidence_score", "confidence_score INTEGER"),
                ("quality_score", "quality_score INTEGER"),
                ("pin_level", "pin_level INTEGER NOT NULL DEFAULT 0"),
                ("expires_at", "expires_at INTEGER"),
                ("derived_source_type", "derived_source_type TEXT"),
                ("derived_source_code", "derived_source_code TEXT"),
            ],
        )?;
        self.ensure_columns(
            "rag_memory_text_units",
            &[
                (
                    "source_type",
                    "source_type TEXT NOT NULL DEFAULT 'PLAIN_TEXT'",
                ),
                (
                    "source_status",
                    "source_status TEXT NOT NULL DEFAULT 'PENDING'",
                ),
                ("title", "title TEXT"),
                ("source_content_hash", "source_content_hash TEXT"),
                (
                    "source_metadata",
                    "source_metadata TEXT NOT NULL DEFAULT '{}'",
                ),
                (
                    "source_created_at",
                    "source_created_at INTEGER NOT NULL DEFAULT 0",
                ),
                (
                    "source_updated_at",
                    "source_updated_at INTEGER NOT NULL DEFAULT 0",
                ),
                ("memory_type", "memory_type TEXT"),
                ("memory_status", "memory_status TEXT"),
                ("review_status", "review_status TEXT"),
                ("version", "version TEXT"),
                ("superseded_by", "superseded_by TEXT"),
                ("conflict_group", "conflict_group TEXT"),
                ("duplicate_group", "duplicate_group TEXT"),
                ("injectable", "injectable INTEGER NOT NULL DEFAULT 1"),
                ("importance_score", "importance_score INTEGER"),
                ("confidence_score", "confidence_score INTEGER"),
                ("quality_score", "quality_score INTEGER"),
                ("pin_level", "pin_level INTEGER NOT NULL DEFAULT 0"),
                ("expires_at", "expires_at INTEGER"),
                ("derived_source_type", "derived_source_type TEXT"),
                ("derived_source_code", "derived_source_code TEXT"),
            ],
        )?;
        self.ensure_columns(
            "rag_project_doc_sources",
            &[
                ("repo_code", "repo_code TEXT"),
                ("module_code", "module_code TEXT"),
                ("module_name", "module_name TEXT"),
            ],
        )?;
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
            .map_err(storage_error)
    }

    pub fn apply_rag_memory_embedded_source_migration(&self) -> Result<(), RagError> {
        self.connection
            .execute_batch(RAG_MEMORY_EMBEDDED_SOURCE_MIGRATION)
            .map_err(storage_error)
    }

    pub fn apply_rag_skill_corpus_migration(&self) -> Result<(), RagError> {
        self.connection
            .execute_batch(RAG_SKILL_CORPUS_MIGRATION)
            .map_err(storage_error)
    }

    pub fn apply_rag_sqlite_extension_index_migration(&self) -> Result<(), RagError> {
        indexes::apply_extension_index_migration(self)
    }

    pub fn sqlite_extension_capabilities(&self) -> SqliteExtensionCapabilities {
        self.sqlite_extensions.clone()
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

    fn ensure_columns(&self, table_name: &str, columns: &[(&str, &str)]) -> Result<(), RagError> {
        for (column_name, column_definition) in columns {
            if !self.column_exists(table_name, column_name)? {
                self.connection
                    .execute_batch(&format!(
                        "ALTER TABLE {table_name} ADD COLUMN {column_definition};"
                    ))
                    .map_err(storage_error)?;
            }
        }
        Ok(())
    }

    fn column_exists(&self, table_name: &str, column_name: &str) -> Result<bool, RagError> {
        Ok(self.table_columns(table_name)?.contains(column_name))
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

impl RagConfigRepository for SqliteRagRepository {
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

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> Result<Vec<T>, RagError> {
    rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
}

fn map_corpus_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RagCorpusRecord> {
    let status = row.get::<_, String>(2)?;
    let capabilities_json = row.get::<_, String>(4)?;
    let filter_schema_json = row.get::<_, String>(5)?;
    let field_mapping_json = row.get::<_, String>(6)?;
    let storage_binding_json = row.get::<_, String>(7)?;

    Ok(RagCorpusRecord {
        corpus_id: row.get(0)?,
        display_name: row.get(1)?,
        status: RagStatus::try_from(status.as_str()).map_err(to_sql_error)?,
        description: row.get(3)?,
        capabilities: parse_json_value(&capabilities_json)?,
        filter_schema: parse_json_value(&filter_schema_json)?,
        field_mapping: parse_json_value::<BTreeMap<String, String>>(&field_mapping_json)?,
        storage_binding: parse_storage_binding(&storage_binding_json)?,
    })
}

fn map_profile_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RagProfileRecord> {
    let profile_type = row.get::<_, String>(1)?;
    let status = row.get::<_, String>(2)?;
    let corpus_ids_json = row.get::<_, String>(4)?;
    let config_json = row.get::<_, String>(6)?;
    let filters_json = row.get::<_, String>(7)?;

    Ok(RagProfileRecord {
        profile_id: row.get(0)?,
        profile_type: RagProfileType::try_from(profile_type.as_str()).map_err(to_sql_error)?,
        status: RagStatus::try_from(status.as_str()).map_err(to_sql_error)?,
        primary_corpus_id: row.get(3)?,
        corpus_ids: parse_json_value(&corpus_ids_json)?,
        required_source_version: row.get(5)?,
        config: parse_json_value(&config_json)?,
        filters: parse_json_value(&filters_json)?,
        degrade_policy: row.get(8)?,
    })
}

fn parse_storage_binding(raw: &str) -> rusqlite::Result<RagStorageBinding> {
    let value: Value = parse_json_value(raw)?;
    let layout = value
        .get("layout")
        .and_then(Value::as_str)
        .map(RagStorageLayout::try_from)
        .transpose()
        .map_err(to_sql_error)?
        .unwrap_or_else(|| {
            if value.get("table").and_then(Value::as_str).is_some() {
                RagStorageLayout::EmbeddedSource
            } else {
                RagStorageLayout::Split
            }
        });
    Ok(RagStorageBinding {
        layout,
        table: value
            .get("table")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        source_table: value
            .get("sourceTable")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        text_unit_table: value
            .get("textUnitTable")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        diagnostics_table: value
            .get("diagnosticsTable")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
    })
}

fn parse_json_value<T>(raw: &str) -> rusqlite::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(raw).map_err(|_| to_sql_error(RagError::StorageFailure))
}

fn to_sql_error(error: RagError) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

fn ensure_database_parent(database_url: &str) -> Result<(), RagError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| RagError::StorageFailure)?;
    }
    Ok(())
}

fn storage_error(_error: rusqlite::Error) -> RagError {
    RagError::StorageFailure
}

#[cfg(test)]
mod tests {
    use crate::{
        application::rag::{RagConfigRepository, RagProfileListQuery},
        domain::rag::{RagProfileType, RagStatus, RagStorageLayout},
    };

    use super::SqliteRagRepository;

    #[test]
    fn skill_corpus_migration_seeds_embedded_table_and_profiles() {
        let repository = SqliteRagRepository::connect(":memory:").unwrap();

        let table_names = repository.rag_table_names().unwrap();
        assert!(table_names.contains("rag_skill_text_units"));

        let columns = repository.table_columns("rag_skill_text_units").unwrap();
        assert!(columns.contains("scope_code"));
        assert!(columns.contains("skill_name"));
        assert!(columns.contains("skill_description"));
        assert!(!columns.contains("workspace_code"));

        let corpus = repository.get_corpus("skill").unwrap().unwrap();
        assert_eq!(corpus.status, RagStatus::Enabled);
        assert_eq!(
            corpus.storage_binding.layout,
            RagStorageLayout::EmbeddedSource
        );
        assert_eq!(
            corpus.storage_binding.table.as_deref(),
            Some("rag_skill_text_units")
        );
        assert_eq!(
            corpus.field_mapping.get("skillName").map(String::as_str),
            Some("skill_name")
        );
        assert!(!corpus.field_mapping.contains_key("workspaceCode"));

        let profiles = repository
            .list_profiles(RagProfileListQuery {
                profile_type: None,
                status: Some(RagStatus::Enabled),
                corpus_id: Some("skill".into()),
            })
            .unwrap();
        let profile_ids = profiles
            .iter()
            .map(|profile| profile.profile_id.as_str())
            .collect::<Vec<_>>();
        assert!(profile_ids.contains(&"skill-ingest-v1"));
        assert!(profile_ids.contains(&"skill-related-v1"));
        assert!(
            profiles
                .iter()
                .any(|profile| profile.profile_type == RagProfileType::Ingest)
        );
        let ingest_profile = profiles
            .iter()
            .find(|profile| profile.profile_id == "skill-ingest-v1")
            .expect("skill ingest profile should be seeded");
        assert_eq!(
            ingest_profile
                .config
                .get("payloadStrategyId")
                .and_then(serde_json::Value::as_str),
            Some("RAG_JSON_TEXT_PAYLOAD_V1")
        );
        assert_eq!(
            ingest_profile
                .config
                .get("payloadMapping")
                .and_then(|mapping| mapping.get("fields"))
                .and_then(|fields| fields.get("skillName"))
                .and_then(|field| field.get("target"))
                .and_then(serde_json::Value::as_str),
            Some("skill_name")
        );
        assert!(
            profiles
                .iter()
                .any(|profile| profile.profile_type == RagProfileType::Retrieval)
        );
    }
}
