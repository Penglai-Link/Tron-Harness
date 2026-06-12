# Review Task: RAG 管理配置 API 接入

## Execution Contract

- feature_slug: rag-management-api-integration
- status: complete
- execution_mode: parallel
- canonical_report_target: reports/review-report.md
- main_agent_owned: false

## TDD Contract

Review work reuses RED/GREEN evidence from backend and frontend workers, runs focused revalidation where needed, and records final acceptance or residual risk by AC.

## Task Rows

- AC-RAGCFG-01 | phase: verify | owner: review-worker | scope: merged review worktree verification for backend corpora list/detail projection and preserved metadata groups | command_ref: CMD-BE-RAG | status: done.
- AC-RAGCFG-02 | phase: verify | owner: review-worker | scope: merged review worktree verification for corpus catalog and verify endpoints | command_ref: CMD-BE-RAG | status: done.
- AC-RAGCFG-03 | phase: verify | owner: review-worker | scope: merged review worktree verification for storage topology persistence and runtime projection compatibility | command_ref: CMD-BE-RAG | status: done.
- AC-RAGCFG-04 | phase: verify | owner: review-worker | scope: merged review worktree verification for ingest and retrieval profile detail/list DTO projection | command_ref: CMD-BE-RAG | status: done.
- AC-RAGCFG-05 | phase: verify | owner: review-worker | scope: merged review worktree verification for profile catalog, filter validation, verify, create, and update flows | command_ref: CMD-BE-RAG | status: done.
- AC-RAGCFG-06 | phase: verify | owner: review-worker | scope: merged review worktree browser verification for corpus API-backed list/create/edit flows | command_ref: CMD-RAG-BROWSER | status: done.
- AC-RAGCFG-07 | phase: verify | owner: review-worker | scope: merged review worktree browser verification for ingest and retrieval profile API-backed flows | command_ref: CMD-RAG-BROWSER | status: done.
- AC-RAGCFG-08 | phase: verify | owner: review-worker | scope: merged review worktree browser verification for backend-backed graph binding feedback in corpus form | command_ref: CMD-RAG-BROWSER | status: done.
- AC-RAGCFG-09 | phase: verify | owner: review-worker | scope: merged review worktree compile, focused regression reruns, and baseline full-suite risk callout | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

- Review worker uses the review worktree defined in `orchestration/worktree-plan.md`.
- Review worker does not change feature scope; it only verifies merged behavior and records evidence/risk.

## Integration Notes

- Reuse fresh browser evidence unless a later change makes it stale.
- Final review must explicitly call out any unsupported topology still deferred to later runtime evolution.

## Open Decisions

- None.
