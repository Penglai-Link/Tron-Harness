# Worktree Plan

## Main Agent

- branch: `feature/xinlangtao/rag-storage-runtime-workbench/base`
- worktree: `/Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-storage-runtime-workbench/base`
- upstream/base: `feature/xinlangtao/rag-frontend-release-0.1.0/base`
- base_commit: `815efe70`
- git_skill: `eliteforge-git-feature-oriented-lite-spec`

## Multi-Agent Base Branch

This branch is the task integration branch. Subagent branches must be created from it and merged back into it.

## Agent Worktrees

| agentId | subtaskName | runtime_agent_type | branch | worktree | status |
| --- | --- | --- | --- | --- | --- |
| explorer | backend-runtime-map | explorer | n/a | n/a | completed |
| explorer | frontend-workbench-map | explorer | n/a | n/a | completed |
| backend | runtime-storage | worker | `feature/xinlangtao/rag-storage-runtime-workbench/backend/runtime-storage` | `/Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-storage-runtime-workbench/backend/runtime-storage` | completed; merged=`b26bb666`; agent=`019eb283-7014-7dd1-94d7-d5925c3840df` |
| frontend | workbench-api | worker | `feature/xinlangtao/rag-storage-runtime-workbench/frontend/workbench-api` | `/Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-storage-runtime-workbench/frontend/workbench-api` | completed; merged=`92a8b836`; agent=`019eb284-33a4-7ac3-bef9-637ec635a08d` |

## Dispatch Decisions

| AC rows | decision | execution_owner | runtime_agent_type | rationale | subtaskName | status |
| --- | --- | --- | --- | --- | --- | --- |
| AC-RSW-01, AC-RSW-02, AC-RSW-03, AC-RSW-04 | spawn | backend-worker | worker | backend storage/profile/runtime scope is cohesive and isolated from frontend page work | backend/runtime-storage | completed |
| AC-RSW-05, AC-RSW-06 | spawn | frontend-worker | worker | frontend workbench/API client scope is isolated from backend schema work and can use mocked API contracts | frontend/workbench-api | completed |
| AC-RSW-07, AC-RSW-08 | lead | lead | n/a | final cross-stack smoke/regression evidence belongs to integration owner | final-verify | completed |

## Branch And Path Rules

- Subagent branch template: `feature/xinlangtao/rag-storage-runtime-workbench/<agentId>/<subtaskName>`.
- Subagent worktree template: `/Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-storage-runtime-workbench/<agentId>/<subtaskName>`.
- Subagents must not edit the base worktree directly.

## Integration Notes

- Backend branch merged with `--no-ff` as `b26bb666 merge: backend rag runtime storage`.
- Frontend branch merged with `--no-ff` as `92a8b836 merge: frontend rag workbench api`.
- Final integration fixes and evidence updates are recorded on the `/base` branch.
