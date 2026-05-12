# Worktree Plan: Common Snowflake ID Generation

## Main Agent

- feature_slug: common-snowflake-id
- main_agent: Codex lead agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- main_branch: feature/clouds3n/common-snowflake-id/base
- main_worktree_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/base
- status: done

## Repository State

- main_repo_path: /Users/clouds3n/.codex/worktrees/fc53/Tron
- default_branch: master
- base_commit: 9629b36
- working_tree_status: clean before execution start

## Multi-Agent Base Branch

- branch: feature/clouds3n/common-snowflake-id/base
- base_ref: master
- merge_strategy: merge
- conflict_owner: Main Agent

## Agent Worktrees

One row is one actual git worktree checkout. Resolve `agent id`, `subtask name`, `branch`, and `worktree path` from the selected `eliteforge-git-feature-oriented-*` skill. Do not duplicate `branch` or `worktree path` across rows.

| agent id | subtask name | task ids | tdd phase | branch | worktree path | base ref | status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| backend | bootstrap-harness | BE-SUPPORT-01 | support | feature/clouds3n/common-snowflake-id/backend/bootstrap-harness | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/backend/bootstrap-harness | feature/clouds3n/common-snowflake-id/base | ready |
| test | red-suite | TEST-RED-01, TEST-RED-02, TEST-RED-03, TEST-RED-04, TEST-RED-05, TEST-RED-06, TEST-RED-07, TEST-RED-08, TEST-RED-09, TEST-RED-10, TEST-RED-11, TEST-RED-12, TEST-RED-13 | red | feature/clouds3n/common-snowflake-id/test/red-suite | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/test/red-suite | feature/clouds3n/common-snowflake-id/base | ready |
| backend | green-implementation | BE-GREEN-01, BE-GREEN-02, BE-GREEN-03, BE-GREEN-04, BE-GREEN-05 | green | feature/clouds3n/common-snowflake-id/backend/green-implementation | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/backend/green-implementation | feature/clouds3n/common-snowflake-id/base | ready |
| test | verify-regression | BE-VERIFY-01 | verify | feature/clouds3n/common-snowflake-id/test/verify-regression | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/test/verify-regression | feature/clouds3n/common-snowflake-id/base | ready |
| review | final-review | REVIEW-01 | review | feature/clouds3n/common-snowflake-id/review/final-review | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/review/final-review | feature/clouds3n/common-snowflake-id/base | ready |

## Branch And Path Rules

- Branch, worktree, main-agent, and subagent naming is owned by `git_skill`; do not define another naming scheme in this feature workspace.
- Every subagent receives exactly one worktree path and one branch; every worker branch may be checked out by only one worktree.
- Worktree paths must mirror the selected-git-skill branch tree under the selected-git-skill worktree root.
- No two subagents share a worktree or branch.
- Subagents must not edit the main worktree.
- Workers write report fragments under `reports/agents/`; the Main Agent updates canonical reports.

## Merge Strategy

The lead provisions worker worktrees from `feature/clouds3n/common-snowflake-id/base`, then integrates in strict order: support harness, RED tests, GREEN implementation, verify, review. After each integration, the lead runs the relevant validation command from the task row and records the event in `orchestration/integration-log.md`.

## Conflict Rules

The lead owns conflict resolution on the integration branch. Any worker diff outside its write scope or any conflict affecting feature docs, Cargo workspace structure, or shared ID module boundaries pauses execution for review before merge.

## Cleanup Plan

Worker worktrees and branches may be removed only after final validation passes, report evidence is complete, and the user accepts delivery or asks for cleanup.

## Open Decisions

- None.
