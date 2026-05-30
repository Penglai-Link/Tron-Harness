# Backend Task: RAG 后端底座 - Memory 与 Project Doc 领域字段

## Execution Contract

- feature_slug: rag-domain-fields
- status: in_progress
- execution_mode: hybrid_worker
- canonical_report_target: reports/backend-result.md
- main_agent_owned: false

## TDD Contract

每个 `tdd` 行都覆盖 RED -> GREEN -> focused VERIFY。本任务允许修改 RAG migration、SQLite migration registry、RAG repository schema evolution、RagService filterSchema 投影和相关测试；不实现真实 ingest/search。

## Task Rows

- AC-RDF-01 | phase: tdd | owner: backend-worker | scope: tron-backend/migrations/*_add_rag_domain_fields.sql tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/tests/*rag* | command_ref: CMD-BE-TEST | status: in_progress.
- AC-RDF-02 | phase: tdd | owner: backend-worker | scope: tron-backend/migrations/*_add_rag_domain_fields.sql tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/tests/*rag* | command_ref: CMD-BE-TEST | status: in_progress.
- AC-RDF-03 | phase: tdd | owner: backend-worker | scope: tron-backend/migrations/*_add_rag_domain_fields.sql tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/src/application/rag/service.rs tron-backend/tests/*rag* | command_ref: CMD-BE-TEST | status: in_progress.
- AC-RDF-04 | phase: tdd | owner: backend-worker | scope: tron-backend/migrations/*_add_rag_domain_fields.sql tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs tron-backend/src/application/rag/service.rs tron-backend/tests/*rag* | command_ref: CMD-BE-TEST | status: in_progress.
- AC-RDF-05 | phase: tdd | owner: backend-worker | scope: tron-backend/src/application/rag/service.rs tron-backend/tests/*rag* | command_ref: CMD-BE-TEST | status: in_progress.
- AC-RDF-06 | phase: tdd | owner: backend-worker | scope: tron-backend/src/app.rs tron-backend/src/adapters/inbound/http/* tron-backend/src/application/rag.rs tron-backend/src/application/rag/* tron-backend/src/application/llm/* tron-backend/src/application/graph/* tron-backend/src/shared/event_bus/* tron-backend/tests/*rag* | command_ref: CMD-BE-CHECK | status: in_progress.

## Worktree Notes

- Worktree path and branch are resolved in `orchestration/worktree-plan.md` from `eliteforge-git-feature-oriented-lite-spec`.
- This slice must keep RAG domain fields internal and avoid adding HTTP routes or execution behavior.
- Schema evolution must be idempotent because `SqliteRagRepository::connect` can run more than once against the same SQLite file.

## Integration Notes

- This task depends on the completed `rag-corpus-storage-profile-foundation` schema and service contract.
- The likely implementation path is one bounded backend worker because migration, seed projection, and service validation share the same files.

## Open Decisions

- None.
