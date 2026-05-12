# Worktree Plan: {{FEATURE_NAME}}

## Main Agent

- feature_slug: {{FEATURE_SLUG}}
- main_agent: first agent that activated this skill
- git_skill: eliteforge-git-feature-oriented-spec | eliteforge-git-feature-oriented-lite-spec
- main_branch: unknown
- main_worktree_path: unknown
- status: draft

## Repository State

- main_repo_path: unknown
- default_branch: unknown
- base_commit: unknown
- working_tree_status: unknown

## Multi-Agent Base Branch

- branch: unknown
- base_ref: unknown
- merge_strategy: merge
- conflict_owner: Main Agent

## Agent Worktrees

One row is one actual git worktree checkout. Resolve `agent id`, `subtask name`, `branch`, and `worktree path` from the selected `eliteforge-git-feature-oriented-*` skill. Do not duplicate `branch` or `worktree path` across rows.

| agent id | subtask name | task ids | tdd phase | branch | worktree path | base ref | status |
| --- | --- | --- | --- | --- | --- | --- | --- |

## Branch And Path Rules

- Branch, worktree, main-agent, and subagent naming is owned by `git_skill`; do not define another naming scheme in this feature workspace.
- Every subagent receives exactly one worktree path and one branch; every worker branch may be checked out by only one worktree.
- Worktree paths must mirror the selected-git-skill branch tree under the selected-git-skill worktree root.
- Subagents must not edit the main worktree.
- Workers write report fragments under `reports/agents/`; the Main Agent updates canonical reports.

## Merge Strategy

Describe merge, rebase, cherry-pick, or patch-apply rules and the validation command after each integration.

## Conflict Rules

Describe how conflicts are detected, who resolves them, and when execution stops.

## Cleanup Plan

Describe when worker worktrees and branches may be removed.

## Open Decisions

- None.
