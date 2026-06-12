# Backend Task: RAG 后端底座 - 诊断、重试、Trace 与事件

## Execution Contract

- feature_slug: rag-diagnostics-retry-trace-events
- status: done
- execution_mode: spawn
- canonical_report_target: reports/backend-result.md
- main_agent_owned: false

## TDD Contract

每个 `tdd` 行都覆盖 RED -> GREEN -> focused VERIFY。本任务允许修改 RAG application/domain/repository/HTTP 模块、EventBus wiring、backend tests 和 feature evidence 文档；不实现 SummaryTree、GraphService、GRAPH route、code-graph、Profile UI 或 Memory 治理流转。

## Task Rows

- AC-RDT-01 | phase: tdd | owner: backend-worker | scope: tron-backend/src/adapters/inbound/http/rag/* tron-backend/tests/rag_diagnostics_api.rs | command_ref: CMD-BE-TEST | status: done.
- AC-RDT-02 | phase: tdd | owner: backend-worker | scope: tron-backend/src/application/rag/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/* tron-backend/tests/rag_diagnostics_contracts.rs | command_ref: CMD-BE-TEST | status: done.
- AC-RDT-03 | phase: tdd | owner: backend-worker | scope: tron-backend/src/adapters/inbound/http/rag/* tron-backend/src/application/rag/* tron-backend/tests/rag_diagnostics_api.rs tron-backend/tests/rag_diagnostics_contracts.rs | command_ref: CMD-BE-TEST | status: done.
- AC-RDT-04 | phase: tdd | owner: backend-worker | scope: tron-backend/src/application/rag/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/* tron-backend/tests/rag_diagnostics_contracts.rs | command_ref: CMD-BE-TEST | status: done.
- AC-RDT-05 | phase: tdd | owner: backend-worker | scope: tron-backend/src/application/rag/* tron-backend/src/application/llm/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/* tron-backend/tests/rag_diagnostics_contracts.rs | command_ref: CMD-BE-TEST | status: done.
- AC-RDT-06 | phase: tdd | owner: backend-worker | scope: tron-backend/src/application/rag/search.rs tron-backend/src/adapters/inbound/http/rag/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/* tron-backend/tests/rag_diagnostics_contracts.rs tron-backend/tests/rag_search_contracts.rs | command_ref: CMD-BE-TEST | status: done.
- AC-RDT-07 | phase: tdd | owner: backend-worker | scope: tron-backend/src/application/rag/* tron-backend/src/app.rs tron-backend/src/shared/event_bus/* tron-backend/tests/rag_diagnostics_contracts.rs | command_ref: CMD-BE-TEST | status: done.
- AC-RDT-08 | phase: tdd | owner: backend-worker | scope: tron-backend/src/application/rag/* tron-backend/src/application/graph/* tron-backend/src/adapters/inbound/http/rag/* tron-backend/tests/*rag* | command_ref: CMD-BE-CHECK | status: done.

## Worktree Notes

- Worker branch must be created from `feature/langtaoxin/rag-diagnostics-retry-trace-events/base`.
- Worker must write final evidence to `docs/features/rag-diagnostics-retry-trace-events/reports/backend-result.md`.
- Worker must not edit frontend files.

## Integration Notes

- Lead owns merge into base and final validation.
- Lead may harden small issues after worker result, but must record takeover in backend result.

## Open Decisions

- None.
