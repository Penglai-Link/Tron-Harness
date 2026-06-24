use std::collections::BTreeSet;

use rusqlite::Connection;

use crate::domain::rag::RagError;

use super::{dynamic_columns_sql, ensure_missing_text_columns, quote_identifier, storage_error};

pub(super) fn ensure_tree_storage_table(
    connection: &Connection,
    table_name: &str,
) -> Result<(), RagError> {
    let columns = tree_columns();
    let table = quote_identifier(table_name)?;
    let source_index = quote_identifier(&format!("idx_{table_name}_source"))?;
    let parent_index = quote_identifier(&format!("idx_{table_name}_parent"))?;
    let code_index = quote_identifier(&format!("idx_{table_name}_code"))?;
    let path_index = quote_identifier(&format!("idx_{table_name}_path"))?;
    let column_sql = dynamic_columns_sql(&columns);
    connection
        .execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS {table} (
                id INTEGER PRIMARY KEY,
                {column_sql},
                UNIQUE (source_code, source_version, scope_type, scope_code, tree_node_code),
                UNIQUE (source_code, source_version, scope_type, scope_code, path_key)
            );
            CREATE INDEX IF NOT EXISTS {source_index}
                ON {table} (source_code, source_version, scope_type, scope_code, ordinal);
            CREATE INDEX IF NOT EXISTS {code_index}
                ON {table} (tree_node_code);
            CREATE INDEX IF NOT EXISTS {parent_index}
                ON {table} (source_code, source_version, scope_type, scope_code, parent_node_code, ordinal);
            CREATE INDEX IF NOT EXISTS {path_index}
                ON {table} (source_code, source_version, scope_type, scope_code, path_key);"
        ))
        .map_err(storage_error)?;
    ensure_missing_text_columns(connection, table_name, &columns)
}

fn tree_columns() -> BTreeSet<String> {
    BTreeSet::from([
        "tree_node_code".to_string(),
        "parent_node_code".to_string(),
        "source_code".to_string(),
        "source_version".to_string(),
        "scope_type".to_string(),
        "scope_code".to_string(),
        "path_key".to_string(),
        "level".to_string(),
        "ordinal".to_string(),
        "title".to_string(),
        "summary_text".to_string(),
        "summary_token_count".to_string(),
        "summary_content_hash".to_string(),
        "summary_fulltext_status".to_string(),
        "summary_embedding_status".to_string(),
        "summary_embedding_model_code".to_string(),
        "summary_embedding_version".to_string(),
        "summary_embedding_text_hash".to_string(),
        "summary_embedding_dimension".to_string(),
        "summary_embedding".to_string(),
        "covered_text_unit_codes".to_string(),
        "covered_text_unit_count".to_string(),
        "child_start_ordinal".to_string(),
        "child_end_ordinal".to_string(),
        "metadata".to_string(),
        "status".to_string(),
        "created_at".to_string(),
        "updated_at".to_string(),
    ])
}
