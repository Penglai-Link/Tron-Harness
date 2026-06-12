# Test Report: RAG 管理配置 API 接入

## Summary

- status: complete
- execution_owner: lead-agent
- result: targeted backend and frontend verification passed in the integrated review worktree; full `make be-test` still exposes the existing non-RAG auth/MCP baseline failures already documented in backend evidence.

## Evidence Rows

- AC-RAGCFG-06 | red: corpus pages previously stayed on browser-local mock persistence and did not prove management API traffic end to end. | green: integrated review worktree browser run passed corpus list/detail/create/update flows against management API mocks with verify-save-request assertions. | browser: clicks=open RAG corpora tab, create corpus, reopen edit page, save update; inputs=corpus identity, storage targets, segment bindings, and filter rows; network=GET /api/v1/rag/corpora, GET /api/v1/rag/corpora/{corpusId}, POST /api/v1/rag/corpora/verify, POST /api/v1/rag/corpora, PUT /api/v1/rag/corpora/{corpusId}; feedback=save success and reloaded API-backed values shown; artifacts=tron-frontend/tests/rag-corpus-form-pages.spec.ts | command_ref: CMD-RAG-BROWSER | status: done.
- AC-RAGCFG-07 | red: profile pages previously mixed local mock state with incomplete API-mode coverage and lacked multi-corpus retrieval round-trip proof. | green: integrated review worktree browser run passed ingest and retrieval create/edit flows with catalog, verify, create, update, and multi-corpus retrieval assertions. | browser: clicks=open RAG profiles tab, create ingest profile, create retrieval profile, edit retrieval profile; inputs=stage strategies, corpus selection, route settings, and filter schema choices; network=GET /api/v1/rag/profiles, GET /api/v1/rag/profiles/{profileId}, GET /api/v1/rag/profiles/catalog, POST /api/v1/rag/profiles/verify, POST /api/v1/rag/profiles, PUT /api/v1/rag/profiles/{profileId}; feedback=profile save success and saved retrieval corpora re-rendered after reload; artifacts=tron-frontend/tests/rag-profile-form-pages.spec.ts | command_ref: CMD-RAG-BROWSER | status: done.
- AC-RAGCFG-08 | red: graph-enabled invalid storage combinations previously lacked a browser proof that backend verification feedback was surfaced to the operator. | green: integrated review worktree browser run passed the invalid-then-corrected graph flow and asserted the backend verification error message before successful correction. | browser: clicks=enable graph, submit invalid corpus, inspect validation error, correct binding, resubmit; inputs=invalid then corrected graph target selection; network=POST /api/v1/rag/corpora/verify and POST /api/v1/rag/corpora; feedback=backend-backed rejection message rendered before successful save; artifacts=tron-frontend/tests/rag-corpus-form-pages.spec.ts | command_ref: CMD-RAG-BROWSER | status: done.
- AC-RAGCFG-09 | red: before the management API slice, there was no integrated proof that the added routes still compiled and that existing RAG-focused runtime surfaces remained intact. | green: `make be-check`, `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api`, and `cargo test --manifest-path tron-backend/Cargo.toml rag_` all passed in the integrated review worktree; `make be-test` reproduced only the known non-RAG auth/MCP baseline failures. | browser: clicks=not_applicable_backend_only; inputs=backend compile check, focused management API tests, focused rag_ regression run, and full-suite baseline attempt; network=not_applicable_backend_suite; feedback=integrated compile and focused regression succeeded while baseline auth/MCP failures remained unchanged; artifacts=tron-backend/tests/rag_management_api.rs, tron-backend/tests/rag_service_contracts.rs, Makefile | command_ref: CMD-BE-TEST | status: done.
- AC-RAGCFG-10 | red: focused storage form test failed because the corpus form had no `Fulltext target` selector and saved no `bindings.fulltext`. | green: frontend worker and integrated base checks passed with Fulltext Target UI, legacy fallback, and API payload assertions. | browser: clicks=open corpus form storage binding, choose fulltext target, submit form; inputs=source/textUnit/fulltext/vector segment bindings; network=POST /api/v1/rag/corpora/verify and POST /api/v1/rag/corpora include fulltext binding; feedback=fulltext selector renders and save flow succeeds; artifacts=tron-frontend/tests/rag-corpus-storage-structure-form.spec.ts, tron-frontend/tests/rag-corpus-form-pages.spec.ts, tron-frontend/tests/rag-management-api-mock.ts | command_ref: CMD-RAG-BROWSER | status: done.
- AC-RAGCFG-11 | red: management fulltext tests failed with null `fulltextTable`, missing legacy fallback, and invalid graph closure. | green: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api fulltext` passed with fulltext topology persistence and projection. | browser: clicks=not_applicable_backend_only; inputs=corpus create/verify payloads with explicit and omitted fulltext binding; network=POST /api/v1/rag/corpora and POST /api/v1/rag/corpora/verify; feedback=runtimeProjection and persisted binding assertions passed; artifacts=tron-backend/tests/rag_management_api.rs | command_ref: CMD-BE-RAG-ROLE | status: done.
- AC-RAGCFG-12 | red: fulltext carrier ingest/search tests failed because `rag_project_doc_fulltext` did not exist. | green: independent fulltext carrier ingest, FTS search hydrate, and retry rebuild tests passed. | browser: clicks=not_applicable_backend_only; inputs=project-doc fulltextTable binding, ingest request, fulltext route query, retry request; network=not_applicable_backend_suite; feedback=carrier rows, FTS rows, search hydrate, and retry assertions passed; artifacts=tron-backend/tests/rag_ingest_contracts.rs, tron-backend/tests/rag_search_contracts.rs, tron-backend/tests/rag_diagnostics_contracts.rs | command_ref: CMD-BE-RAG-ROLE | status: done.
- AC-RAGCFG-13 | red: mixed Graph carrier lacked canonical SQLite hydrate proof. | green: `mixed_graph_carrier_search_hydrates_sqlite_canonical_text_units` passed after Graph search hydrates rows from SQLite `textUnitTable`. | browser: clicks=not_applicable_backend_only; inputs=Graph carrier query with carrier-only text and SQLite canonical text units; network=not_applicable_backend_suite; feedback=carrier recall text matched while returned text came from SQLite canonical table; artifacts=tron-backend/tests/rag_skill_graph_contracts.rs | command_ref: CMD-BE-RAG-MIXED | status: done.
- AC-RAGCFG-14 | red: graph capability validation still forced all master data into Graph and masked Graph target shape errors. | green: management fulltext graph tests and full graph contract suite passed. | browser: clicks=not_applicable_backend_only; inputs=graph-enabled verify payloads and graph contract search/ingest/retry flows; network=POST /api/v1/rag/corpora/verify plus backend graph suite; feedback=graph closure, shape validation, and graph runtime assertions passed; artifacts=tron-backend/tests/rag_management_api.rs, tron-backend/tests/rag_skill_graph_contracts.rs | command_ref: CMD-BE-RAG-MIXED | status: done.
- AC-RAGCFG-15 | red: role-aware storage risked regressing existing SQLite/vector/graph corpora. | green: backend compile, graph contracts, vectorTable regressions, fulltextTable regressions, and frontend RAG corpus tests passed. | browser: clicks=open corpus storage form and submit API-backed create/edit flows; inputs=embedded SQLite, split SQLite vectorTable, split SQLite fulltextTable, legacy Graph single-node, and frontend storage bindings; network=GET/POST/PUT /api/v1/rag/corpora plus backend suites; feedback=all focused regressions passed; artifacts=tron-backend/tests/rag_skill_graph_contracts.rs, tron-backend/tests/rag_search_contracts.rs, tron-backend/tests/rag_ingest_contracts.rs, tron-frontend/tests/rag-corpus-storage-structure-form.spec.ts | command_ref: CMD-BE-RUNTIME | status: done.

## Commands

- CMD-RAG-BROWSER | command: `make fe-test TEST='tests/rag-corpus-form-pages.spec.ts tests/rag-profile-form-pages.spec.ts'` | result: passed | notes: 4 tests passed in the integrated review worktree.
- CMD-FE-CHECK | command: `make fe-check` | result: passed | notes: typecheck and build passed after frontend API integration changes.
- CMD-RAG-STORAGE-FULLTEXT | command: `make fe-test TEST='tests/rag-corpus-storage-structure-form.spec.ts --grep "creates target-first mixed-media corpus config"'` | result: passed | notes: worker observed RED before implementation and GREEN after adding the Fulltext Target segment.
- CMD-BE-RAG-ROLE | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api fulltext && cargo test --manifest-path tron-backend/Cargo.toml --test rag_ingest_contracts project_doc_ingest_populates_distinct_fulltext_storage_when_configured && cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts project_doc_fulltext_route_reads_distinct_carrier_and_hydrates_text_units && cargo test --manifest-path tron-backend/Cargo.toml --test rag_diagnostics_contracts retry_rebuilds_fulltext_and_vector_from_persisted_text_units` | result: passed | notes: fulltext role backend tests passed.
- CMD-BE-RAG-MIXED | command: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts` | result: passed | notes: 8 graph tests passed.
- CMD-BE-TEST | command: `make be-test` | result: baseline_failed | notes: full backend suite reproduced 7 pre-existing non-RAG auth/MCP failures while integrated RAG management compile and focused regression checks remained green.

## Changed Paths

- `tron-backend/src/adapters/inbound/http/rag.rs`
- `tron-backend/src/adapters/inbound/http/rag/error_mapper.rs`
- `tron-backend/src/adapters/inbound/http/rag/handler.rs`
- `tron-backend/src/adapters/inbound/http/rag/routes.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
- `tron-backend/src/application/rag.rs`
- `tron-backend/src/application/rag/management.rs`
- `tron-backend/src/domain/rag/model.rs`
- `tron-backend/tests/rag_management_api.rs`
- `tron-frontend/src/domain/rag/api/ragManagementApi.ts`
- `tron-frontend/src/domain/rag/hooks/useRagCorpora.ts`
- `tron-frontend/src/domain/rag/hooks/useRagCorpusDetail.ts`
- `tron-frontend/src/domain/rag/hooks/useRagProfileDetail.ts`
- `tron-frontend/src/domain/rag/hooks/useRagProfiles.ts`
- `tron-frontend/src/domain/rag/pages/RagCorpusFormPage.tsx`
- `tron-frontend/src/domain/rag/pages/RagPage.tsx`
- `tron-frontend/src/domain/rag/pages/RagProfileFormPage.tsx`
- `tron-frontend/src/shared/api/serviceConfig.ts`
- `tron-frontend/tests/rag-corpus-form-pages.spec.ts`
- `tron-frontend/tests/rag-management-api-mock.ts`
- `tron-frontend/tests/rag-profile-form-pages.spec.ts`

## Blockers

- None.
