# Review Task: RAG 后端底座 - 同步写入 DAG

## Execution Contract

- feature_slug: rag-ingest-dag
- status: ready
- execution_mode: pending_dispatch
- canonical_report_target: reports/review-report.md
- main_agent_owned: true

## TDD Contract

Review rows run after implementation evidence exists. Review must compare changed files against API contract、database design、acceptance rows、test plan、RAG boundary rules and forbidden feature behavior.

## Task Rows

- AC-RID-01 | phase: review | owner: lead | scope: docs/features/rag-ingest-dag/reports/* tron-backend/src/adapters/inbound/http/rag/* tron-backend/tests/rag_ingest_api.rs | command_ref: none | status: ready.
- AC-RID-02 | phase: review | owner: lead | scope: docs/features/rag-ingest-dag/reports/* tron-backend/src/application/rag/* tron-backend/src/domain/rag/* | command_ref: none | status: ready.
- AC-RID-03 | phase: review | owner: lead | scope: docs/features/rag-ingest-dag/reports/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/tests/rag_ingest_contracts.rs | command_ref: none | status: ready.
- AC-RID-04 | phase: review | owner: lead | scope: docs/features/rag-ingest-dag/reports/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/tests/rag_ingest_contracts.rs | command_ref: none | status: ready.
- AC-RID-05 | phase: review | owner: lead | scope: docs/features/rag-ingest-dag/reports/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/tests/rag_ingest_contracts.rs | command_ref: none | status: ready.
- AC-RID-06 | phase: review | owner: lead | scope: docs/features/rag-ingest-dag/reports/* tron-backend/src/application/rag/* tron-backend/src/application/llm/* tron-backend/tests/rag_ingest_contracts.rs | command_ref: none | status: ready.
- AC-RID-07 | phase: review | owner: lead | scope: docs/features/rag-ingest-dag/reports/* tron-backend/src/adapters/inbound/http/* tron-backend/src/application/rag/* tron-backend/src/application/graph/* tron-backend/src/shared/event_bus/* | command_ref: none | status: ready.

## Worktree Notes

- Review runs from the integrated base branch after implementation reports are available.

## Integration Notes

- Review should reuse fresh `make be-test` and `make be-check` evidence unless covered files change after VERIFY.

## Open Decisions

- None.
