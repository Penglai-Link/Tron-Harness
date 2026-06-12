# Frontend Task: RAG 管理配置 API 接入

## Execution Contract

- feature_slug: rag-management-api-integration
- status: active
- execution_mode: parallel
- canonical_report_target: reports/frontend-result.md
- main_agent_owned: false

## TDD Contract

Frontend work follows the backend management API contract and preserves the approved corpus/profile page UX. API mode must stop using browser-local mock persistence for corpora and profiles.

## Task Rows

- AC-RAGCFG-06 | phase: tdd | owner: frontend-worker | scope: `tron-frontend/src/domain/rag/pages/RagPage.tsx`, corpus API wrappers/hooks, corpus form submit/load flow, and removal of local mock persistence for corpus management | command_ref: CMD-RAG-BROWSER | status: done.
- AC-RAGCFG-07 | phase: tdd | owner: frontend-worker | scope: `tron-frontend/src/domain/rag/pages/RagProfileFormPage.tsx`, profile API wrappers/hooks, profile list/detail/create/update API mode, and retrieval multi-corpus form round-trip | command_ref: CMD-RAG-BROWSER | status: done.
- AC-RAGCFG-08 | phase: tdd | owner: frontend-worker | scope: corpus form error handling for unsupported graph/storage combinations and backend-backed validation feedback | command_ref: CMD-RAG-BROWSER | status: done.
- AC-RAGCFG-10 | phase: tdd | owner: frontend-storage-worker | scope: `tron-frontend/src/domain/rag/model/RagModel.ts`, corpus form storage-binding model/helpers/page, RAG mock/API tests, and browser payload assertions for independent `fulltext` segment | command_ref: CMD-RAG-BROWSER | status: done.

## Worktree Notes

- Frontend worker worktree and branch are resolved in `orchestration/worktree-plan.md`.
- Frontend worker owns `tron-frontend/src/domain/rag/**`, focused RAG tests, and any required `serviceConfig` domain registration.

## Integration Notes

- Do not regress existing mock-mode support outside the explicit API path.
- Prefer dedicated `ragApi.ts` and focused hooks/selectors over keeping network logic inside page components.
- Browser proof must assert real RAG management endpoint calls rather than localStorage-only state changes.
- Keep `diagnostics` implicit; only `source`, `textUnit`, `fulltext`, and `vector` are editable segment bindings.

## Open Decisions

- None.
