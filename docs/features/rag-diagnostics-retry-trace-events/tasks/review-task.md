# Review Task: RAG 后端底座 - 诊断、重试、Trace 与事件

## Execution Contract

- feature_slug: rag-diagnostics-retry-trace-events
- status: done
- execution_mode: lead_review
- canonical_report_target: reports/review-report.md
- main_agent_owned: true

## TDD Contract

Review rows run after implementation evidence exists. Review must compare changed files against AC rows, safety boundaries and command evidence.

## Task Rows

- AC-RDT-01 | phase: review | owner: lead | scope: docs/features/rag-diagnostics-retry-trace-events/reports/* tron-backend/src/adapters/inbound/http/rag/* tron-backend/tests/rag_diagnostics_api.rs | command_ref: none | status: done.
- AC-RDT-02 | phase: review | owner: lead | scope: docs/features/rag-diagnostics-retry-trace-events/reports/* tron-backend/src/application/rag/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/* | command_ref: none | status: done.
- AC-RDT-03 | phase: review | owner: lead | scope: docs/features/rag-diagnostics-retry-trace-events/reports/* tron-backend/src/adapters/inbound/http/rag/* tron-backend/src/application/rag/* | command_ref: none | status: done.
- AC-RDT-04 | phase: review | owner: lead | scope: docs/features/rag-diagnostics-retry-trace-events/reports/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/* | command_ref: none | status: done.
- AC-RDT-05 | phase: review | owner: lead | scope: docs/features/rag-diagnostics-retry-trace-events/reports/* tron-backend/src/application/rag/* tron-backend/src/application/llm/* | command_ref: none | status: done.
- AC-RDT-06 | phase: review | owner: lead | scope: docs/features/rag-diagnostics-retry-trace-events/reports/* tron-backend/src/application/rag/search.rs tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/* | command_ref: none | status: done.
- AC-RDT-07 | phase: review | owner: lead | scope: docs/features/rag-diagnostics-retry-trace-events/reports/* tron-backend/src/application/rag/* tron-backend/src/app.rs tron-backend/src/shared/event_bus/* | command_ref: none | status: done.
- AC-RDT-08 | phase: review | owner: lead | scope: docs/features/rag-diagnostics-retry-trace-events/reports/* tron-backend/src/application/rag/* tron-backend/src/application/graph/* tron-backend/src/adapters/inbound/http/rag/* | command_ref: none | status: done.

## Worktree Notes

- Review should reuse `CMD-BE-TEST` and `CMD-BE-CHECK` evidence unless stale or missing.
- Review must verify trace/event payload safety explicitly.

## Integration Notes

- If implementation evidence is incomplete, do not mark review done.

## Open Decisions

- None.
