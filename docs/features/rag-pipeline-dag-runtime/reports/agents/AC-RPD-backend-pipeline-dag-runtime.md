# Backend Agent Evidence: RAG Pipeline DAG Runtime

## Summary

- Implemented the minimum mergeable backend slice: pipeline catalog, profile DAG payload/verify, additive ingest/search `stageResults`, and minimal branch/merge artifact contract tests.
- Focused backend RAG verification is green; full backend Makefile validation retains known external blockers.

## Scope

- owner: backend-worker
- branch: `feature/xinlangtao/rag-frontend-release-0.1.0/backend/pipeline-dag-runtime`
- AC rows: AC-RPD-01, AC-RPD-02, AC-RPD-03, AC-RPD-04, AC-RPD-05, AC-RPD-08

## RED

- AC-RPD-01/02/05: `make be-test` failed after RED tests because pipeline catalog/types/compiler did not exist. Representative errors: missing `RagPipelineInput`, `RagPipelineNodeInput`, `RagPipelineEdgeInput`, `pipeline_catalog`, and `validate_pipeline`.
- AC-RPD-03/04: RED tests asserted additive `stage_results` on ingest/search results before the runtime result structs exposed that field.

## GREEN

- AC-RPD-01: implemented `GET /api/v1/rag/pipeline/catalog?profileType=INGEST|RETRIEVAL`, backed by a pipeline stage/strategy/artifact registry and existing uniform `{ code, message, data }` wrapper.
- AC-RPD-02: profile management input/view now supports `pipeline.mode/nodes/edges`; verify/create/update validates node IDs, edge references, cycles, stage/profile support, strategy ownership, join/failure policies, and Stage-owned artifact keys.
- AC-RPD-03: ingest request shape and old response fields remain unchanged; `stageResults` is added from the existing ingest stage loop.
- AC-RPD-04: search request shape and old response fields remain unchanged; `stageResults` is added from existing route/fuse/dedup/context execution.
- AC-RPD-05: reusable pipeline compiler tests cover branch-ready levels, merge node inputs, and Stage-owned artifact key enforcement.
- AC-RPD-08: RAG management, ingest, and search contract suites remain passing.

## Commands

- RED: `make be-test` failed with expected missing pipeline/stageResults compile errors.
- GREEN: `cargo test --manifest-path tron-backend/Cargo.toml --lib rag` passed 13/13.
- GREEN: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api pipeline_catalog_and_profile_management_accept_new_dag_payload -- --nocapture` passed 1/1.
- GREEN: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts memory_text_contract_ingest_writes_business_text_unit_and_sidecar_indexes -- --nocapture` passed 1/1.
- GREEN: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts memory_search_returns_active_scoped_fulltext_vector_hits_and_context_pack -- --nocapture` passed 1/1.
- VERIFY: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api` passed 12/12.
- VERIFY: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts` passed 13/13.
- VERIFY: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts` passed 30/30.
- VERIFY: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_api` passed 2/2.
- VERIFY: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_api` passed 6/6.
- VERIFY: `make be-test` failed after 112 passing lib tests on known environment/non-RAG failures: `EncryptionNotConfigured` and stdio MCP 150 ms timeouts.
- VERIFY: final feature-doc validation is covered by the lead review report after frontend integration.
- VERIFY: `git diff --check` passed.

## Changed Files

- `tron-backend/src/application/rag/pipeline.rs`
- `tron-backend/src/application/rag.rs`
- `tron-backend/src/application/rag/management.rs`
- `tron-backend/src/application/rag/ingest_profile.rs`
- `tron-backend/src/application/rag/ingest.rs`
- `tron-backend/src/application/rag/ingest/executor.rs`
- `tron-backend/src/application/rag/search.rs`
- `tron-backend/src/application/rag/ingest_payload.rs`
- `tron-backend/src/adapters/inbound/http/rag/handler.rs`
- `tron-backend/src/adapters/inbound/http/rag/routes.rs`
- `tron-backend/src/adapters/inbound/http/rag/response.rs`
- `tron-backend/tests/rag_management_api.rs`
- `tron-backend/tests/rag_ingest_contracts.rs`
- `tron-backend/tests/rag_search_contracts.rs`
- `docs/features/rag-pipeline-dag-runtime/reports/backend-result.md`
- `docs/features/rag-pipeline-dag-runtime/reports/agents/AC-RPD-backend-pipeline-dag-runtime.md`

## Blockers

- Full `make be-test` is blocked by existing non-RAG environment/runtime tests. Focused backend RAG evidence is green.
- Feature-doc final validation is covered by the integrated lead evidence after frontend merge.
- The project-wide `ApiResponse::success` currently returns `SUCCESS`/`ok`, not the feature-spec example `0`/`success`; this slice kept existing wrapper semantics to avoid cross-project response-code churn.
