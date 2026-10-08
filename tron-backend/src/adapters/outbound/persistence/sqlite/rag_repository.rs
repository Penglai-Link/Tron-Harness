mod diagnostics;
mod field_mapping;
mod index_schema;
mod indexes;
mod ingest;
mod ingest_values;
mod lifecycle;
mod operations;

mod search;
mod sql_plan;

include!("rag_repository/root/repository.rs");
include!("rag_repository/root/runtime_cleanup.rs");
include!("rag_repository/root/row_mapping.rs");

pub(crate) fn vector_index_tables_for_cleanup(
    connection: &rusqlite::Connection,
    table: &str,
) -> Result<Vec<String>, crate::domain::rag::RagError> {
    indexes::vector_index_tables(connection, table)
}
