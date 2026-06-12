# Backend Result: RAG Pipeline DAG Runtime

## Summary

- Backend slice implemented Pipeline Catalog, Profile DAG validation, additive runtime stage results, and focused backend RAG regression evidence.
- Full backend `make be-test` still hits known non-RAG environment failures, while focused RAG suites pass.

## Evidence Rows

- AC-RPD-01 | red: `make be-test` failed after RED tests because pipeline catalog and pipeline types were missing | green: catalog unit and HTTP management tests passed after adding registry and route | command_ref: CMD-BE-RAG-PIPELINE | browser:not applicable | status: green.
- AC-RPD-02 | red: `make be-test` failed after RED tests because `RagPipelineInput` and `validate_pipeline` were missing | green: pipeline validation tests reject duplicate nodes, bad edges, cycles, invalid policies, unsupported stages, unsupported strategies, and strategy artifact-key override | command_ref: CMD-BE-RAG-PIPELINE | browser:not applicable | status: green.
- AC-RPD-03 | red: ingest tests asserted missing additive `stage_results` | green: ingest compatibility tests pass with old fields plus additive `stageResults` | command_ref: CMD-BE-RAG-PIPELINE | browser:not applicable | status: green.
- AC-RPD-04 | red: search tests asserted missing additive `stage_results` | green: search compatibility tests pass with old fields plus additive `stageResults` | command_ref: CMD-BE-RAG-PIPELINE | browser:not applicable | status: green.
- AC-RPD-05 | red: branch and merge artifact contract test failed before pipeline planning existed | green: `rag_pipeline_plan_preserves_branch_merge_levels_and_stage_owned_artifacts` passes | command_ref: CMD-BE-RAG-PIPELINE | browser:not applicable | status: green.
- AC-RPD-08 | red: backend focused compatibility had no Pipeline DAG runtime coverage before this slice | green: focused RAG lib, management, ingest, search, ingest API, and search API suites pass | command_ref: CMD-BE-RAG-PIPELINE | browser:not applicable | status: verified.
- AC-RPD-14 | red: Phase 3 pipeline test failed because advanced stage catalog entries and the `query.effective` route contract were missing | green: catalog and compiler expose `EXTRACT_TREE`, `PERSIST_TREE`, `QUESTION_GENERALIZATION`, and `QUERY_REWRITE` with stage-owned artifact keys | command_ref: CMD-BE-PHASE3-RAG | browser:not applicable | status: verified.
- AC-RPD-15 | red: advanced ingest tree pipeline failed with `IngestProfileInvalid` before runtime support | green: `EXTRACT_TREE` generates tree and related graph artifacts, `PERSIST_TREE` consumes them, and persistence receives generated graph without request `relatedGraph` | command_ref: CMD-BE-PHASE3-INGEST | browser:not applicable | status: verified.
- AC-RPD-16 | red: advanced retrieval query pipeline failed with `RetrievalProfileInvalid` before runtime support | green: `QUESTION_GENERALIZATION` and `QUERY_REWRITE` execute before route search and route/vector/ranking/rerank use effective query artifacts | command_ref: CMD-BE-PHASE3-SEARCH | browser:not applicable | status: verified.
- AC-RPD-17 | red: advanced stage result assertions were unreachable before advanced nodes compiled and executed | green: stage results now come from executed advanced nodes with actual input and output artifact keys for tree and query handoffs | command_ref: CMD-BE-PHASE3-SEARCH | browser:not applicable | status: verified.
- AC-RPD-18 | red: Makefile-first gate was attempted and failed in non-RAG auth/MCP tests while focused RED isolated missing RAG behavior | green: focused Phase 3 RAG regression and whitespace checks pass after implementation | command_ref: CMD-BE-PHASE3-REGRESSION | browser:not applicable | status: verified.

## Changed Paths

- `tron-backend/src/application/rag/pipeline.rs`
- `tron-backend/src/application/rag.rs`
- `tron-backend/src/application/rag/management.rs`
- `tron-backend/src/application/rag/ingest_profile.rs`
- `tron-backend/src/application/rag/ingest.rs`
- `tron-backend/src/application/rag/ingest/executor.rs`
- `tron-backend/src/application/rag/search.rs`
- `tron-backend/src/application/rag/search_ranking.rs`
- `tron-backend/src/application/rag/ingest_artifact.rs`
- `tron-backend/src/application/rag/strategy_registry.rs`
- `tron-backend/src/application/rag/ingest_payload.rs`
- `tron-backend/src/adapters/inbound/http/rag/handler.rs`
- `tron-backend/src/adapters/inbound/http/rag/routes.rs`
- `tron-backend/src/adapters/inbound/http/rag/response.rs`
- `tron-backend/tests/rag_management_api.rs`
- `tron-backend/tests/rag_ingest_contracts.rs`
- `tron-backend/tests/rag_search_contracts.rs`
- `docs/features/rag-pipeline-dag-runtime/reports/phase3-advanced-stages-result.md`
- `docs/features/rag-pipeline-dag-runtime/reports/final-evidence.md`
- `docs/features/rag-pipeline-dag-runtime/tasks/backend-task.md`
- `docs/features/rag-pipeline-dag-runtime/orchestration/worktree-plan.md`

## Commands

- RED: `make be-test` failed after RED tests with missing pipeline catalog, pipeline input types, pipeline validation, and runtime stage result fields.
- GREEN: `cargo test --manifest-path tron-backend/Cargo.toml --lib rag` passed 13/13.
- GREEN: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api pipeline_catalog_and_profile_management_accept_new_dag_payload -- --nocapture` passed 1/1.
- GREEN: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts memory_text_contract_ingest_writes_business_text_unit_and_sidecar_indexes -- --nocapture` passed 1/1.
- GREEN: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts memory_search_returns_active_scoped_fulltext_vector_hits_and_context_pack -- --nocapture` passed 1/1.
- VERIFY: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api` passed 12/12.
- VERIFY: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts` passed 13/13.
- VERIFY: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` passed 30/30.
- VERIFY: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_api` passed 2/2.
- VERIFY: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_api` passed 6/6.
- VERIFY: `make be-test` reached RAG suites and then failed on known non-RAG `EncryptionNotConfigured` auth/MCP handler tests and stdio MCP timeout tests.
- VERIFY: `git diff --check` passed in the backend worker branch.
- Phase 3 RED: `cargo test --manifest-path tron-backend/Cargo.toml --lib rag_pipeline` failed with 3 passed and 2 failed for missing advanced catalog and effective-query route contract.
- Phase 3 RED: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts` failed with 15 passed and 1 failed for the unsupported advanced tree pipeline.
- Phase 3 RED: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` failed with 32 passed and 1 failed for the unsupported advanced query pipeline.
- Phase 3 GREEN: `cargo test --manifest-path tron-backend/Cargo.toml --lib rag_pipeline` passed 5/5.
- Phase 3 GREEN: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts` passed 16/16.
- Phase 3 GREEN: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` passed 33/33.
- Phase 3 VERIFY: `cargo test --manifest-path tron-backend/Cargo.toml --lib rag && cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api && cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts && cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts && cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_api && cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_api` passed.

## Blockers

- None.
