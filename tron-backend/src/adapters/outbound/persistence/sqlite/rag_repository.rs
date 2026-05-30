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
    },
};

use super::migrations::RAG_FOUNDATION_MIGRATION;

pub struct SqliteRagRepository {
    connection: Connection,
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

        let repository = Self { connection };
        repository.apply_rag_foundation_migration()?;
        Ok(repository)
    }

    pub fn apply_rag_foundation_migration(&self) -> Result<(), RagError> {
        self.connection
            .execute_batch(RAG_FOUNDATION_MIGRATION)
            .map_err(storage_error)
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
        statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(storage_error)?
            .collect::<Result<BTreeSet<_>, _>>()
            .map_err(storage_error)
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
    Ok(RagStorageBinding {
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
