# Backend Task: RAG 后端底座 - 同步写入 DAG

## Execution Contract

- feature_slug: rag-ingest-dag
- status: ready
- execution_mode: pending_dispatch
- canonical_report_target: reports/backend-result.md
- main_agent_owned: false

## TDD Contract

每个 `tdd` 行都覆盖 RED -> GREEN -> focused VERIFY。本任务允许修改 RAG application/domain/repository/HTTP 模块、SQLite profile seed、backend tests 和 feature evidence 文档；不实现 search、debug、retry、SummaryTree、Graph 或 Memory 治理流转。

## Task Rows

- AC-RID-01 | phase: tdd | owner: backend-worker | scope: tron-backend/src/adapters/inbound/http/rag/* tron-backend/src/app.rs tron-backend/tests/rag_ingest_api.rs | command_ref: CMD-BE-TEST | status: ready.
- AC-RID-02 | phase: tdd | owner: backend-worker | scope: tron-backend/src/application/rag/* tron-backend/src/domain/rag/* tron-backend/tests/rag_ingest_contracts.rs | command_ref: CMD-BE-TEST | status: ready.
- AC-RID-03 | phase: tdd | owner: backend-worker | scope: tron-backend/src/application/rag/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/tests/rag_ingest_contracts.rs | command_ref: CMD-BE-TEST | status: ready.
- AC-RID-04 | phase: tdd | owner: backend-worker | scope: tron-backend/src/application/rag/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/tests/rag_ingest_contracts.rs | command_ref: CMD-BE-TEST | status: ready.
- AC-RID-05 | phase: tdd | owner: backend-worker | scope: tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/tests/rag_ingest_contracts.rs | command_ref: CMD-BE-TEST | status: ready.
- AC-RID-06 | phase: tdd | owner: backend-worker | scope: tron-backend/src/application/rag/* tron-backend/src/application/llm/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/tests/rag_ingest_contracts.rs | command_ref: CMD-BE-TEST | status: ready.
- AC-RID-07 | phase: tdd | owner: backend-worker | scope: tron-backend/src/adapters/inbound/http/* tron-backend/src/application/rag/* tron-backend/src/application/graph/* tron-backend/src/shared/event_bus/* tron-backend/tests/*rag* | command_ref: CMD-BE-CHECK | status: ready.

## Worktree Notes

- Worktree path and branch are resolved in `orchestration/worktree-plan.md` from `eliteforge-git-feature-oriented-lite-spec`.
- This slice should be implemented by one bounded backend worker unless the user explicitly approves lead solo.
- The worker must keep all model-provider calls fake in tests.

## Integration Notes

- This task depends on completed `rag-corpus-storage-profile-foundation` and `rag-domain-fields`.
- Integration should run `make be-test`, `make be-check`, and docs final validation from the base branch.

## Open Decisions

- None.
