# Worktree Plan: User Management Model

## Main Agent

- feature_slug: user-management-model
- main_agent: Codex lead agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- main_branch: feature/clouds3n/user-management-model/base
- main_worktree_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/base
- status: complete

## Repository State

- main_repo_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/base
- default_branch: master
- base_commit: 74d87fe
- working_tree_status: Workflow B user-management refactor complete; reset-response contract fix integrated and final validation recorded

## Multi-Agent Base Branch

- branch: feature/clouds3n/user-management-model/base
- base_ref: master
- merge_strategy: merge
- conflict_owner: Main Agent

## Agent Worktrees

One row is one actual git worktree checkout. Resolve `agent id`, `subtask name`, `branch`, and `worktree path` from the selected `eliteforge-git-feature-oriented-*` skill. Do not duplicate `branch` or `worktree path` across rows.

| agent id | subtask name | Codex runtime agent id | Codex runtime agent name | task ids | tdd phase | branch | worktree path | base ref | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| database | schema-migrations | 019e1c38-2e8c-79c0-9331-337ede76c718 | Lorentz | DB-GREEN-01 | green | feature/clouds3n/user-management-model/database/schema-migrations | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/database/schema-migrations | feature/clouds3n/user-management-model/base | integrated |
| test | red-suite | prior-session-red-suite | red-test-worker | TEST-RED-01, TEST-RED-02, TEST-RED-03, TEST-RED-04, TEST-RED-05, TEST-RED-06, TEST-RED-07, TEST-RED-08 | red | feature/clouds3n/user-management-model/test/red-suite | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-suite | feature/clouds3n/user-management-model/base | integrated |
| backend | green-user-api | 019e1c38-c29d-7913-be7e-bfe92ed51299 | Nietzsche | BE-GREEN-01, BE-GREEN-02, BE-GREEN-03, BE-GREEN-04 | green | feature/clouds3n/user-management-model/backend/green-user-api | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/backend/green-user-api | feature/clouds3n/user-management-model/base | integrated |
| frontend | green-admin-users | 019e1c57-af36-7503-932e-c28b88f211cc | Sartre | FE-GREEN-01 | green | feature/clouds3n/user-management-model/frontend/green-admin-users | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/frontend/green-admin-users | feature/clouds3n/user-management-model/base | integrated |
| test | verify-regression | 019e1c74-a4ff-7862-a062-f87a93bcafad; 019e1c84-e822-7313-8205-8f533fe2331f | Singer; Parfit | DB-VERIFY-01, BE-VERIFY-01, FE-VERIFY-01, TEST-VERIFY-01 | verify | feature/clouds3n/user-management-model/test/verify-regression | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/verify-regression | feature/clouds3n/user-management-model/base | integrated |
| review | final-review | prior-session-final-review | final-review-worker | REVIEW-01 | review | feature/clouds3n/user-management-model/review/final-review | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/review/final-review | feature/clouds3n/user-management-model/base | integrated |
| backend | fix-create-validation | prior-session-backend-fix-create-validation | backend-fix-worker | BE-FIX-01 | green | feature/clouds3n/user-management-model/backend/fix-create-validation | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/backend/fix-create-validation | feature/clouds3n/user-management-model/base | integrated |
| frontend | fix-detail-not-found | prior-session-frontend-fix-detail-not-found | frontend-fix-worker | FE-FIX-01 | green | feature/clouds3n/user-management-model/frontend/fix-detail-not-found | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/frontend/fix-detail-not-found | feature/clouds3n/user-management-model/base | integrated |
| backend | fix-status-contract | prior-session-backend-fix-status-contract | backend-contract-fix-worker | BE-FIX-02 | green | feature/clouds3n/user-management-model/backend/fix-status-contract | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/backend/fix-status-contract | feature/clouds3n/user-management-model/base | integrated |
| review | post-fix-final-review | 019e1cff-1099-7453-83fd-0e8ca30de639 | Herschel | REVIEW-02 | review | feature/clouds3n/user-management-model/review/post-fix-final-review | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/review/post-fix-final-review | feature/clouds3n/user-management-model/base | integrated |
| test | red-interaction-gaps | 019e1f57-d577-7c33-af7b-c8fe8238bb0f | Einstein | TEST-RED-09, TEST-RED-10, TEST-RED-11, TEST-RED-12, TEST-RED-13 | red | feature/clouds3n/user-management-model/test/red-interaction-gaps | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-interaction-gaps | feature/clouds3n/user-management-model/base | integrated |
| frontend | green-interaction-gaps | 019e1f62-2a9b-75d1-a475-8dbcbee1be2a | Volta | FE-GREEN-02 | green | feature/clouds3n/user-management-model/frontend/green-interaction-gaps | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/frontend/green-interaction-gaps | feature/clouds3n/user-management-model/base | integrated |
| test | verify-interaction-gaps | 019e1f72-7956-78b3-bf2b-4da61c8c1c0b | McClintock | FE-VERIFY-02 | verify | feature/clouds3n/user-management-model/test/verify-interaction-gaps | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/verify-interaction-gaps | feature/clouds3n/user-management-model/base | integrated |
| review | interaction-gap-review | 019e1f7a-b170-75d3-ac54-74b03968f440 | Popper | REVIEW-03 | review | feature/clouds3n/user-management-model/review/interaction-gap-review | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/review/interaction-gap-review | feature/clouds3n/user-management-model/base | integrated |
| test | reset-contract-red | 019e1f82-2e38-7bf3-bce5-1e731fa2b375 | Socrates | TEST-RED-14 | red | feature/clouds3n/user-management-model/test/reset-contract-red | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/reset-contract-red | feature/clouds3n/user-management-model/base | integrated |
| frontend | fix-reset-contract | 019e1f88-6504-7a21-a360-20ba75ebf5db | Gibbs | FE-FIX-02 | green | feature/clouds3n/user-management-model/frontend/fix-reset-contract | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/frontend/fix-reset-contract | feature/clouds3n/user-management-model/base | integrated |

## Branch And Path Rules

- Branch, worktree, main-agent, and subagent naming is owned by `git_skill`; do not define another naming scheme in this feature workspace.
- Every subagent receives exactly one worktree path and one branch; every worker branch may be checked out by only one worktree.
- Worktree paths must mirror the selected-git-skill branch tree under the selected-git-skill worktree root.
- No two subagents share a worktree or branch.
- Subagents must not edit the main worktree.
- Workers write report fragments under `reports/agents/`; the Main Agent updates canonical reports.

## Merge Strategy

Retained execution order is `test/red-suite` -> `database/schema-migrations` + `backend/green-user-api` + `frontend/green-admin-users` -> `test/verify-regression` -> `review/final-review` -> targeted fix workers for final-review findings -> `review/post-fix-final-review`. Workflow B evolution resumed with `test/red-interaction-gaps` -> `frontend/green-interaction-gaps` -> `test/verify-interaction-gaps` -> `review/interaction-gap-review` -> `test/reset-contract-red` -> `frontend/fix-reset-contract`. The lead agent merged each worker branch into `feature/clouds3n/user-management-model/base` and recorded events in `orchestration/integration-log.md`.

## Conflict Rules

The lead agent owns conflict resolution on the base branch. Any cross-scope edit outside a worker's assigned write scope pauses integration for review.

## Cleanup Plan

Worker worktrees and branches are cleaned only after final validation passes and user accepts delivery.

## Open Decisions

- None.
