# Test Plan

## Test Plan Decision

Use backend integration tests for storage/profile persistence, frontend Playwright tests for profile/workbench UI flows, and API smoke commands for saved ingest/search runtime compatibility.

## Proof Rows

- AC-RSW-01 | layer: backend-integration | case: management create/update ensures SQLite tables for configured storage targets and exposes usable runtime projection | red: targeted test fails before storage objects are created | green: test passes after table materialization | command_ref: CMD-RAG-MGMT-TEST.
- AC-RSW-02 | layer: backend-integration | case: supported SQLite targets materialize immediately while graph-only targets remain delegated to graph runtime instead of fake SQLite DDL | red: graph/table split behavior is ambiguous in runtime projection | green: management tests keep graph closure valid and SQLite materialization additive | command_ref: CMD-RAG-MGMT-TEST.
- AC-RSW-03 | layer: backend-integration | case: saved ingest profile compiles and can drive `/api/v1/rag/ingest` for Project Doc input | red: ingest profile save/runtime load fails | green: ingest API accepts saved profile | command_ref: CMD-RAG-E2E-TEST.
- AC-RSW-04 | layer: backend-integration | case: saved retrieval profile compiles and can drive `/api/v1/rag/search` | red: search profile save/runtime load fails | green: search API returns results or a valid empty result with trace | command_ref: CMD-RAG-E2E-TEST.
- AC-RSW-05 | layer: frontend-browser | case: profile create/edit pages load catalog/detail and save backend-shaped payloads in API mode | red: Playwright mock/API contract test fails with stale mock-only flow | green: profile page tests pass | command_ref: CMD-FE-RAG-PROFILE.
- AC-RSW-06 | layer: frontend-browser | case: workbench page performs ingest and search requests and renders visible feedback | red: Playwright workbench test fails before API wiring | green: workbench test passes with network assertions | command_ref: CMD-FE-RAG-WORKBENCH.
- AC-RSW-07 | layer: api-smoke | case: saved profile API tests and Project Doc ingest/search contract tests prove `/api/v1/rag/ingest` and `/api/v1/rag/search` can run with the configured corpus/profile chain | red: source/profile/storage failures surface through API or contract tests | green: API and Project Doc ingest/search tests pass | command_ref: CMD-RAG-E2E-TEST.
- AC-RSW-08 | layer: regression | case: existing RAG management and corpus/profile browser suites remain green | red: regression suite fails after changes | green: focused regression suite passes | command_ref: CMD-RAG-REGRESSION.

## Command Index

- CMD-BE-CHECK: `make be-check` | cost: medium | rerun: before integration and final report.
- CMD-RAG-MGMT-TEST: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api` | cost: medium | rerun: after backend storage/profile changes.
- CMD-RAG-E2E-TEST: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api --test rag_ingest_api --test rag_search_api --test rag_ingest_contracts --test rag_search_contracts` | cost: high | rerun: after runtime wiring and final integration.
- CMD-FE-RAG-PROFILE: `make fe-test TEST='tests/rag-profile-form-pages.spec.ts'` | cost: medium | rerun: after frontend profile changes.
- CMD-FE-RAG-WORKBENCH: `make fe-test TEST='tests/rag-profile-form-pages.spec.ts'` | cost: medium | rerun: after workbench changes.
- CMD-RAG-CURL: local backend curl ingest/search smoke commands with `$TOKEN` | cost: manual/API | rerun: optional manual evidence when a live authenticated server is available.
- CMD-RAG-REGRESSION: `make fe-test TEST='tests/rag-corpus-form-pages.spec.ts tests/rag-corpus-storage-structure-form.spec.ts tests/rag-profile-form-pages.spec.ts'` plus focused backend RAG tests | cost: high | rerun: final report or when related files change.

## Evidence Rules

Evidence rows must include the command, red/green/verify result, changed paths, and browser evidence fields for UI claims: clicks, inputs, network, feedback, and artifacts.
