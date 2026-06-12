# Test Plan

## Command Index

- CMD-1: `make be-test`
- CMD-2: `make be-check`
- CMD-3: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts graph_text_units_can_use_sqlite_source_with_graph_retrieval_segments -- --nocapture`
- CMD-4: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts graph_text_units_can_use_sqlite_fulltext_carrier_without_sqlite_text_units -- --nocapture`
- CMD-5: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts project_doc_fulltext_route_reads_distinct_carrier_and_hydrates_text_units -- --nocapture`
- CMD-6: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts project_doc_vector_route_reads_distinct_vector_storage_table_when_configured -- --nocapture`
- CMD-7: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts segmented_graph_runtime_writes_and_searches_role_specific_nodes -- --nocapture`
- CMD-8: `cargo test --manifest-path tron-backend/Cargo.toml write_plan -- --nocapture`
- CMD-9: `make be-clean`
- CMD-10: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts -- --nocapture`
- CMD-11: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts -- --nocapture`
- CMD-12: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts -- --nocapture`
- CMD-13: `rg "RagStorageWriteMode|SqliteRetrieval|SqliteSourceOnly|GraphSingleNode|GraphSegmented" tron-backend/src/adapters/outbound/persistence/rag_graph_repository`

## Proof Rows

- AC-1: case: planner compiles backend operations from segment bindings; red: existing GraphBacked hard-coded branch has no planner model; green: planner unit/contract coverage passes; command_ref: CMD-2.
- AC-2: case: existing focused RAG runtime tests still pass; red: any regression in carrier/segmented tests; green: focused tests pass; command_ref: CMD-4, CMD-5, CMD-6, CMD-7.
- AC-3: case: source-only SQLite plus Graph retrieval segments persists both media; red: source row absent or ingest fails; green: source row exists and graph hit exists; command_ref: CMD-3.
- AC-4: case: SQLite and Graph adapter dispatch are separate paths; red: source-only path compiles full SQL retrieval plan; green: source-only write uses adapter path without text-unit plan; command_ref: CMD-2, CMD-3.
- AC-5: case: evidence report contains RED/GREEN/VERIFY rows; red: missing command outputs; green: report updated; command_ref: CMD-1 through CMD-7.
- AC-6: case: planner groups operations by concrete storage target and exposes target metadata; red: mode-based operation has no target id; green: write plan unit tests pass; command_ref: CMD-8.
- AC-7: case: backend adapter owns SQLite source-only/retrieval and Graph single/segmented branching; red: plan carries backend-specific mode names; green: adapter tests and focused runtime tests pass and old mode names are absent from runtime code; command_ref: CMD-2, CMD-3, CMD-7, CMD-8, CMD-13.
- AC-8: case: existing focused RAG runtime tests still pass after generic planner refactor; red: any compatibility regression; green: focused tests and full RAG ingest/search/graph suites pass; command_ref: CMD-3 through CMD-7, CMD-10 through CMD-12.
- AC-9: case: evidence report contains RED/GREEN/VERIFY rows for generic planner refactor; red: missing command outputs; green: report updated; command_ref: CMD-1 through CMD-13.
