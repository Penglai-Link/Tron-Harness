# Worktree Plan: Common Snowflake ID Generation

## Main Agent

- feature_slug: common-snowflake-id
- main_agent: Codex lead agent
- status: waiting_human_decision

## Repository State

- main_repo_path: /Users/clouds3n/Coding/open-source/ai/Tron
- base_branch: feature/clouds3n/common-snowflake-id
- base_commit: efd1557
- working_tree_status: dirty; existing uncommitted docs and skill files are present in the main worktree

## Integration Branch

- branch: eliteforge/integration/common-snowflake-id
- base_ref: feature/clouds3n/common-snowflake-id
- merge_strategy: merge
- conflict_owner: Main Agent

## Agent Worktrees

| agent id | task ids | tdd phase | branch | worktree path | base ref | status |
| --- | --- | --- | --- | --- | --- | --- |
| backend-harness | BE-SUPPORT-01 | support | eliteforge/backend-harness/common-snowflake-id | ../.eliteforge-worktrees/Tron/eliteforge/backend-harness/common-snowflake-id | eliteforge/integration/common-snowflake-id | waiting_human_decision |
| red-snowflake | TEST-RED-01, TEST-RED-02, TEST-RED-03, TEST-RED-04, TEST-RED-05, TEST-RED-06, TEST-RED-07, TEST-RED-08, TEST-RED-09, TEST-RED-10, TEST-RED-11, TEST-RED-12, TEST-RED-13 | red | eliteforge/red-snowflake/common-snowflake-id | ../.eliteforge-worktrees/Tron/eliteforge/red-snowflake/common-snowflake-id | eliteforge/integration/common-snowflake-id | waiting_human_decision |
| green-snowflake | BE-GREEN-01, BE-GREEN-02, BE-GREEN-03, BE-GREEN-04, BE-GREEN-05 | green | eliteforge/green-snowflake/common-snowflake-id | ../.eliteforge-worktrees/Tron/eliteforge/green-snowflake/common-snowflake-id | eliteforge/integration/common-snowflake-id | waiting_human_decision |
| verify-snowflake | BE-VERIFY-01 | verify | eliteforge/verify-snowflake/common-snowflake-id | ../.eliteforge-worktrees/Tron/eliteforge/verify-snowflake/common-snowflake-id | eliteforge/integration/common-snowflake-id | waiting_human_decision |
| review-snowflake | REVIEW-01 | review | eliteforge/review-snowflake/common-snowflake-id | ../.eliteforge-worktrees/Tron/eliteforge/review-snowflake/common-snowflake-id | eliteforge/integration/common-snowflake-id | waiting_human_decision |

## Branch And Path Rules

- Every subagent receives exactly one worktree path and one branch.
- Worker branch format: `eliteforge/<agent-id>/<feature-slug>`.
- Worker worktree path format: `../.eliteforge-worktrees/<repo-name>/eliteforge/<agent-id>/<feature-slug>/`.
- The `<agent-id>` segment must come before `<feature-slug>`.
- No two subagents share a worktree or branch.
- Subagents must not edit the main worktree.
- Workers write report fragments under `reports/agents/`; the Main Agent updates canonical reports.

## Merge Strategy

The lead creates or verifies `eliteforge/integration/common-snowflake-id`, provisions worker worktrees from that branch, then integrates in strict order: support harness, RED tests, GREEN implementation, verify, review. After each integration, the lead runs the relevant validation command from the task row and records the event in `orchestration/integration-log.md`.

## Conflict Rules

The lead owns conflict resolution on the integration branch. Any worker diff outside its write scope or any conflict affecting feature docs, Cargo workspace structure, or shared ID module boundaries pauses execution for review before merge.

## Cleanup Plan

Worker worktrees and branches may be removed only after final validation passes, report evidence is complete, and the user accepts delivery or asks for cleanup.

## Open Decisions

- None.
