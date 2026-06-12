# Frontend Task Rows

## Execution Contract

- canonical_report_target: `reports/frontend-result.md`
- base_branch: `feature/xinlangtao/rag-storage-runtime-workbench/base`
- execution_owner: frontend-worker when delegated, lead for integration and final judgment

## TDD Contract

Add or update Playwright tests before behavior changes, then implement API-mode profile/workbench flows and record browser evidence by AC row.

## Task Rows

- AC-RSW-05 | phase: tdd | owner: frontend-worker | scope: `tron-frontend/src/domain/rag/api/*` `tron-frontend/src/domain/rag/pages/RagProfileFormPage.tsx` `tron-frontend/tests/rag-profile-form-pages.spec.ts` `tron-frontend/tests/rag-management-api-mock.ts` | command_ref: CMD-FE-RAG-PROFILE | status: pending.
- AC-RSW-06 | phase: tdd | owner: frontend-worker | scope: `tron-frontend/src/domain/rag/pages/*` `tron-frontend/src/domain/rag/api/*` `tron-frontend/tests/*rag*workbench*` `tron-frontend/tests/rag-*.spec.ts` | command_ref: CMD-FE-RAG-WORKBENCH | status: pending.
- AC-RSW-08 | phase: verify | owner: lead | scope: focused frontend RAG regression suites | command_ref: CMD-RAG-REGRESSION | status: pending.

## Worktree Notes

Frontend implementation may be delegated to a subagent branch after the API contract is stable enough to avoid backend/frontend write conflicts.
