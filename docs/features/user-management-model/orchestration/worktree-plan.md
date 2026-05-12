# Worktree Plan: User Management Model

## Main Agent

- feature_slug: user-management-model
- main_agent: Codex lead agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- main_branch: feature/clouds3n/user-management-model/base
- main_worktree_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/base
- status: done

## Repository State

- main_repo_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/base
- default_branch: master
- base_commit: 74d87fe
- working_tree_status: dirty (feature workspace continuation in progress)

## Multi-Agent Base Branch

- branch: feature/clouds3n/user-management-model/base
- base_ref: master
- merge_strategy: merge
- conflict_owner: Main Agent

## Agent Worktrees

One row is one actual git worktree checkout. Resolve `agent id`, `subtask name`, `branch`, and `worktree path` from the selected `eliteforge-git-feature-oriented-*` skill. Do not duplicate `branch` or `worktree path` across rows.

| agent id | subtask name | task ids | tdd phase | branch | worktree path | base ref | status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| database | schema-migrations | DB-GREEN-01 | green | feature/clouds3n/user-management-model/database/schema-migrations | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/database/schema-migrations | feature/clouds3n/user-management-model/base | ready |
| test | red-suite | TEST-RED-01, TEST-RED-02, TEST-RED-03, TEST-RED-04, TEST-RED-05, TEST-RED-06, TEST-RED-07, TEST-RED-08 | red | feature/clouds3n/user-management-model/test/red-suite | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-suite | feature/clouds3n/user-management-model/base | integrated |
| backend | green-user-api | BE-GREEN-01, BE-GREEN-02, BE-GREEN-03, BE-GREEN-04 | green | feature/clouds3n/user-management-model/backend/green-user-api | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/backend/green-user-api | feature/clouds3n/user-management-model/base | ready |
| frontend | green-admin-users | FE-GREEN-01 | green | feature/clouds3n/user-management-model/frontend/green-admin-users | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/frontend/green-admin-users | feature/clouds3n/user-management-model/base | ready |
| test | verify-regression | DB-VERIFY-01, BE-VERIFY-01, FE-VERIFY-01, TEST-VERIFY-01 | verify | feature/clouds3n/user-management-model/test/verify-regression | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/verify-regression | feature/clouds3n/user-management-model/base | ready |
| review | final-review | REVIEW-01 | review | feature/clouds3n/user-management-model/review/final-review | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/review/final-review | feature/clouds3n/user-management-model/base | ready |

## Branch And Path Rules

- Branch, worktree, main-agent, and subagent naming is owned by `git_skill`; do not define another naming scheme in this feature workspace.
- Every subagent receives exactly one worktree path and one branch; every worker branch may be checked out by only one worktree.
- Worktree paths must mirror the selected-git-skill branch tree under the selected-git-skill worktree root.
- No two subagents share a worktree or branch.
- Subagents must not edit the main worktree.
- Workers write report fragments under `reports/agents/`; the Main Agent updates canonical reports.

## Merge Strategy

Execution order is `test/red-suite` -> `database/schema-migrations` + `backend/green-user-api` + `frontend/green-admin-users` -> `test/verify-regression` -> `review/final-review`. The lead agent merges each worker branch into `feature/clouds3n/user-management-model/base` and records events in `orchestration/integration-log.md`.

## Conflict Rules

The lead agent owns conflict resolution on the base branch. Any cross-scope edit outside a worker's assigned write scope pauses integration for review.

## Cleanup Plan

Worker worktrees and branches are cleaned only after final validation passes and user accepts delivery.

## Open Decisions

- None.
