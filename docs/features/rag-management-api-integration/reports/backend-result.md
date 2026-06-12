# Backend Result: RAG 管理配置 API 接入

## Summary

- status: complete
- execution_owner: main-agent
- branch: feature/xinlangtao/rag-frontend-release-0.1.0/base
- result: evolved corpus management topology so `source`, `textUnit`, and `vector` can preserve independent SQLite bindings through management APIs, and SQLite runtime execution now honors optional `vectorTable` compatibility data across ingest/search/debug/retry flows. `make be-check`, `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api`, `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts`, `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts`, and `cargo test --manifest-path tron-backend/Cargo.toml --test rag_diagnostics_contracts` passed.

## Evidence Rows

- AC-RAGCFG-01 | red: initial `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api` failed before implementation because `/api/v1/rag/corpora` and `/api/v1/rag/corpora/{corpusId}` returned 404 and did not project management DTOs with metadata groups or field mappings. | green: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api` passed with protected corpora list/detail responses exposing management DTOs, grouped field mappings, metadata groups, and legacy runtime projection compatibility. | browser: clicks=not_applicable_backend_only; inputs=authenticated corpus list and detail requests plus DTO assertions; network=GET /api/v1/rag/corpora and GET /api/v1/rag/corpora/{corpusId}; feedback=management list/detail projection assertions passed; artifacts=tron-backend/tests/rag_management_api.rs, tron-backend/src/application/rag/management.rs | command_ref: CMD-BE-RAG | status: done.
- AC-RAGCFG-02 | red: initial `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api` failed before implementation because `/api/v1/rag/corpora/catalog` and `/api/v1/rag/corpora/verify` were missing. | green: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api` passed with corpus catalog payloads for storage backends, contract fields, capability rules, and verify diagnostics for unsupported topology combinations. | browser: clicks=not_applicable_backend_only; inputs=corpus catalog and verify JSON payloads; network=GET /api/v1/rag/corpora/catalog and POST /api/v1/rag/corpora/verify; feedback=catalog options and verify error-code assertions passed; artifacts=tron-backend/tests/rag_management_api.rs, tron-backend/src/adapters/inbound/http/rag/handler.rs | command_ref: CMD-BE-RAG | status: done.
- AC-RAGCFG-03 | red: independent SQLite vector bindings were previously collapsed onto `textUnit` during runtime projection, so create/update could not preserve a distinct vector target in compatibility storage. | green: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api` passed with create/update persisting `storage_topology_json`, preserving metadata groups, and projecting optional `vectorTable` data into legacy runtime `storage_binding_json` for split SQLite corpora. | browser: clicks=not_applicable_backend_only; inputs=create and update corpus payloads plus SQLite schema/value checks; network=POST /api/v1/rag/corpora and PUT /api/v1/rag/corpora/{corpusId}; feedback=topology persistence and runtime projection assertions passed; artifacts=tron-backend/tests/rag_management_api.rs, tron-backend/src/application/rag/management.rs, tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs | command_ref: CMD-BE-RAG | status: done.
- AC-RAGCFG-04 | red: initial `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api` failed before implementation because `/api/v1/rag/profiles` and `/api/v1/rag/profiles/{profileId}` were missing and could not expose management DTOs for ingest and retrieval profiles. | green: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api` passed with profile list/detail responses exposing ingest and retrieval management DTOs, including retrieval multi-corpus rows and runtime-readable normalization. | browser: clicks=not_applicable_backend_only; inputs=authenticated profile list and detail requests plus management DTO assertions; network=GET /api/v1/rag/profiles and GET /api/v1/rag/profiles/{profileId}; feedback=profile projection assertions passed; artifacts=tron-backend/tests/rag_management_api.rs, tron-backend/src/application/rag/management.rs | command_ref: CMD-BE-RAG | status: done.
- AC-RAGCFG-05 | red: initial `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api` failed before implementation because profile catalog, filter validation, verify, create, and update flows were missing. | green: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api` passed with profile catalog/filter validation/verify/create/update flows validating corpus, strategy, and filter references while keeping saved runtime profile storage readable by existing RAG services. | browser: clicks=not_applicable_backend_only; inputs=profile catalog validate verify create update payloads; network=GET /api/v1/rag/profiles/catalog, POST /api/v1/rag/filters/validate, POST /api/v1/rag/profiles, and PUT /api/v1/rag/profiles/{profileId}; feedback=validation and persistence assertions passed; artifacts=tron-backend/tests/rag_management_api.rs, tron-backend/src/application/rag/management.rs, tron-backend/src/adapters/inbound/http/rag/error_mapper.rs | command_ref: CMD-BE-RAG | status: done.
- AC-RAGCFG-09 | red: runtime management evolution originally only preserved optional `vectorTable` compatibility data, so ingest/search/debug/retry still fell back to `textUnitTable` vector-side SQLite storage and independent vector bindings were not executable end-to-end. | green: `make be-check`, `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api`, `cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts`, `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts`, and `cargo test --manifest-path tron-backend/Cargo.toml --test rag_diagnostics_contracts` passed after the SQLite runtime evolution, proving separate `vectorTable` execution now works through management persistence, ingest, search, and retry/debug flows while request/response contracts stay stable. | browser: clicks=not_applicable_backend_only; inputs=backend compile check plus focused management, ingest, search, and diagnostics/retry contract tests; network=not_applicable_backend_suite; feedback=compile and focused runtime regressions passed with separate vector storage enabled; artifacts=tron-backend/tests/rag_management_api.rs, tron-backend/tests/rag_ingest_contracts.rs, tron-backend/tests/rag_search_contracts.rs, tron-backend/tests/rag_diagnostics_contracts.rs, tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/sql_plan.rs | command_ref: CMD-BE-RUNTIME | status: done.
- AC-RAGCFG-11 | red: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api fulltext` failed because `fulltextTable` was null, legacy payloads omitted fulltext fallback, and create/update did not persist fulltext topology. | green: the same focused management test passed with explicit `bindings.fulltext`, legacy fallback to `textUnit`, and persisted `storage_binding_json.fulltextTable`. | browser: clicks=not_applicable_backend_only; inputs=corpus create/verify payloads with explicit and omitted fulltext bindings; network=POST /api/v1/rag/corpora and POST /api/v1/rag/corpora/verify; feedback=runtimeProjection and persisted binding assertions passed; artifacts=tron-backend/tests/rag_management_api.rs, tron-backend/src/application/rag/management.rs, tron-backend/src/domain/rag/model.rs | command_ref: CMD-BE-RAG-ROLE | status: done.
- AC-RAGCFG-14 | red: graph-enabled corpus validation still required all master data segments to be Graph and masked malformed Graph target shape errors. | green: focused management tests passed with fulltext Graph target satisfying graph closure and malformed Graph target returning the expected shape validation error. | browser: clicks=not_applicable_backend_only; inputs=graph-enabled corpus verify payloads with fulltext Graph target and malformed Graph target; network=POST /api/v1/rag/corpora/verify; feedback=graph closure and graph-shape validation assertions passed; artifacts=tron-backend/tests/rag_management_api.rs, tron-backend/src/application/rag/management.rs | command_ref: CMD-BE-RAG-MIXED | status: done.

## Commands

- CMD-BE-RAG | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api` | result: passed | notes: final focused run passed 3 tests covering corpora routes, profile routes, and topology persistence/runtime projection.
- CMD-BE-RUNTIME | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api && cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts && cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts && cargo test --manifest-path tron-backend/Cargo.toml --test rag_diagnostics_contracts` | result: passed | notes: focused management plus runtime execution regressions passed, including separate `vectorTable` ingest/search/retry coverage.
- CMD-BE-CHECK | command: `make be-check` | result: passed | notes: backend compile check passed from the worker worktree after management API integration.
- CMD-BE-RAG-ROLE | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api fulltext` | result: passed | notes: 4 fulltext management tests passed in integrated base.
- CMD-BE-RAG-MIXED | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts` | result: passed | notes: 8 graph tests passed, including mixed Graph carrier canonical hydrate.

## Changed Paths

- `tron-backend/src/application/rag.rs`
- `tron-backend/src/application/rag/management.rs`
- `tron-backend/src/domain/rag/model.rs`
- `tron-backend/src/adapters/inbound/http/rag.rs`
- `tron-backend/src/adapters/inbound/http/rag/error_mapper.rs`
- `tron-backend/src/adapters/inbound/http/rag/handler.rs`
- `tron-backend/src/adapters/inbound/http/rag/management_support.rs`
- `tron-backend/src/adapters/inbound/http/rag/routes.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/sql_plan.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/index_schema.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/indexes.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/ingest.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/search.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/diagnostics.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/retry.rs`
- `tron-backend/migrations/20260610_0001_add_rag_management_storage_topology.sql`
- `tron-backend/tests/rag_management_api.rs`
- `tron-backend/tests/rag_ingest_contracts.rs`
- `tron-backend/tests/rag_search_contracts.rs`
- `tron-backend/tests/rag_diagnostics_contracts.rs`
- `tron-backend/tests/rag_service_contracts.rs`
- `tron-frontend/src/domain/rag/model/RagModel.ts`
- `docs/features/rag-management-api-integration/orchestration/feature-evolution-log.md`
- `docs/features/rag-management-api-integration/spec/acceptance.md`
- `docs/features/rag-management-api-integration/spec/api-contract.md`
- `docs/features/rag-management-api-integration/spec/database-design.md`
- `docs/features/rag-management-api-integration/spec/feature-spec.md`
- `docs/features/rag-management-api-integration/spec/test-plan.md`
- `docs/features/rag-management-api-integration/tasks/backend-task.md`
- `docs/features/rag-management-api-integration/reports/backend-result.md`

## Blockers

- None.
