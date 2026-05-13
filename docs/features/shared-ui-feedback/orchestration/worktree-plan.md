# Worktree Plan: Shared UI Feedback

## Main Agent

- feature_slug: shared-ui-feedback
- main_agent: Codex lead agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- main_branch: feature/clouds3n/shared-ui-feedback/base
- main_worktree_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/shared-ui-feedback/base
- status: ready

## Repository State

- main_repo_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/shared-ui-feedback/base
- planning_source_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/base
- default_branch: master
- base_commit: 6c7beea
- working_tree_status: base worktree provisioned; planning docs copied from user-management prerequisite workspace

## Multi-Agent Base Branch

- branch: feature/clouds3n/shared-ui-feedback/base
- base_ref: feature/clouds3n/user-management-model/base
- merge_strategy: merge
- conflict_owner: Main Agent

## Agent Worktrees

| agent id | subtask name | codex agent id | codex agent name | task ids | tdd phase | branch | worktree path | base ref | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| test | red-feedback-runtime | 019e2214-4586-7610-bec7-2dc094890798 | Gibbs | TEST-RED-01 | red | feature/clouds3n/shared-ui-feedback/test/red-feedback-runtime | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/shared-ui-feedback/test/red-feedback-runtime | feature/clouds3n/shared-ui-feedback/base | merged_red |
| frontend | green-toast-provider | 019e221c-9607-7732-9b77-f94acf2db265 | Cicero | FE-GREEN-01 | green | feature/clouds3n/shared-ui-feedback/frontend/green-toast-provider | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/shared-ui-feedback/frontend/green-toast-provider | feature/clouds3n/shared-ui-feedback/base | merged_green |
| test | verify-feedback-runtime | 019e2225-8f0c-76c0-a25a-2f8b898b56f9 | Ampere | TEST-VERIFY-01 | verify | feature/clouds3n/shared-ui-feedback/test/verify-feedback-runtime | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/shared-ui-feedback/test/verify-feedback-runtime | feature/clouds3n/shared-ui-feedback/base | merged_verify |
| review | final-review | pending-spawn | pending-spawn | REVIEW-01 | review | feature/clouds3n/shared-ui-feedback/review/final-review | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/shared-ui-feedback/review/final-review | feature/clouds3n/shared-ui-feedback/base | ready |

## Branch And Path Rules

- Branch and worktree naming follows `eliteforge-git-feature-oriented-lite-spec`.
- Each worker branch maps to exactly one worktree path.
- Workers must not edit the lead/base worktree directly.
- Workers write report fragments under `docs/features/shared-ui-feedback/reports/agents/`.

## Merge Strategy

Integrate in strict order: `TEST-RED-01`, `FE-GREEN-01`, `TEST-VERIFY-01`, then `REVIEW-01`.

## Conflict Rules

Any worker diff outside its assigned scope pauses integration for lead review.

## Cleanup Plan

Worker worktrees and branches remain until the user accepts delivery or asks for cleanup.

## Open Decisions

- None.
