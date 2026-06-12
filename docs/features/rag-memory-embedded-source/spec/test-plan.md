# Test Plan

## Command Index

- CMD-1: `make be-check`
- CMD-2: `make be-test`

## Proof Rows

- AC-1: `rag_service_contracts::rag_service_resolves_enabled_corpus_storage_binding_from_repository_port`；CMD-2。
- AC-2: `rag_ingest_contracts::memory_plain_text_ingest_writes_source_text_unit_fulltext_and_vector_fields`；CMD-2。
- AC-3: `rag_search_contracts`、`rag_diagnostics_contracts`、`rag_diagnostics_api`；CMD-2。
- AC-4: `rag_foundation_schema::rag_foundation_migration_creates_approved_tables_and_seeds_corpora_idempotently` 与 `rag_domain_fields_migration_adds_memory_governance_columns_and_indexes_idempotently`；CMD-2。
- AC-5: `rag_ingest_contracts::project_doc_markdown_ingest_persists_document_fields_and_heading_split_units` 与 `rag_search_contracts::project_doc_search_hydrates_heading_path_and_document_metadata`；CMD-2。
