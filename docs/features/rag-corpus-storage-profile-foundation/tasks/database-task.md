# Database Task: RAG 后端底座 - Corpus 存储与 Profile 基础

## Execution Contract

- feature_slug: rag-corpus-storage-profile-foundation
- status: ready
- execution_mode: pending_worktree_plan
- canonical_report_target: reports/database-result.md
- main_agent_owned: false

## TDD Contract

每个 `tdd` 行都覆盖 RED -> GREEN -> focused VERIFY。数据库任务只允许修改 RAG migration、SQLite migration registry、RAG SQLite repository schema/seed 相关测试和必要 fixture，不实现真实 ingest/search。

## Task Rows

- AC-RAG-01 | phase: tdd | owner: database-worker | scope: tron-backend/migrations/*_create_rag_foundation.sql tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/tests/*rag* | command_ref: CMD-BE-TEST | status: ready.
- AC-RAG-02 | phase: tdd | owner: database-worker | scope: tron-backend/migrations/*_create_rag_foundation.sql tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/tests/*rag* | command_ref: CMD-BE-TEST | status: ready.
- AC-RAG-08 | phase: tdd | owner: database-worker | scope: tron-backend/migrations/*_create_rag_foundation.sql tron-backend/src/domain/rag.rs tron-backend/src/domain/rag/* tron-backend/tests/*rag* | command_ref: CMD-BE-TEST | status: ready.

## Worktree Notes

- Worktree path and branch are resolved in `orchestration/worktree-plan.md` from `eliteforge-git-feature-oriented-lite-spec`.
- Database worker must not edit HTTP routes, frontend files, model provider adapters, GraphService, or EventBus code.

## Integration Notes

- Integrate database task before backend service task because service tests depend on seeded Corpus/Profile records.
- Migration must be idempotent and use project SQLite migration registration style.

## Open Decisions

- None.
