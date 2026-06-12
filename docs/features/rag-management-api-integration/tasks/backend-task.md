# Backend Task: RAG 管理配置 API 接入

## Execution Contract

- feature_slug: rag-management-api-integration
- status: active
- execution_mode: parallel
- canonical_report_target: reports/backend-result.md
- main_agent_owned: false

## TDD Contract

每个 `tdd` 行都覆盖 RED -> GREEN -> focused VERIFY。后端任务负责 RAG management DTO、catalog/verify/create/update HTTP route、repository persistence compatibility、legacy runtime projection、SQLite 独立 `vectorTable` runtime execution support 和 focused backend tests，不改写更广义 mixed-backend runtime contract。

## Task Rows

- AC-RAGCFG-01 | phase: tdd | owner: backend-worker | scope: `tron-backend/src/application/rag/*` DTO projection for corpora list/detail and metadata group preservation | command_ref: CMD-BE-RAG | status: done.
- AC-RAGCFG-02 | phase: tdd | owner: backend-worker | scope: `tron-backend/src/application/rag/*` corpus catalog and verify services plus `tron-backend/src/adapters/inbound/http/rag/*` request/response wiring | command_ref: CMD-BE-RAG | status: done.
- AC-RAGCFG-03 | phase: tdd | owner: backend-worker | scope: `tron-backend/migrations/*rag*`, `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`, corpus create/update persistence, optional `vectorTable` compatibility projection, and runtime projection compatibility | command_ref: CMD-BE-RAG | status: done.
- AC-RAGCFG-04 | phase: tdd | owner: backend-worker | scope: `tron-backend/src/application/rag/*` profile list/detail management DTO projection | command_ref: CMD-BE-RAG | status: done.
- AC-RAGCFG-05 | phase: tdd | owner: backend-worker | scope: `tron-backend/src/application/rag/*`, `tron-backend/src/adapters/inbound/http/rag/*`, profile catalog/filter validate/verify/create/update and runtime-readable normalization | command_ref: CMD-BE-RAG | status: done.
- AC-RAGCFG-09 | phase: tdd | owner: backend-worker | scope: `tron-backend/src/adapters/inbound/http/rag/*`, `tron-backend/src/application/rag/*`, `tron-backend/tests/*rag*` compile and runtime-regression guards | command_ref: CMD-BE-TEST | status: done.
- AC-RAGCFG-11 | phase: tdd | owner: backend-management-worker | scope: `tron-backend/src/application/rag/management.rs`, `tron-backend/src/domain/rag/model.rs`, SQLite config serialization, and `tron-backend/tests/rag_management_api.rs` for `fulltext` topology/projection/backward compatibility | command_ref: CMD-BE-RAG-ROLE | status: done.
- AC-RAGCFG-12 | phase: tdd | owner: backend-runtime-worker | scope: SQLite `fulltextTable` model parsing, SQL plan, carrier table creation, ingest carrier writes, FTS sync, fulltext search hydrate, diagnostics status, and retry rebuild | command_ref: CMD-BE-RAG-ROLE | status: done.
- AC-RAGCFG-13 | phase: tdd | owner: backend-runtime-worker | scope: `GraphBackedRagRepository` role-aware route dispatch, mixed SQLite canonical plus Graph fulltext/vector retrieval, and canonical text-unit hydration | command_ref: CMD-BE-RAG-MIXED | status: done.
- AC-RAGCFG-14 | phase: tdd | owner: backend-management-worker | scope: graph-capability validation rules and catalog/runtimeProjection messages for role-aware graph segment closure | command_ref: CMD-BE-RAG-MIXED | status: done.
- AC-RAGCFG-15 | phase: verify | owner: backend-runtime-worker | scope: regression matrix for embedded SQLite, split SQLite, split SQLite with vectorTable, fulltextTable, and legacy graph single-node corpora | command_ref: CMD-BE-RUNTIME | status: done.

## Worktree Notes

- Backend worker worktree and branch are resolved in `orchestration/worktree-plan.md`.
- Backend worker must preserve existing response envelope and auth/permission middleware patterns used by other management modules.

## Integration Notes

- Introduce management endpoints without regressing existing runtime routes.
- Prefer new request/response types dedicated to management APIs instead of overloading ingest/search DTOs.
- Keep persistence backward-compatible for legacy rows that do not yet have `storage_topology_json`.
- Phase 1.1 compatibility projection is now matched by runtime support: an independently bound SQLite `vectorTable` can execute end-to-end through ingest/search/debug/retry without changing wider mixed-backend rules.
- Role-aware projection must add `fulltext` while preserving old payloads that only submit `source`, `textUnit`, and `vector`.

## Open Decisions

- None.
