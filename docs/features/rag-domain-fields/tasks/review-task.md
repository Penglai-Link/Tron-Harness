# Review Task: RAG 后端底座 - Memory 与 Project Doc 领域字段

## Execution Contract

- feature_slug: rag-domain-fields
- status: ready
- execution_mode: pending_worktree_plan
- canonical_report_target: reports/review-report.md
- main_agent_owned: true

## TDD Contract

Review rows run after implementation evidence exists. Review must compare changed files against AC rows, proof rows, database design, API contract, command policy, and forbidden boundary changes.

## Task Rows

- AC-RDF-01 | phase: review | owner: lead | scope: docs/features/rag-domain-fields/reports/* tron-backend/migrations/* tron-backend/src/adapters/outbound/persistence/sqlite/* | command_ref: none | status: ready.
- AC-RDF-02 | phase: review | owner: lead | scope: docs/features/rag-domain-fields/reports/* tron-backend/migrations/* tron-backend/src/adapters/outbound/persistence/sqlite/* | command_ref: none | status: ready.
- AC-RDF-03 | phase: review | owner: lead | scope: docs/features/rag-domain-fields/reports/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/src/application/rag* | command_ref: none | status: ready.
- AC-RDF-04 | phase: review | owner: lead | scope: docs/features/rag-domain-fields/reports/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/src/application/rag* | command_ref: none | status: ready.
- AC-RDF-05 | phase: review | owner: lead | scope: docs/features/rag-domain-fields/reports/* tron-backend/src/application/rag* tron-backend/tests/*rag* | command_ref: none | status: ready.
- AC-RDF-06 | phase: review | owner: lead | scope: docs/features/rag-domain-fields/reports/* tron-backend/src/app.rs tron-backend/src/adapters/inbound/http/* tron-backend/src/application/llm/* tron-backend/src/application/graph/* tron-backend/src/shared/event_bus/* | command_ref: none | status: ready.

## Worktree Notes

- Review runs from the integrated base branch after implementation reports are available.

## Integration Notes

- Review should reuse fresh `make be-test` and `make be-check` evidence unless covered files change after VERIFY.

## Open Decisions

- None.
