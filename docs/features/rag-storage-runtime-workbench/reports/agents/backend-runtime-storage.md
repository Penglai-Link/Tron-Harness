# Backend Runtime Storage Agent Report

## Scope

- Agent: backend-worker
- Branch: `feature/xinlangtao/rag-storage-runtime-workbench/backend/runtime-storage`
- AC rows: AC-RSW-01, AC-RSW-02, AC-RSW-03, AC-RSW-04

## AC-RSW-01

- Changed paths:
  - `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/index_schema.rs`
  - `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
  - `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/sql_plan.rs`
  - `tron-backend/tests/rag_management_api.rs`
- RED:
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api corpus_management_create_and_update_persist_topology_and_runtime_projection -- --nocapture`
  - Failed as expected: `rag_team_playbook_sources should include column id; actual columns: []`.
- GREEN:
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api corpus_management_create_and_update_persist_topology_and_runtime_projection -- --nocapture`
  - Passed after corpus-aware SQLite runtime materialization created source/textUnit main tables, independent fulltext/vector tables, diagnostics tables, and mapping/filter columns.
- VERIFY:
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api -- --nocapture`
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts -- --nocapture`
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts -- --nocapture`

## AC-RSW-02

- Changed paths:
  - `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/index_schema.rs`
  - `docs/features/rag-storage-runtime-workbench/reports/agents/backend-runtime-storage.md`
- Decision:
  - Graph-only runtime storage remains an explicit no-op in SQLite materialization when no SQLite runtime table is projected.
  - Mixed Graph projections that include SQLite helper tables still materialize only those SQLite tables.
  - This preserves the existing lazy graph initialization behavior and avoids destructive or fake graph DDL for seeded `skill-graph`.
- GREEN/VERIFY:
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api -- --nocapture`
  - Existing graph closure tests continue to pass.

## AC-RSW-03

- Changed paths:
  - `tron-backend/tests/rag_management_api.rs`
- RED:
  - Added a saved ingest profile -> `/api/v1/rag/ingest` runtime test.
  - Intermediate failures proved the test reached runtime execution: missing stage returned `RAG_INGEST_STAGE_FAILED`; non-degradable embedding returned `RAG_EMBEDDING_FAILED`.
- GREEN:
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api profile_management_saved_ingest_profile_is_loaded_by_ingest_api -- --nocapture`
  - Passed with a full saved ingest DAG and `SKIP_VECTOR_ON_UNAVAILABLE`, proving the ingest API loads and compiles the saved profile.
- VERIFY:
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_api -- --nocapture`
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts -- --nocapture`

## AC-RSW-04

- Changed paths:
  - `tron-backend/tests/rag_management_api.rs`
- RED/GREEN:
  - Extended profile management save coverage to call `/api/v1/rag/search` with the saved retrieval profile `docs-memory-fusion-v1`.
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api -- --nocapture`
  - Passed, proving the search API loads and compiles the saved retrieval profile.
- VERIFY:
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_api -- --nocapture`
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts -- --nocapture`

## Command Log

- Baseline:
  - `make be-check` passed before changes.
- RED:
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api corpus_management_create_and_update_persist_topology_and_runtime_projection -- --nocapture` failed with missing runtime storage table columns.
- GREEN:
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api corpus_management_create_and_update_persist_topology_and_runtime_projection -- --nocapture` passed.
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api profile_management_saved_ingest_profile_is_loaded_by_ingest_api -- --nocapture` passed.
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api -- --nocapture` passed.
- VERIFY:
  - `make be-check` passed after changes.
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_api -- --nocapture` passed.
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_api -- --nocapture` passed.
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts -- --nocapture` passed.
  - `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts -- --nocapture` passed.

## Blockers

- None.

## Residual Risks

- SQLite materialization is intentionally additive only. Existing tables with incompatible primary keys or uniqueness cannot be destructively repaired by this AC slice.
- Graph storage remains lazy/no-op for physical schema creation; follow-up graph runtime work should add graph-specific materialization only when the graph adapter exposes a safe schema contract.
