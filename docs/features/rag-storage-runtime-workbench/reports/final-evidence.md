# Final Evidence

## Summary

The integration branch `feature/xinlangtao/rag-storage-runtime-workbench/base` now contains the backend runtime-storage slice and frontend workbench/API slice. Backend corpus create/update materializes supported SQLite storage objects, saved profiles are exercised through ingest/search APIs, the RAG workbench sends real runtime requests in API mode, and focused backend/frontend regressions pass.

## Evidence Rows

- AC-RSW-01 | red: backend worker captured missing runtime storage table columns before materialization | green: corpus management create/update test passes with materialized source/textUnit/fulltext/vector/diagnostics SQLite objects | verify: final backend focused test command passed | command_ref: CMD-RAG-MGMT-TEST | status: passed.
- AC-RSW-02 | red: graph/table split behavior required explicit runtime-storage handling | green: SQLite objects materialize additively and graph-only targets remain delegated to graph runtime rather than fake SQLite tables | verify: graph closure and backend RAG tests passed | command_ref: CMD-RAG-MGMT-TEST | status: passed.
- AC-RSW-03 | red: saved ingest profile runtime test reached stage/embedding failures before compatible config was persisted | green: saved profile drives POST `/api/v1/rag/ingest` successfully | verify: ingest API and contract suites passed | command_ref: CMD-RAG-E2E-TEST | status: passed.
- AC-RSW-04 | red: saved retrieval profile needed runtime search proof | green: saved retrieval profile drives POST `/api/v1/rag/search` successfully | verify: search API and contract suites passed | command_ref: CMD-RAG-E2E-TEST | status: passed.
- AC-RSW-05 | red: profile page API-mode coverage exposed stale mock-only flow before runtime workbench feedback | green: profile create/edit tests pass with backend-shaped profile catalog/detail/create/update data | verify: frontend check and RAG browser regression passed | command_ref: CMD-FE-RAG-PROFILE | browser: clicks=profile navigation/save, inputs=identity/corpus/stage/route fields, network=profile catalog/detail/verify/create/update requests, feedback=visible save/validation messages, artifacts=`tron-frontend/test-results/` | status: passed.
- AC-RSW-06 | red: workbench runtime flow failed before ingest/search API wiring | green: workbench posts ingest/search bodies and renders success feedback | verify: focused RAG Playwright tests passed | command_ref: CMD-FE-RAG-WORKBENCH | browser: clicks=Workbench tab, Submit Ingest, Run Search, inputs=corpus/profile selectors, target scope, source JSON, query, limit, token budget, network=POST `/api/v1/rag/ingest` and POST `/api/v1/rag/search` request bodies asserted, feedback=`Ingest completed...` and `Search returned 1 hit.`, artifacts=`tron-frontend/test-results/` | status: passed.
- AC-RSW-07 | red: previous manual Project Doc write command could fail with `RAG_SOURCE_RESOLVE_FAILED` when URI/root did not match | green: automated API/contract tests cover saved profile ingest/search plus Project Doc ingest/search and split carrier table retrieval | verify: final backend focused command passed; no live `$TOKEN` curl was run in this automation | command_ref: CMD-RAG-E2E-TEST | status: passed-with-note.
- AC-RSW-08 | red: final integration initially needed dependency install in the isolated frontend worktree and API access-context alignment | green: `make fe-install` restored frontend dependencies; workbench `agentCode` is `AGENT_CODEX` and SPACE target scopes align with `accessContext.workspaceCode` | verify: backend checks, frontend checks, and focused RAG browser regressions passed | command_ref: CMD-RAG-REGRESSION | status: passed.

## Commands

- `git diff --check && make be-check` passed.
- `cargo test --manifest-path tron-backend/Cargo.toml --test rag_management_api --test rag_ingest_api --test rag_search_api --test rag_ingest_contracts --test rag_search_contracts` passed.
- `make fe-install` passed after `make fe-check` first reported missing `tsc` in the new worktree.
- `make fe-check` passed.
- `make fe-test TEST='tests/rag-profile-form-pages.spec.ts'` passed.
- `make fe-test TEST='tests/rag-corpus-form-pages.spec.ts tests/rag-corpus-storage-structure-form.spec.ts tests/rag-profile-form-pages.spec.ts'` passed.

## Open Risks

- Manual live-server curl evidence was not executed because this automation did not have a confirmed runtime `$TOKEN`. The same API route shape is covered by backend HTTP integration tests and frontend Playwright network assertions.
- Graph physical schema creation is still deferred to graph runtime capabilities; this delivery does not introduce graph DDL/materialization management APIs.
- SQLite table materialization is additive and does not destructively repair incompatible existing table schemas.
- Existing Vite large chunk warning remains during frontend production build.
