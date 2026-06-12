# Backend Result

## Summary

Backend worker `feature/xinlangtao/rag-storage-runtime-workbench/backend/runtime-storage` completed runtime storage materialization and saved profile runtime compatibility. The branch was merged into base with `b26bb666 merge: backend rag runtime storage`.

## Evidence Rows

- AC-RSW-01 | red: `rag_management_api corpus_management_create_and_update_persist_topology_and_runtime_projection` failed before materialization with missing runtime table columns | green: same test passed after corpus-aware SQLite runtime tables, FTS/vector carrier tables, diagnostics tables, mappings, and filter columns were created | verify: `make be-check` and focused RAG backend tests passed | command_ref: CMD-RAG-MGMT-TEST | status: passed.
- AC-RSW-02 | red: graph/table split projection behavior was ambiguous before the backend slice | green: graph-only storage remains delegated to graph runtime while SQLite projected tables materialize additively; graph closure tests remain valid | verify: `rag_management_api` and RAG contract suites passed | command_ref: CMD-RAG-MGMT-TEST | status: passed.
- AC-RSW-03 | red: saved ingest profile runtime test initially reached runtime failures (`RAG_INGEST_STAGE_FAILED`, then non-degradable embedding failure) | green: saved ingest profile with full DAG and `SKIP_VECTOR_ON_UNAVAILABLE` drove POST `/api/v1/rag/ingest` successfully | verify: `rag_ingest_api` and `rag_ingest_contracts` passed | command_ref: CMD-RAG-E2E-TEST | status: passed.
- AC-RSW-04 | red: saved retrieval profile runtime compatibility needed HTTP search coverage | green: saved retrieval profile `docs-memory-fusion-v1` drove POST `/api/v1/rag/search` successfully | verify: `rag_search_api` and `rag_search_contracts` passed | command_ref: CMD-RAG-E2E-TEST | status: passed.

## Changed Paths

- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/index_schema.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/sql_plan.rs`
- `tron-backend/tests/rag_management_api.rs`
- `docs/features/rag-storage-runtime-workbench/reports/agents/backend-runtime-storage.md`

## Commands

- RED: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api corpus_management_create_and_update_persist_topology_and_runtime_projection -- --nocapture` failed with missing runtime storage table columns.
- GREEN: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api corpus_management_create_and_update_persist_topology_and_runtime_projection -- --nocapture` passed.
- GREEN: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api profile_management_saved_ingest_profile_is_loaded_by_ingest_api -- --nocapture` passed.
- VERIFY: `make be-check` passed.
- VERIFY: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api --test rag_ingest_api --test rag_search_api --test rag_ingest_contracts --test rag_search_contracts` passed.

## Blockers

- None.

## Residual Risks

- SQLite materialization is additive only. Existing incompatible table definitions are not destructively repaired by this AC slice.
- Graph physical schema creation remains delegated to the graph runtime path; this slice does not add graph-specific DDL/materialization APIs.
