# Runtime Task: RAG Role-Aware Storage

## Execution Contract

- feature_slug: rag-management-api-integration
- status: complete
- execution_mode: parallel
- canonical_report_target: reports/runtime-result.md
- main_agent_owned: false

## TDD Contract

Runtime work upgrades ingest/search/debug/retry from whole-corpus backend dispatch to role-aware segment dispatch. It must preserve all current runtime HTTP contracts and keep legacy corpus rows compatible.

## Task Rows

- AC-RAGCFG-12 | phase: tdd | owner: runtime-worker | scope: SQLite `fulltextTable` model parsing, SQL plan, carrier table creation, ingest carrier writes, FTS sync, fulltext search hydrate, diagnostics status, and retry rebuild | command_ref: CMD-BE-RAG-ROLE | status: done.
- AC-RAGCFG-13 | phase: tdd | owner: runtime-worker | scope: `GraphBackedRagRepository` role-aware route dispatch, mixed SQLite canonical + Graph fulltext/vector retrieval, and canonical text-unit hydration | command_ref: CMD-BE-RAG-MIXED | status: done.
- AC-RAGCFG-15 | phase: verify | owner: runtime-worker | scope: regression matrix for embedded SQLite, split SQLite, split SQLite with vectorTable, fulltextTable, and legacy graph single-node corpora | command_ref: CMD-BE-RUNTIME | status: done.

## Worktree Notes

- Runtime worker branch and worktree are resolved in `orchestration/worktree-plan.md`.
- Runtime worker owns RAG persistence runtime files under `tron-backend/src/adapters/outbound/persistence/**` plus focused runtime tests.
- Runtime worker is not alone in the codebase; do not revert management or frontend worker changes.

## Integration Notes

- `source` and `textUnit` remain canonical hydration data.
- `fulltext` and `vector` are retrieval carriers. They may share a target with `textUnit`, share with each other, or bind independently.
- Missing `fulltext` in legacy rows falls back to `textUnit`.
- Graph carrier support should prefer existing graph single-node query helpers and avoid new public HTTP contracts.

## Open Decisions

- None.
