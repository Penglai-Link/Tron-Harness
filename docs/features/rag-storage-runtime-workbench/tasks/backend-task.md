# Backend Task Rows

## Execution Contract

- canonical_report_target: `reports/backend-result.md`
- base_branch: `feature/xinlangtao/rag-storage-runtime-workbench/base`
- execution_owner: backend-worker when delegated, lead for integration and final judgment

## TDD Contract

Write failing tests before implementation for behavior-changing rows, then implement the smallest runtime-compatible path and record evidence by AC row.

## Task Rows

- AC-RSW-01 | phase: tdd | owner: backend-worker | scope: `tron-backend/src/application/rag*` `tron-backend/src/adapters/outbound/persistence/sqlite/*` `tron-backend/tests/*rag*` | command_ref: CMD-RAG-MGMT-TEST | status: pending.
- AC-RSW-02 | phase: tdd | owner: backend-worker | scope: `tron-backend/src/application/rag*` `tron-backend/src/adapters/outbound/persistence/sqlite/*` `tron-backend/tests/*rag*` | command_ref: CMD-RAG-MGMT-TEST | status: pending.
- AC-RSW-03 | phase: tdd | owner: backend-worker | scope: `tron-backend/src/application/rag*` `tron-backend/src/adapters/inbound/http/rag/*` `tron-backend/tests/*rag*` | command_ref: CMD-RAG-E2E-TEST | status: pending.
- AC-RSW-04 | phase: tdd | owner: backend-worker | scope: `tron-backend/src/application/rag*` `tron-backend/src/adapters/inbound/http/rag/*` `tron-backend/tests/*rag*` | command_ref: CMD-RAG-E2E-TEST | status: pending.
- AC-RSW-07 | phase: verify | owner: lead | scope: `docs/features/rag-storage-runtime-workbench/reports/*` runtime API smoke commands | command_ref: CMD-RAG-CURL | status: pending.
- AC-RSW-08 | phase: verify | owner: lead | scope: focused RAG backend/frontend suites | command_ref: CMD-RAG-REGRESSION | status: pending.

## Worktree Notes

Backend implementation may be delegated to a subagent branch after read-only exploration confirms stable write scope.
