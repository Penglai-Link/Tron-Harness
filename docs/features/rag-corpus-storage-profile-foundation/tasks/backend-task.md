# Backend Task: RAG 后端底座 - Corpus 存储与 Profile 基础

## Execution Contract

- feature_slug: rag-corpus-storage-profile-foundation
- status: done
- execution_mode: hybrid
- canonical_report_target: reports/backend-result.md
- main_agent_owned: false

## TDD Contract

每个 `tdd` 行都覆盖 RED -> GREEN -> focused VERIFY。后端任务只实现 RAG foundation 的 domain/application/repository port、DTO、registry 和校验逻辑，不新增 HTTP route，不调用模型、GraphService 或 EventBus。

## Task Rows

- AC-RAG-03 | phase: tdd | owner: backend-worker | scope: tron-backend/src/domain/rag.rs tron-backend/src/domain/rag/* tron-backend/src/application/rag.rs tron-backend/src/application/rag/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/tests/*rag* | command_ref: CMD-BE-TEST | status: done.
- AC-RAG-04 | phase: tdd | owner: backend-worker | scope: tron-backend/src/domain/rag.rs tron-backend/src/domain/rag/* tron-backend/src/application/rag.rs tron-backend/src/application/rag/* tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/tests/*rag* | command_ref: CMD-BE-TEST | status: done.
- AC-RAG-05 | phase: tdd | owner: backend-worker | scope: tron-backend/src/domain/rag.rs tron-backend/src/domain/rag/* tron-backend/src/application/rag.rs tron-backend/src/application/rag/* tron-backend/tests/*rag* | command_ref: CMD-BE-TEST | status: done.
- AC-RAG-06 | phase: tdd | owner: backend-worker | scope: tron-backend/src/domain/rag.rs tron-backend/src/domain/rag/* tron-backend/src/application/rag.rs tron-backend/src/application/rag/* tron-backend/tests/*rag* | command_ref: CMD-BE-TEST | status: done.
- AC-RAG-07 | phase: tdd | owner: backend-worker | scope: tron-backend/src/domain/rag.rs tron-backend/src/domain/rag/* tron-backend/src/application/rag.rs tron-backend/src/application/rag/* tron-backend/tests/*rag* | command_ref: CMD-BE-TEST | status: done.
- AC-RAG-09 | phase: tdd | owner: backend-worker | scope: tron-backend/src/app.rs tron-backend/src/adapters/inbound/http/* tron-backend/src/application/rag.rs tron-backend/src/application/rag/* tron-backend/src/application/llm/* tron-backend/src/application/graph/* tron-backend/src/shared/event_bus/* tron-backend/tests/*rag* | command_ref: CMD-BE-CHECK | status: done.

## Worktree Notes

- Worktree path and branch are resolved in `orchestration/worktree-plan.md` from `eliteforge-git-feature-oriented-lite-spec`.
- Backend worker must build on the database migration contract and avoid changing table shape outside the approved 7-table design.

## Integration Notes

- Integrate after database task or on a branch rebased from the database-integrated base.
- Keep all external HTTP and frontend behavior unchanged for this feature.

## Open Decisions

- None.
