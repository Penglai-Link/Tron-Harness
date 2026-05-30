# Database Schema Seed Report

## Scope

- Agent: database
- Subtask: schema-seed
- Branch: `feature/langtaoxin/rag-corpus-storage-profile-foundation/database/schema-seed`

## AC Evidence

### AC-RAG-01

- Implemented `20260530_0022_create_rag_foundation.sql`.
- Creates the 7 approved tables:
  - `rag_corpora`
  - `rag_profiles`
  - `rag_memory_sources`
  - `rag_project_doc_sources`
  - `rag_memory_text_units`
  - `rag_project_doc_text_units`
  - `rag_diagnostics`
- Idempotently seeds Corpus rows:
  - `memory-card` as `enabled`
  - `project-doc` as `enabled`
  - `code-graph` as `reserved`
- Proof test: `rag_foundation_migration_creates_approved_tables_and_seeds_corpora_idempotently`.

### AC-RAG-02

- Source tables stay split by Corpus:
  - `rag_memory_sources`
  - `rag_project_doc_sources`
- TextUnit tables stay split by Corpus:
  - `rag_memory_text_units`
  - `rag_project_doc_text_units`
- No unified `rag_sources`, unified `rag_text_units`, standalone fulltext tables, or standalone vector tables are created.
- Fulltext and vector fields live inside each TextUnit table:
  - `search_text`
  - `fulltext_status`
  - `embedding_model_code`
  - `embedding_version`
  - `embedding_text_hash`
  - `embedding_dimension`
  - `embedding`
  - `embedding_status`
- Proof test: `rag_foundation_keeps_source_and_text_unit_storage_split_by_corpus`.

### AC-RAG-08

- Source and TextUnit tables preserve `scope_type` and `scope_code`.
- Added the minimal `RagScopeFields` row-shape type for database-facing model evidence.
- No permission columns or permission behavior were added.
- Proof test: `rag_foundation_preserves_scope_isolation_columns_without_permission_columns`.

## Changed Paths

- `tron-backend/migrations/20260530_0022_create_rag_foundation.sql`
- `tron-backend/src/adapters/outbound/persistence/sqlite.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
- `tron-backend/src/domain.rs`
- `tron-backend/src/domain/rag.rs`
- `tron-backend/tests/rag_foundation_schema.rs`
- `docs/features/rag-corpus-storage-profile-foundation/reports/agents/database-schema-seed.md`

## Commands

- RED: `make be-test`
- Format: `cargo fmt --manifest-path tron-backend/Cargo.toml`
- GREEN: `make be-test`

## RED Summary

- `make be-test` failed before implementation with `E0432`.
- Failing unit: `tests/rag_foundation_schema.rs`.
- Missing symbol: `tron_backend::adapters::outbound::persistence::SqliteRagRepository`.

## GREEN Summary

- `make be-test` passed.
- RAG proof file passed: `tests/rag_foundation_schema.rs`, 3 tests.
- Full backend suite passed, including doc tests.

## Residual Risks

- This subtask intentionally does not implement ingest, search, HTTP routes, LLM runtime calls, GraphService calls, or EventBus calls.
- The RAG repository only exposes schema and seed inspection support needed by this database foundation task.
