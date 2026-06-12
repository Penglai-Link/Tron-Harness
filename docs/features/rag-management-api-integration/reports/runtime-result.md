# Runtime Result: RAG Role-Aware Storage

## Summary

- status: complete
- execution_owner: backend-runtime-worker
- branch: feature/xinlangtao/rag-frontend-release-0.1.0/backend/role-aware-runtime-storage
- result: role-aware storage carriers are implemented and verified for independent SQLite fulltext, legacy fallback, mixed Graph carrier search with SQLite canonical hydration, and existing vector/graph regressions.

## Evidence Rows

- AC-RAGCFG-12 | red: focused ingest/search tests failed because `rag_project_doc_fulltext` did not exist and management `fulltextTable` projected as null. | green: integrated base tests passed for management `fulltextTable`, SQLite fulltext carrier ingest, FTS search hydrate, diagnostics status, and retry rebuild. | browser: clicks=not_applicable_backend_only; inputs=independent fulltext target, legacy missing-fulltext payload, ingest/search/retry requests; network=not_applicable_backend_suite; feedback=fulltext carrier rows, FTS sidecar, and text-unit hydrate assertions passed; artifacts=tron-backend/tests/rag_management_api.rs, tron-backend/tests/rag_ingest_contracts.rs, tron-backend/tests/rag_search_contracts.rs, tron-backend/tests/rag_diagnostics_contracts.rs | command_ref: CMD-BE-RAG-ROLE | status: done.
- AC-RAGCFG-13 | red: mixed Graph carrier behavior initially lacked canonical SQLite hydrate proof and Graph-backed dispatch was whole-corpus oriented. | green: integrated base `rag_skill_graph_contracts` passed with `mixed_graph_carrier_search_hydrates_sqlite_canonical_text_units`, proving Graph fulltext carrier recall returns SQLite canonical text. | browser: clicks=not_applicable_backend_only; inputs=Graph carrier corpus with SQLite `textUnitTable`, graph node carrier-only text mutation, graph fulltext query; network=not_applicable_backend_suite; feedback=search matched carrier text while returned `text` came from SQLite canonical table; artifacts=tron-backend/tests/rag_skill_graph_contracts.rs, tron-backend/src/adapters/outbound/persistence/rag_graph_repository.rs, tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/search.rs | command_ref: CMD-BE-RAG-MIXED | status: done.
- AC-RAGCFG-15 | red: role-aware changes risked regressing split vector, embedded SQLite, and legacy graph single-node behavior. | green: integrated base regressions passed for full graph contracts, distinct vector ingest/search/retry, distinct fulltext ingest/search/retry, and frontend RAG form flows. | browser: clicks=not_applicable_backend_only; inputs=legacy graph, split SQLite vectorTable, split SQLite fulltextTable, and frontend storage form flows; network=not_applicable_backend_suite; feedback=focused regression matrix passed; artifacts=tron-backend/tests/rag_skill_graph_contracts.rs, tron-backend/tests/rag_search_contracts.rs, tron-backend/tests/rag_ingest_contracts.rs, tron-backend/tests/rag_diagnostics_contracts.rs | command_ref: CMD-BE-RUNTIME | status: done.

## Commands

- CMD-BE-RAG-ROLE | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api fulltext && cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts project_doc_ingest_populates_distinct_fulltext_storage_when_configured && cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts project_doc_fulltext_route_reads_distinct_carrier_and_hydrates_text_units && cargo test --manifest-path tron-backend/Cargo.toml --test rag_diagnostics_contracts retry_rebuilds_fulltext_and_vector_from_persisted_text_units` | result: passed | notes: management, ingest, search, and retry/debug fulltext role tests passed.
- CMD-BE-RAG-MIXED | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts` | result: passed | notes: 8 graph tests passed, including mixed Graph carrier search hydrating SQLite canonical text units.
- CMD-BE-RUNTIME | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts project_doc_ingest_populates_distinct_vector_storage_when_configured && cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts project_doc_vector_route_reads_distinct_vector_storage_table_when_configured && cargo test --manifest-path tron-backend/Cargo.toml --test rag_diagnostics_contracts retry_vector_rebuild_supports_distinct_vector_storage_table` | result: passed | notes: distinct vector storage regressions remained green.

## Changed Paths

- `tron-backend/src/adapters/outbound/persistence/rag_graph_repository.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/diagnostics.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/index_schema.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/indexes.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/ingest.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/retry.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/search.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/sql_plan.rs`
- `tron-backend/src/application/rag/management.rs`
- `tron-backend/src/domain/rag/model.rs`
- `tron-backend/tests/rag_management_api.rs`
- `tron-backend/tests/rag_ingest_contracts.rs`
- `tron-backend/tests/rag_search_contracts.rs`
- `tron-backend/tests/rag_diagnostics_contracts.rs`
- `tron-backend/tests/rag_skill_graph_contracts.rs`

## Blockers

- None.
