use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::domain::rag::{RagCorpusRecord, RagError};

use super::{SqliteRagRepository, storage_error};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqliteRagCorpusPlan {
    pub corpus_id: String,
    pub embedded_source: bool,
    pub text_unit_sqlite: bool,
    pub fulltext_sqlite: bool,
    pub vector_sqlite: bool,
    pub source_table: Option<String>,
    pub text_unit_table: String,
    pub fulltext_table: Option<String>,
    pub vector_table: Option<String>,
    pub diagnostics_table: Option<String>,
    pub source_columns: BTreeSet<String>,
    pub text_unit_columns: BTreeSet<String>,
    pub fulltext_columns: BTreeSet<String>,
    pub vector_columns: BTreeSet<String>,
    pub field_mapping: BTreeMap<String, String>,
    pub source_field_mapping: BTreeMap<String, String>,
    pub text_unit_field_mapping: BTreeMap<String, String>,
    pub filter_columns: BTreeMap<String, SqliteRagFilterColumn>,
    pub return_field_columns: BTreeMap<String, SqliteRagProjectionColumn>,
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
    pub fulltext_updated_at_column: Option<String>,
    pub vector_updated_at_column: Option<String>,
    pub text_unit_unique_key: Vec<String>,
    pub fulltext_unique_key: Vec<String>,
    pub vector_unique_key: Vec<String>,
}

impl SqliteRagCorpusPlan {
    pub(crate) fn has_distinct_source_table(&self) -> bool {
        self.source_table.is_some()
    }

    pub(crate) fn has_distinct_vector_table(&self) -> bool {
        self.vector_sqlite && self.vector_table.is_some()
    }

    pub(crate) fn has_distinct_fulltext_table(&self) -> bool {
        self.fulltext_sqlite && self.fulltext_table.is_some()
    }

    pub(crate) fn has_sqlite_fulltext(&self) -> bool {
        self.fulltext_sqlite
    }

    pub(crate) fn has_sqlite_vector(&self) -> bool {
        self.vector_sqlite
    }

    pub(crate) fn has_sqlite_text_units(&self) -> bool {
        self.text_unit_sqlite
    }

    pub(crate) fn stores_fulltext_carrier_rows(&self) -> bool {
        self.fulltext_sqlite && (!self.text_unit_sqlite || self.fulltext_table.is_some())
    }

    pub(crate) fn stores_vector_carrier_rows(&self) -> bool {
        self.vector_sqlite && (!self.text_unit_sqlite || self.vector_table.is_some())
    }

    pub(crate) fn source_storage_table(&self) -> &str {
        self.source_table
            .as_deref()
            .unwrap_or(self.text_unit_table.as_str())
    }

    pub(crate) fn vector_storage_table(&self) -> &str {
        self.vector_table
            .as_deref()
            .unwrap_or(self.text_unit_table.as_str())
    }

    pub(crate) fn fulltext_storage_table(&self) -> &str {
        self.fulltext_table
            .as_deref()
            .unwrap_or(self.text_unit_table.as_str())
    }
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
pub(crate) struct SqliteRagProjectionColumn {
    pub logical_name: String,
    pub column: String,
    pub alias: SqliteRagTableAlias,
    pub value_type: SqliteRagValueType,
}

pub(crate) fn compile_sql_corpus_plan(
    repository: &SqliteRagRepository,
    corpus: &RagCorpusRecord,
) -> Result<SqliteRagCorpusPlan, RagError> {
    let binding = &corpus.storage_binding;
    repository.ensure_sqlite_runtime_storage_for_corpus(corpus)?;
    let configured_text_unit_table = binding
        .sqlite_table_for_segment("textUnit")
        .map(ToOwned::to_owned);
    let source_table = binding
        .sqlite_table_for_segment("source")
        .map(ToOwned::to_owned);
    let fulltext_sqlite = binding.sqlite_table_for_segment("fulltext").is_some();
    let configured_fulltext_table = binding
        .sqlite_table_for_segment("fulltext")
        .map(ToOwned::to_owned);
    let vector_sqlite = binding.sqlite_table_for_segment("vector").is_some();
    let configured_vector_table = binding
        .sqlite_table_for_segment("vector")
        .map(ToOwned::to_owned);
    let text_unit_sqlite = configured_text_unit_table.is_some();
    let text_unit_table = configured_text_unit_table
        .clone()
        .or_else(|| configured_fulltext_table.clone())
        .or_else(|| configured_vector_table.clone())
        .ok_or(RagError::StorageBindingInvalid)?;
    let embedded_source = source_table.as_deref() == Some(text_unit_table.as_str());
    let source_table = source_table.filter(|table| table != &text_unit_table);
    let fulltext_table = configured_fulltext_table.filter(|table| table != &text_unit_table);
    let vector_table = configured_vector_table.filter(|table| table != &text_unit_table);
    validate_identifier(&text_unit_table)?;
    if let Some(table) = &source_table {
        validate_identifier(table)?;
    }
    if let Some(table) = &fulltext_table {
        validate_identifier(table)?;
    }
    if let Some(table) = &vector_table {
        validate_identifier(table)?;
    }
    if let Some(table) = binding.diagnostics_table() {
        validate_identifier(table)?;
    }

    let physical_source_columns = match &source_table {
        Some(table) => repository.table_columns(table)?,
        None => BTreeSet::new(),
    };
    let text_unit_columns = repository.table_columns(&text_unit_table)?;
    let fulltext_columns = match &fulltext_table {
        Some(table) => repository.table_columns(table)?,
        None => text_unit_columns.clone(),
    };
    let vector_columns = match &vector_table {
        Some(table) => repository.table_columns(table)?,
        None => text_unit_columns.clone(),
    };
    let source_columns = if source_table.is_some() {
        physical_source_columns
    } else {
        text_unit_columns.clone()
    };
    if fulltext_table.is_some() {
        ensure_required_columns(
            &fulltext_columns,
            &[
                "text_unit_code",
                "source_code",
                "source_version",
                "scope_type",
                "scope_code",
                "ordinal",
                "status",
                "search_text",
                "updated_at",
            ],
        )?;
    }
    if vector_table.is_some() {
        ensure_required_columns(
            &vector_columns,
            &[
                "text_unit_code",
                "source_code",
                "source_version",
                "scope_type",
                "scope_code",
                "ordinal",
                "updated_at",
            ],
        )?;
    }
    let filter_columns = compile_filter_columns(
        corpus,
        source_table.is_some(),
        &source_columns,
        &text_unit_columns,
    );
    let return_field_columns =
        compile_return_field_columns(corpus, &source_columns, &text_unit_columns, &filter_columns);

    let source_status_column = if source_table.is_some() {
        first_existing(&source_columns, &["source_status", "status"])
    } else {
        first_existing(&text_unit_columns, &["source_status", "status"])
    };

    let text_unit_unique_key = text_unit_unique_key(repository, &text_unit_table)?;
    let fulltext_unique_key = vector_unique_key(
        repository,
        fulltext_table
            .as_deref()
            .unwrap_or(text_unit_table.as_str()),
        &fulltext_columns,
    )?;
    let vector_unique_key = vector_unique_key(
        repository,
        vector_table.as_deref().unwrap_or(text_unit_table.as_str()),
        &vector_columns,
    )?;

    Ok(SqliteRagCorpusPlan {
        corpus_id: corpus.corpus_id.clone(),
        embedded_source,
        text_unit_sqlite,
        fulltext_sqlite,
        vector_sqlite,
        source_table,
        text_unit_table,
        fulltext_table,
        vector_table,
        diagnostics_table: binding.diagnostics_table().map(ToOwned::to_owned),
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
        fulltext_updated_at_column: first_existing(&fulltext_columns, &["updated_at"]),
        vector_updated_at_column: first_existing(&vector_columns, &["updated_at"]),
        text_unit_unique_key,
        fulltext_unique_key,
        vector_unique_key,
        field_mapping: corpus.field_mapping.clone(),
        source_field_mapping: corpus.source_field_mapping.clone(),
        text_unit_field_mapping: corpus.text_unit_field_mapping.clone(),
        source_columns,
        text_unit_columns,
        fulltext_columns,
        vector_columns,
        filter_columns,
        return_field_columns,
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

fn compile_return_field_columns(
    corpus: &RagCorpusRecord,
    source_columns: &BTreeSet<String>,
    text_unit_columns: &BTreeSet<String>,
    filter_columns: &BTreeMap<String, SqliteRagFilterColumn>,
) -> BTreeMap<String, SqliteRagProjectionColumn> {
    let mut return_columns = BTreeMap::<String, SqliteRagProjectionColumn>::new();
    let source_mapping = if corpus.source_field_mapping.is_empty() {
        &corpus.field_mapping
    } else {
        &corpus.source_field_mapping
    };
    for (logical_name, column) in source_mapping {
        if source_columns.contains(column) && !is_return_field_reserved(logical_name) {
            let value_type = filter_columns
                .get(logical_name)
                .map(|field| field.value_type)
                .unwrap_or(SqliteRagValueType::String);
            return_columns.insert(
                logical_name.clone(),
                SqliteRagProjectionColumn {
                    logical_name: logical_name.clone(),
                    column: column.clone(),
                    alias: SqliteRagTableAlias::Source,
                    value_type,
                },
            );
        }
    }
    for (logical_name, column) in &corpus.text_unit_field_mapping {
        if text_unit_columns.contains(column) && !is_return_field_reserved(logical_name) {
            let value_type = filter_columns
                .get(logical_name)
                .map(|field| field.value_type)
                .unwrap_or(SqliteRagValueType::String);
            return_columns
                .entry(logical_name.clone())
                .or_insert_with(|| SqliteRagProjectionColumn {
                    logical_name: logical_name.clone(),
                    column: column.clone(),
                    alias: SqliteRagTableAlias::TextUnit,
                    value_type,
                });
        }
    }
    for (logical_name, filter_column) in filter_columns {
        let physical_columns = match filter_column.alias {
            SqliteRagTableAlias::Source => source_columns,
            SqliteRagTableAlias::TextUnit => text_unit_columns,
        };
        if physical_columns.contains(&filter_column.column)
            && !is_return_field_reserved(logical_name)
        {
            return_columns
                .entry(logical_name.clone())
                .or_insert_with(|| SqliteRagProjectionColumn {
                    logical_name: logical_name.clone(),
                    column: filter_column.column.clone(),
                    alias: filter_column.alias,
                    value_type: filter_column.value_type,
                });
        }
    }
    return_columns
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

fn ensure_required_columns(columns: &BTreeSet<String>, required: &[&str]) -> Result<(), RagError> {
    if required.iter().all(|column| columns.contains(*column)) {
        Ok(())
    } else {
        Err(RagError::StorageBindingInvalid)
    }
}

