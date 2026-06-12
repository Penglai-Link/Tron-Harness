# Worktree Plan: RAG 管理配置 API 接入

## Main Agent

- feature_slug: rag-management-api-integration
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: parallel
- main_branch: feature/xinlangtao/rag-frontend-release-0.1.0/base
- main_worktree_path: /Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-frontend-release-0.1.0/base
- status: complete

## Repository State

- default_branch: master
- base_ref: 1c29d1e5
- current_branch: feature/xinlangtao/rag-frontend-release-0.1.0/base
- note: AC-RAGCFG-01 through AC-RAGCFG-09 are complete; AC-RAGCFG-10 through AC-RAGCFG-15 extend the same feature workspace from management integration into role-aware storage runtime.

## Multi-Agent Base Branch

- branch: feature/xinlangtao/rag-frontend-release-0.1.0/base
- base_ref: 1c29d1e5
- merge_strategy: worker branches merge back into the multi-agent base branch; lead owns integration and final validation
- conflict_owner: Main Agent

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-RAGCFG-01, AC-RAGCFG-02, AC-RAGCFG-03, AC-RAGCFG-04, AC-RAGCFG-05, AC-RAGCFG-09 | spawn | backend-worker | backend DTO, migration, repository, HTTP route, and validation changes share one bounded write scope and should complete RED -> GREEN -> VERIFY in a dedicated worker | worker_ref=backend/management-api-integration; runtime_agent_type=worker; codex_agent_id=019eaf74-c847-7be0-8a69-03ed22a6e3ab | complete |
| AC-RAGCFG-06, AC-RAGCFG-07, AC-RAGCFG-08 | parallel | frontend-worker | after the API contract is frozen in base docs, frontend API wiring and page/browser changes can proceed in parallel with backend implementation because their write scope is disjoint | worker_ref=frontend/management-api-pages; runtime_agent_type=worker; codex_agent_id=019eaf75-2fab-70f3-a82b-1968502ba9d9 | complete |
| AC-RAGCFG-01, AC-RAGCFG-02, AC-RAGCFG-03, AC-RAGCFG-04, AC-RAGCFG-05, AC-RAGCFG-06, AC-RAGCFG-07, AC-RAGCFG-08, AC-RAGCFG-09 | spawn | review-worker | final review and regression verification ran in the dedicated review worktree after applying backend and frontend worker changes for merged verification | worker_ref=review/management-api-regression; runtime_agent_type=lead; codex_agent_id=current-thread | complete |
| AC-RAGCFG-10 | parallel | frontend-storage-worker | frontend fulltext segment UI, payload serialization, and browser assertions are disjoint from backend runtime implementation | worker_ref=frontend/fulltext-storage-segment; runtime_agent_type=worker; codex_agent_id=019eb10b-bf49-7653-b800-6672a966c30a | complete |
| AC-RAGCFG-11, AC-RAGCFG-14 | parallel | backend-contract-worker | management RED tests can be authored in isolation while the runtime worker owns shared backend production model changes | worker_ref=backend/fulltext-topology-management; runtime_agent_type=worker; codex_agent_id=019eb10c-069c-7d63-bccf-a3c780603d70 | complete |
| AC-RAGCFG-12, AC-RAGCFG-13, AC-RAGCFG-15 | spawn | backend-runtime-worker | SQLite fulltext carrier and mixed Graph carrier execution share runtime persistence files and need one owner to avoid split-brain dispatch semantics | worker_ref=backend/role-aware-runtime-storage; runtime_agent_type=worker; codex_agent_id=019eb10e-7415-7121-8a99-a35c70cd0a76 | complete |

## Agent Worktrees

| git agent id | subtask name | codex agent id | codex agent name | ac_refs | tdd phase | branch | worktree path | base ref | status | runtime_agent_type |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| backend | management-api-integration | 019eaf74-c847-7be0-8a69-03ed22a6e3ab | Leibniz | AC-RAGCFG-01, AC-RAGCFG-02, AC-RAGCFG-03, AC-RAGCFG-04, AC-RAGCFG-05, AC-RAGCFG-09 | tdd | feature/xinlangtao/rag-frontend-release-0.1.0/backend/management-api-integration | /Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-frontend-release-0.1.0/backend/management-api-integration | feature/xinlangtao/rag-frontend-release-0.1.0/base | complete | worker |
| frontend | management-api-pages | 019eaf75-2fab-70f3-a82b-1968502ba9d9 | Meitner | AC-RAGCFG-06, AC-RAGCFG-07, AC-RAGCFG-08 | tdd | feature/xinlangtao/rag-frontend-release-0.1.0/frontend/management-api-pages | /Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-frontend-release-0.1.0/frontend/management-api-pages | feature/xinlangtao/rag-frontend-release-0.1.0/base | complete | worker |
| review | management-api-regression | current-thread | lead-agent | AC-RAGCFG-01, AC-RAGCFG-02, AC-RAGCFG-03, AC-RAGCFG-04, AC-RAGCFG-05, AC-RAGCFG-06, AC-RAGCFG-07, AC-RAGCFG-08, AC-RAGCFG-09 | verify | feature/xinlangtao/rag-frontend-release-0.1.0/review/management-api-regression | /Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-frontend-release-0.1.0/review/management-api-regression | feature/xinlangtao/rag-frontend-release-0.1.0/base | complete | lead |
| frontend | fulltext-storage-segment | 019eb10b-bf49-7653-b800-6672a966c30a | Hume | AC-RAGCFG-10 | tdd | feature/xinlangtao/rag-frontend-release-0.1.0/frontend/fulltext-storage-segment | /Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-frontend-release-0.1.0/frontend/fulltext-storage-segment | feature/xinlangtao/rag-frontend-release-0.1.0/base | complete | worker |
| backend | fulltext-topology-management | 019eb10c-069c-7d63-bccf-a3c780603d70 | Descartes | AC-RAGCFG-11, AC-RAGCFG-14 | tdd | feature/xinlangtao/rag-frontend-release-0.1.0/backend/fulltext-topology-management | /Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-frontend-release-0.1.0/backend/fulltext-topology-management | feature/xinlangtao/rag-frontend-release-0.1.0/base | complete | worker |
| backend | role-aware-runtime-storage | 019eb10e-7415-7121-8a99-a35c70cd0a76 | Kepler | AC-RAGCFG-12, AC-RAGCFG-13, AC-RAGCFG-15 | tdd | feature/xinlangtao/rag-frontend-release-0.1.0/backend/role-aware-runtime-storage | /Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-frontend-release-0.1.0/backend/role-aware-runtime-storage | feature/xinlangtao/rag-frontend-release-0.1.0/base | complete | worker |

## Branch And Path Rules

- Branch and worktree naming follow `eliteforge-git-feature-oriented-lite-spec`.
- Worker worktree paths must mirror the branch tree under `/Users/langtaoxin/.codex/worktrees/tron`.
- Delegated workers do not edit the lead checkout and must stay within their assigned write scope.

## Merge Strategy

- Freeze spec/api/database docs in base first.
- Integrate the backend worker first when frontend verification depends on running management endpoints or backend contract fixes.
- Allow backend and frontend implementation to overlap only after the API contract and accepted DTO shapes are stable in base.
- Merge worker branches into `feature/xinlangtao/rag-frontend-release-0.1.0/base` with `--no-ff` when possible.

## Conflict Rules

- Lead owns all conflict resolution.
- Backend worker owns `tron-backend/**` RAG management changes.
- Frontend worker owns `tron-frontend/src/domain/rag/**`, focused RAG tests, and any required `serviceConfig` domain registration.
- If shared files such as model contracts or route registries unexpectedly overlap, stop parallel edits for that file family and integrate sequentially.

## Cleanup Plan

- Keep the base and worker worktrees until final evidence passes and the user decides on push/merge behavior.
- After final acceptance, remove worker worktrees and delete merged worker branches.

## Open Decisions

- None.
