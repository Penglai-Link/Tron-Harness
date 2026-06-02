use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::domain::rag::{RagCorpusRecord, RagError, RagStorageLayout};

use super::{SqliteRagRepository, storage_error};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqliteRagCorpusPlan {
    pub corpus_id: String,
    pub layout: RagStorageLayout,
    pub source_table: Option<String>,
    pub text_unit_table: String,
    pub diagnostics_table: Option<String>,
    pub source_columns: BTreeSet<String>,
    pub text_unit_columns: BTreeSet<String>,
    pub field_mapping: BTreeMap<String, String>,
    pub filter_columns: BTreeMap<String, SqliteRagFilterColumn>,
    pub source_metadata_columns: Vec<SqliteRagMetadataColumn>,
    pub source_status_column: String,
    pub text_unit_status_column: String,
    pub source_content_hash_column: Option<String>,
    pub text_unit_content_hash_column: Option<String>,
    pub source_metadata_column: Option<String>,
    pub text_unit_metadata_column: Option<String>,
    pub source_created_at_column: Option<String>,
    pub source_updated_at_column: Option<String>,
    pub text_unit_created_at_column: Option<String>,
    pub text_unit_updated_at_column: Option<String>,
    pub text_unit_unique_key: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SqliteRagTableAlias {
    Source,
    TextUnit,
}

impl SqliteRagTableAlias {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Source => "src",
            Self::TextUnit => "tu",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SqliteRagValueType {
    String,
    Integer,
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqliteRagFilterColumn {
    pub column: String,
    pub alias: SqliteRagTableAlias,
    pub value_type: SqliteRagValueType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqliteRagMetadataColumn {
    pub logical_name: String,
    pub column: String,
    pub value_type: SqliteRagValueType,
}

pub(crate) fn compile_sql_corpus_plan(
    repository: &SqliteRagRepository,
    corpus: &RagCorpusRecord,
) -> Result<SqliteRagCorpusPlan, RagError> {
    let binding = &corpus.storage_binding;
    let (source_table, text_unit_table) = match binding.layout {
        RagStorageLayout::Split => (
            Some(
                binding
                    .source_table
                    .as_deref()
                    .ok_or(RagError::StorageBindingInvalid)?
                    .to_string(),
            ),
            binding
                .text_unit_table
                .as_deref()
                .ok_or(RagError::StorageBindingInvalid)?
                .to_string(),
        ),
        RagStorageLayout::EmbeddedSource => (
            None,
            binding
                .table
                .as_deref()
                .ok_or(RagError::StorageBindingInvalid)?
                .to_string(),
        ),
    };
    validate_identifier(&text_unit_table)?;
    if let Some(table) = &source_table {
        validate_identifier(table)?;
    }
    if let Some(table) = &binding.diagnostics_table {
        validate_identifier(table)?;
    }

    let physical_source_columns = match &source_table {
        Some(table) => repository.table_columns(table)?,
        None => BTreeSet::new(),
    };
    let text_unit_columns = repository.table_columns(&text_unit_table)?;
    let source_columns = if source_table.is_some() {
        physical_source_columns
    } else {
        text_unit_columns.clone()
    };
    let filter_columns = compile_filter_columns(
        corpus,
        source_table.is_some(),
        &source_columns,
        &text_unit_columns,
    );
    let source_metadata_columns =
        compile_source_metadata_columns(corpus, &source_columns, &filter_columns);

    let source_status_column = match binding.layout {
        RagStorageLayout::Split => first_existing(&source_columns, &["source_status", "status"]),
        RagStorageLayout::EmbeddedSource => {
            first_existing(&text_unit_columns, &["source_status", "status"])
        }
    };

    let text_unit_unique_key = text_unit_unique_key(repository, &text_unit_table)?;

    Ok(SqliteRagCorpusPlan {
        corpus_id: corpus.corpus_id.clone(),
        layout: binding.layout,
        source_table,
        text_unit_table,
        diagnostics_table: binding.diagnostics_table.clone(),
        source_status_column: source_status_column.ok_or(RagError::StorageBindingInvalid)?,
        text_unit_status_column: first_existing(
            &text_unit_columns,
            &["text_unit_status", "status"],
        )
        .ok_or(RagError::StorageBindingInvalid)?,
        source_content_hash_column: first_existing(
            &source_columns,
            &["source_content_hash", "content_hash"],
        ),
        text_unit_content_hash_column: first_existing(
            &text_unit_columns,
            &["text_unit_content_hash", "content_hash"],
        ),
        source_metadata_column: first_existing(&source_columns, &["source_metadata", "metadata"]),
        text_unit_metadata_column: first_existing(
            &text_unit_columns,
            &["text_unit_metadata", "metadata"],
        ),
        source_created_at_column: first_existing(
            &source_columns,
            &["source_created_at", "created_at"],
        ),
        source_updated_at_column: first_existing(
            &source_columns,
            &["source_updated_at", "updated_at"],
        ),
        text_unit_created_at_column: first_existing(
            &text_unit_columns,
            &["text_unit_created_at", "created_at"],
        ),
        text_unit_updated_at_column: first_existing(
            &text_unit_columns,
            &["text_unit_updated_at", "updated_at"],
        ),
        text_unit_unique_key,
        field_mapping: corpus.field_mapping.clone(),
        source_columns,
        text_unit_columns,
        filter_columns,
        source_metadata_columns,
    })
}

pub(crate) fn quote_identifier(identifier: &str) -> Result<String, RagError> {
    validate_identifier(identifier)?;
    Ok(format!("\"{identifier}\""))
}

pub(crate) fn validate_identifier(identifier: &str) -> Result<(), RagError> {
    if !identifier.is_empty()
        && identifier.chars().all(|character| {
            character.is_ascii_lowercase() || character == '_' || character.is_ascii_digit()
        })
    {
        Ok(())
    } else {
        Err(RagError::StorageBindingInvalid)
    }
}

fn compile_filter_columns(
    corpus: &RagCorpusRecord,
    has_source_table: bool,
    source_columns: &BTreeSet<String>,
    text_unit_columns: &BTreeSet<String>,
) -> BTreeMap<String, SqliteRagFilterColumn> {
    parse_filter_fields(&corpus.filter_schema)
        .into_iter()
        .filter_map(|(logical_name, (column, value_type))| {
            let alias = choose_filter_alias(
                &logical_name,
                &column,
                has_source_table,
                source_columns,
                text_unit_columns,
            )?;
            Some((
                logical_name,
                SqliteRagFilterColumn {
                    column,
                    alias,
                    value_type,
                },
            ))
        })
        .collect()
}

fn compile_source_metadata_columns(
    corpus: &RagCorpusRecord,
    source_columns: &BTreeSet<String>,
    filter_columns: &BTreeMap<String, SqliteRagFilterColumn>,
) -> Vec<SqliteRagMetadataColumn> {
    let mut metadata_columns = BTreeMap::<String, SqliteRagMetadataColumn>::new();
    for (logical_name, column) in &corpus.field_mapping {
        if source_columns.contains(column) && !is_common_identity_field(logical_name) {
            let value_type = filter_columns
                .get(logical_name)
                .map(|field| field.value_type)
                .unwrap_or(SqliteRagValueType::String);
            metadata_columns.insert(
                logical_name.clone(),
                SqliteRagMetadataColumn {
                    logical_name: logical_name.clone(),
                    column: column.clone(),
                    value_type,
                },
            );
        }
    }
    for (logical_name, filter_column) in filter_columns {
        if filter_column.alias == SqliteRagTableAlias::Source
            && source_columns.contains(&filter_column.column)
            && !is_common_identity_field(logical_name)
        {
            metadata_columns
                .entry(logical_name.clone())
                .or_insert_with(|| SqliteRagMetadataColumn {
                    logical_name: logical_name.clone(),
                    column: filter_column.column.clone(),
                    value_type: filter_column.value_type,
                });
        }
    }
    metadata_columns.into_values().collect()
}

fn choose_filter_alias(
    logical_name: &str,
    column: &str,
    has_source_table: bool,
    source_columns: &BTreeSet<String>,
    text_unit_columns: &BTreeSet<String>,
) -> Option<SqliteRagTableAlias> {
    if logical_name == "status" || logical_name == "sourceStatus" {
        return (has_source_table && source_columns.contains(column))
            .then_some(SqliteRagTableAlias::Source)
            .or_else(|| {
                text_unit_columns
                    .contains(column)
                    .then_some(SqliteRagTableAlias::TextUnit)
            });
    }
    if logical_name == "textUnitStatus" {
        return text_unit_columns
            .contains(column)
            .then_some(SqliteRagTableAlias::TextUnit);
    }
    if matches!(
        logical_name,
        "sourceCode" | "sourceVersion" | "scopeType" | "scopeCode"
    ) {
        return text_unit_columns
            .contains(column)
            .then_some(SqliteRagTableAlias::TextUnit)
            .or_else(|| {
                if !has_source_table {
                    return None;
                }
                source_columns
                    .contains(column)
                    .then_some(SqliteRagTableAlias::Source)
            });
    }
    if has_source_table && source_columns.contains(column) {
        Some(SqliteRagTableAlias::Source)
    } else if text_unit_columns.contains(column) {
        Some(SqliteRagTableAlias::TextUnit)
    } else {
        None
    }
}

fn parse_filter_fields(value: &Value) -> BTreeMap<String, (String, SqliteRagValueType)> {
    value
        .get("fields")
        .and_then(Value::as_object)
        .map(|fields| {
            fields
                .iter()
                .filter_map(|(name, raw)| {
                    let column = raw.get("column").and_then(Value::as_str)?;
                    let value_type = match raw.get("type").and_then(Value::as_str) {
                        Some("integer" | "number") => SqliteRagValueType::Integer,
                        Some("boolean") => SqliteRagValueType::Boolean,
                        _ => SqliteRagValueType::String,
                    };
                    Some((name.clone(), (column.to_string(), value_type)))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn first_existing(columns: &BTreeSet<String>, candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .find(|candidate| columns.contains(**candidate))
        .map(|candidate| (*candidate).to_string())
}

fn text_unit_unique_key(
    repository: &SqliteRagRepository,
    table_name: &str,
) -> Result<Vec<String>, RagError> {
    let unique_indexes = unique_indexes(repository, table_name)?;
    let preferred = vec![
        "source_code".to_string(),
        "source_version".to_string(),
        "scope_type".to_string(),
        "scope_code".to_string(),
        "ordinal".to_string(),
    ];
    if unique_indexes.contains(&preferred) {
        return Ok(preferred);
    }
    let legacy = vec![
        "source_code".to_string(),
        "source_version".to_string(),
        "ordinal".to_string(),
    ];
    if unique_indexes.contains(&legacy) {
        return Ok(legacy);
    }
    Err(RagError::StorageBindingInvalid)
}

fn unique_indexes(
    repository: &SqliteRagRepository,
    table_name: &str,
) -> Result<Vec<Vec<String>>, RagError> {
    let pragma = format!("PRAGMA index_list({})", quote_identifier(table_name)?);
    let mut statement = repository
        .connection
        .prepare(&pragma)
        .map_err(storage_error)?;
    let index_names = statement
        .query_map([], |row| {
            let unique = row.get::<_, i64>(2)? == 1;
            let name = row.get::<_, String>(1)?;
            Ok(unique.then_some(name))
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let mut indexes = Vec::new();
    for index_name in index_names {
        let pragma = format!("PRAGMA index_info({})", quote_identifier(&index_name)?);
        let mut statement = repository
            .connection
            .prepare(&pragma)
            .map_err(storage_error)?;
        let columns = statement
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(2)?))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        let mut columns = columns;
        columns.sort_by_key(|(sequence, _)| *sequence);
        indexes.push(
            columns
                .into_iter()
                .map(|(_, column)| column)
                .collect::<Vec<_>>(),
        );
    }
    Ok(indexes)
}

fn is_common_identity_field(logical_name: &str) -> bool {
    matches!(
        logical_name,
        "sourceCode"
            | "sourceVersion"
            | "scopeType"
            | "scopeCode"
            | "status"
            | "sourceStatus"
            | "sourceMetadata"
            | "text"
            | "searchText"
            | "embedding"
    )
}
