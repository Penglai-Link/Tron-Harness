# Review Task: RAG 后端底座 - Corpus 存储与 Profile 基础

## Execution Contract

- feature_slug: rag-corpus-storage-profile-foundation
- status: ready
- execution_mode: pending_worktree_plan
- canonical_report_target: reports/review-report.md
- main_agent_owned: true

## TDD Contract

Review rows run after implementation evidence exists. Review must compare changed files against AC rows, proof rows, database design, API contract, command policy, and forbidden boundary changes.

## Task Rows

- AC-RAG-01 | phase: review | owner: lead | scope: docs/features/rag-corpus-storage-profile-foundation/reports/* tron-backend/migrations/* tron-backend/src/adapters/outbound/persistence/sqlite/* | command_ref: none | status: ready.
- AC-RAG-02 | phase: review | owner: lead | scope: docs/features/rag-corpus-storage-profile-foundation/reports/* tron-backend/migrations/* tron-backend/src/adapters/outbound/persistence/sqlite/* | command_ref: none | status: ready.
- AC-RAG-03 | phase: review | owner: lead | scope: docs/features/rag-corpus-storage-profile-foundation/reports/* tron-backend/src/domain/rag* tron-backend/src/application/rag* | command_ref: none | status: ready.
- AC-RAG-04 | phase: review | owner: lead | scope: docs/features/rag-corpus-storage-profile-foundation/reports/* tron-backend/src/domain/rag* tron-backend/src/application/rag* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs | command_ref: none | status: ready.
- AC-RAG-05 | phase: review | owner: lead | scope: docs/features/rag-corpus-storage-profile-foundation/reports/* tron-backend/src/application/rag* | command_ref: none | status: ready.
- AC-RAG-06 | phase: review | owner: lead | scope: docs/features/rag-corpus-storage-profile-foundation/reports/* tron-backend/src/application/rag* | command_ref: none | status: ready.
- AC-RAG-07 | phase: review | owner: lead | scope: docs/features/rag-corpus-storage-profile-foundation/reports/* tron-backend/src/application/rag* | command_ref: none | status: ready.
- AC-RAG-08 | phase: review | owner: lead | scope: docs/features/rag-corpus-storage-profile-foundation/reports/* tron-backend/src/domain/rag* tron-backend/migrations/* | command_ref: none | status: ready.
- AC-RAG-09 | phase: review | owner: lead | scope: docs/features/rag-corpus-storage-profile-foundation/reports/* tron-backend/src/app.rs tron-backend/src/adapters/inbound/http/* tron-backend/src/application/llm/* tron-backend/src/application/graph/* tron-backend/src/shared/event_bus/* | command_ref: none | status: ready.

## Worktree Notes

- Review runs from the integrated base branch after implementation reports are available.

## Integration Notes

- Review should not rerun medium commands when fresh passing VERIFY evidence exists and no covered files changed after VERIFY.

## Open Decisions

- None.
