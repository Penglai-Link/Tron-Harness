# Backend Task: RAG 后端底座 - 检索编排与 Context Pack

## Execution Contract

- feature_slug: rag-search-context-pack
- status: done
- execution_mode: spawn
- canonical_report_target: reports/backend-result.md
- main_agent_owned: false

## TDD Contract

每个 `tdd` 行都覆盖 RED -> GREEN -> focused VERIFY。本任务允许修改 RAG application/domain/repository/HTTP 模块、SQLite retrieval profile seed、backend tests 和 feature evidence 文档；不实现 debug、retry、trace、EventBus、SummaryTree、GraphService、GRAPH route 或 Memory 治理流转。

## Task Rows

- AC-RSC-01 | phase: tdd | owner: backend-worker | scope: tron-backend/src/adapters/inbound/http/rag/* tron-backend/src/app.rs tron-backend/tests/rag_search_api.rs | command_ref: CMD-BE-TEST | status: done.
- AC-RSC-02 | phase: tdd | owner: backend-worker | scope: tron-backend/src/application/rag/* tron-backend/src/domain/rag/* tron-backend/tests/rag_search_contracts.rs | command_ref: CMD-BE-TEST | status: done.
- AC-RSC-03 | phase: tdd | owner: backend-worker | scope: tron-backend/src/application/rag/* tron-backend/tests/rag_search_contracts.rs tron-backend/tests/rag_service_contracts.rs | command_ref: CMD-BE-TEST | status: done.
- AC-RSC-04 | phase: tdd | owner: backend-worker | scope: tron-backend/src/application/rag/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/* tron-backend/tests/rag_search_contracts.rs | command_ref: CMD-BE-TEST | status: done.
- AC-RSC-05 | phase: tdd | owner: backend-worker | scope: tron-backend/src/application/rag/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/* tron-backend/tests/rag_search_contracts.rs | command_ref: CMD-BE-TEST | status: done.
- AC-RSC-06 | phase: tdd | owner: backend-worker | scope: tron-backend/src/application/rag/* tron-backend/src/application/llm/* tron-backend/tests/rag_search_contracts.rs | command_ref: CMD-BE-TEST | status: done.
- AC-RSC-07 | phase: tdd | owner: backend-worker | scope: tron-backend/src/application/rag/* tron-backend/tests/rag_search_contracts.rs | command_ref: CMD-BE-TEST | status: done.
- AC-RSC-08 | phase: tdd | owner: backend-worker | scope: tron-backend/src/adapters/inbound/http/* tron-backend/src/application/rag/* tron-backend/src/application/graph/* tron-backend/src/shared/event_bus/* tron-backend/tests/*rag* | command_ref: CMD-BE-CHECK | status: done.

## Worktree Notes

- Worktree path and branch are resolved in `orchestration/worktree-plan.md` from `eliteforge-git-feature-oriented-lite-spec`.
- This slice should be implemented by one bounded backend worker: `backend/search-context-pack`.
- The worker must keep all model-provider calls fake in tests.

## Integration Notes

- This task depends on completed `rag-corpus-storage-profile-foundation`、`rag-domain-fields` 和 `rag-ingest-dag`。
- Integration should run `make be-test`, `make be-check`, and docs final validation from the base branch.

## Open Decisions

- None.
