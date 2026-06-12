# Backend Result

## Summary

Implemented the requested backend write-planner evolution in the active release base worktree.

- Phase 1: added `RagStorageWritePlan`, compiled from `storageBinding.segmentBindings` and `segmentTargets`.
- Phase 2: added SQLite `persist_source_only`, so `source=SQLITE` with `textUnit/fulltext/vector=GRAPH` writes both media without requiring a SQLite text-unit/search table.
- Phase 3: added `RagStorageWriteAdapterRegistry`, which dispatches backend adapters from the compiled plan.
- Phase 4: generalized write-plan operations from fixed backend mode names into concrete `segment -> target -> backend` operations; SQLite and Graph adapters now own their own write-combination branching.

## AC Status

- AC-1: Pass. Graph-backed ingest now compiles a write plan before persistence dispatch.
- AC-2: Pass. Focused compatibility tests for SQLite carrier, Graph segmented, fulltext carrier hydration, and vector carrier search pass.
- AC-3: Pass. New source-only SQLite plus Graph retrieval test passes.
- AC-4: Pass. Source-only SQLite uses a dedicated source plan and does not compile the full SQLite retrieval plan.
- AC-5: Pass. RED/GREEN/VERIFY evidence is recorded below.
- AC-6: Pass. `RagStorageWritePlan` groups operations by concrete target id and carries target metadata.
- AC-7: Pass. Backend-specific branching is implemented in SQLite/Graph adapters, not in plan mode names.
- AC-8: Pass. Focused compatibility tests and full RAG ingest/search/graph integration suites pass.
- AC-9: Pass. RED/GREEN/VERIFY evidence for the generic planner refactor is recorded below.

## Evidence Ledger

- RED: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts graph_text_units_can_use_sqlite_source_with_graph_retrieval_segments -- --nocapture` initially failed with `no such table: rag_skill_graph_sources`.
- GREEN: CMD-3 passed after adding `RagStorageWritePlan`, `RagStorageWriteAdapterRegistry`, and SQLite `persist_source_only`.
- GREEN: CMD-4 passed, protecting SQLite fulltext carrier without SQLite textUnit.
- GREEN: CMD-5 passed, protecting fulltext distinct carrier hydration.
- GREEN: CMD-6 passed, protecting vector distinct storage-table search.
- GREEN: CMD-7 passed, protecting Graph segmented write/search.
- VERIFY: CMD-2 `make be-check` passed.
- RED: CMD-8 initially failed before implementation because `RagStorageWriteOperation` had no `target_id` or `target` fields, proving the old mode-based plan could not expose concrete target operations.
- GREEN: CMD-8 passed after `RagStorageWritePlan` grouped operations by concrete `segmentTargets` and preserved fulltext fallback to `textUnit`.
- GREEN: CMD-3 passed after SQLite adapter-owned branching preserved source-only SQLite plus Graph retrieval writes.
- GREEN: CMD-4 passed after SQLite adapter-owned branching preserved SQLite fulltext carrier without SQLite textUnit.
- GREEN: CMD-7 passed after Graph adapter-owned branching preserved segmented Graph writes.
- GREEN: CMD-5 and CMD-6 passed after the generic planner refactor, preserving distinct fulltext/vector carrier search and textUnit hydration.
- GREEN: CMD-10 passed with 12/12 `rag_skill_graph_contracts` tests.
- GREEN: CMD-11 passed with 30/30 `rag_search_contracts` tests.
- GREEN: CMD-12 passed with 13/13 `rag_ingest_contracts` tests.
- VERIFY: CMD-9 `make be-clean` passed before the final serial verification run, recovering from an earlier parallel cargo artifact race.
- VERIFY: CMD-2 `make be-check` passed after cleanup.
- VERIFY: CMD-13 returned no matches, confirming `RagStorageWriteMode`, `SqliteRetrieval`, `SqliteSourceOnly`, `GraphSingleNode`, and `GraphSegmented` are absent from `rag_graph_repository` runtime code.
- VERIFY: CMD-1 `make be-test` ran and failed outside RAG scope: 107 passed, 7 failed. Failures were `EncryptionNotConfigured` in auth/MCP variable tests and MCP stdio timeout assertions.

## Subagent Review

Read-only explorer `019eb4f7-2c94-7e12-acec-53d9c495748f` reviewed the boundary and confirmed the same implementation shape:

- Keep full SQLite search plan strict.
- Add source-only SQLite write path instead of broadening retrieval plan.
- Do not let source-only SQLite trigger Graph search hydration from SQLite textUnit.
- Reuse existing Graph single-node and segmented writes behind a planner/adapter dispatch boundary.

## Isolation Note

The selected base worktree already carried uncommitted RAG release baseline changes. Per the worktree plan, implementation stayed in the requested base worktree as a controlled isolation deviation so the new work would build against the actual active baseline.
