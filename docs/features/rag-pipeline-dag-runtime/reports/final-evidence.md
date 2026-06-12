# Final Evidence: RAG Pipeline DAG Runtime

## Summary

- status: complete

## AC Evidence Matrix

| AC | status | evidence |
| --- | --- | --- |
| AC-RPD-01 | verified | Pipeline catalog registry and endpoint covered by backend report and `rag_management_api`. |
| AC-RPD-02 | verified | Profile DAG validation and persistence covered by backend report and `rag_management_api`. |
| AC-RPD-03 | verified | Ingest request compatibility plus additive `stageResults` covered by ingest contract tests. |
| AC-RPD-04 | verified | Search request compatibility plus additive `stageResults` covered by search contract tests. |
| AC-RPD-05 | verified | DAG branch, merge, and stage-owned artifact keys covered by pipeline unit tests. |
| AC-RPD-06 | verified | Catalog-backed profile builder covered by Playwright profile form tests. |
| AC-RPD-07 | verified | Workbench stage result display covered by Playwright workbench test. |
| AC-RPD-08 | verified | Integrated focused frontend and backend regressions passed; full backend Makefile failure is isolated to known non-RAG environment tests. |
| AC-RPD-09 | verified | Compiler rejects missing input artifacts and non-repeatable duplicate stages and preserves branch/merge execution levels; focused RAG lib regression passed 15/15 after Phase 3. |
| AC-RPD-10 | verified | Ingest accepts pipeline-only profiles, runs through pipeline levels/artifact store, and no longer depends on fixed `match stage_id.as_str()` dispatch; `rag_ingest_contracts` passed 16/16 after Phase 3. |
| AC-RPD-11 | verified | Retrieval accepts pipeline-only profiles and runs search stages through pipeline execution with typed artifacts for route/fuse/dedup/context data; `rag_search_contracts` passed 33/33 after Phase 3. |
| AC-RPD-12 | verified | DAG stage results now come from actual executed nodes; blank-query structured-filter search reports only executed route stages, and branch/merge artifact checks are covered by pipeline tests. |
| AC-RPD-13 | verified | Integration attempted `make be-test` first and isolated remaining failures to non-RAG auth/MCP tests; focused RAG regression passed. |
| AC-RPD-14 | verified | Pipeline catalog and compiler expose `EXTRACT_TREE`, `PERSIST_TREE`, `QUESTION_GENERALIZATION`, and `QUERY_REWRITE` with stage-owned artifact keys; `CMD-BE-PHASE3-RAG` passed 5/5 after RED. |
| AC-RPD-15 | verified | Ingest DAG executes `EXTRACT_TREE` and `PERSIST_TREE`, generates `graph.related`, and hands it to persistence without request `relatedGraph`; `rag_ingest_contracts` passed 16/16 after RED. |
| AC-RPD-16 | verified | Retrieval DAG executes `QUESTION_GENERALIZATION` and `QUERY_REWRITE` before route search, and route search/vector embedding/ranking/rerank use effective query artifacts while preserving response `query`; `rag_search_contracts` passed 33/33 after RED. |
| AC-RPD-17 | verified | Advanced `stageResults` are emitted from executed nodes with actual input/output artifact keys for tree extraction/persistence and query generalization/rewrite handoffs. |
| AC-RPD-18 | verified | `make be-test` was attempted first and non-RAG failures were isolated; focused Phase 3 RAG regression and `git diff --check` passed. |
| AC-RPD-19 | verified | Search trace persistence now stores safe `stageResults` summaries for executed retrieval nodes when trace is enabled and writes no row when trace is disabled; diagnostics contracts 7/7 and diagnostics API 3/3 passed after RED. |
| AC-RPD-20 | verified | Workbench renders ingest/search stage detail rows, trace code, warning labels, and legacy fallback without `stageResults`; Playwright RAG profile/workbench test passed 3/3 after RED and base review-fix. |
| AC-RPD-21 | verified | Phase 4 Makefile-first attempt isolated known non-RAG failures; focused backend RAG regression, frontend check, Playwright proof, backend fmt, `git diff --check`, and final docs validation passed. |

## Commands

- `git diff --check` passed.
- `make fe-check` passed.
- `make fe-test TEST=tron-frontend/tests/rag-profile-form-pages.spec.ts` passed 3/3.
- `cargo test --manifest-path tron-backend/Cargo.toml --lib rag` passed 15/15.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api` passed 12/12.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts` passed 16/16.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` passed 33/33.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_api` passed 2/2.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_api` passed 6/6.
- `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-pipeline-dag-runtime --phase final` passed.
- `make be-test` still fails on known non-RAG auth/MCP environment tests after RAG suites pass.
- Phase 2 integration: `cargo test --manifest-path tron-backend/Cargo.toml --lib rag` passed 14/14.
- Phase 2 integration: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts` passed 15/15.
- Phase 2 integration: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` passed 32/32.
- Phase 2 integration: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api` passed 12/12, `rag_ingest_api` passed 2/2, and `rag_search_api` passed 6/6.
- Phase 2 integration: focused RAG regression command passed.
- Phase 3 RED: `cargo test --manifest-path tron-backend/Cargo.toml --lib rag_pipeline` failed before implementation with 3 passed and 2 failed for missing advanced catalog/effective-query contract.
- Phase 3 RED: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts` failed before implementation with 15 passed and 1 failed for `IngestProfileInvalid` on the advanced tree pipeline.
- Phase 3 RED: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` failed before implementation with 32 passed and 1 failed for `RetrievalProfileInvalid` on the advanced query pipeline.
- Phase 3 GREEN: `cargo test --manifest-path tron-backend/Cargo.toml --lib rag_pipeline` passed 5/5.
- Phase 3 GREEN: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts` passed 16/16.
- Phase 3 GREEN: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` passed 33/33.
- Phase 3 VERIFY: `cargo test --manifest-path tron-backend/Cargo.toml --lib rag && cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api && cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts && cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts && cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_api && cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_api` passed.
- Phase 3 VERIFY: `cargo fmt --manifest-path tron-backend/Cargo.toml --check`, `git diff --check`, and final feature docs validation passed after worker merge.
- Phase 4 RED: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_diagnostics_contracts && cargo test --manifest-path tron-backend/Cargo.toml --test rag_diagnostics_api` failed before implementation because `SEARCH_TRACE` payload omitted `stageResults`.
- Phase 4 RED: `make fe-test TEST=tron-frontend/tests/rag-profile-form-pages.spec.ts` failed before UI change because Workbench did not render labeled stage warning feedback.
- Phase 4 GREEN/VERIFY: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_diagnostics_contracts && cargo test --manifest-path tron-backend/Cargo.toml --test rag_diagnostics_api` passed 7/7 and 3/3 after backend merge.
- Phase 4 GREEN/VERIFY: `make fe-test TEST=tron-frontend/tests/rag-profile-form-pages.spec.ts` passed 3/3 after frontend merge and route-stage review-fix.
- Phase 4 VERIFY: `make fe-check`, `cargo fmt --manifest-path tron-backend/Cargo.toml --check`, and `git diff --check` passed.
- Phase 4 VERIFY: focused RAG regression command passed across lib rag 15/15, management API 12/12, diagnostics contracts 7/7, diagnostics API 3/3, ingest contracts 16/16, search contracts 33/33, ingest API 2/2, and search API 6/6.
- Phase 4 MAKEFILE-FIRST: `make be-test` was attempted and failed only in known non-RAG `EncryptionNotConfigured` auth/MCP handler tests and stdio MCP timeout/assertion tests.

## Risks

- Full backend Makefile remains blocked by existing non-RAG test environment issues: missing encryption configuration in auth/MCP handler tests and MCP stdio timeout assertions.
- Tree graph persistence currently hands generated graph artifacts to the existing ingest persistence command. Deeper graph-store materialization remains a future adapter concern.
