# Worktree Plan: {{FEATURE_NAME}}

## Main Agent

- feature_slug: {{FEATURE_SLUG}}
- main_agent: first agent that activated this skill
- status: draft

## Repository State

- main_repo_path: unknown
- base_branch: unknown
- base_commit: unknown
- working_tree_status: unknown

## Integration Branch

- branch: eliteforge/integration/{{FEATURE_SLUG}}
- base_ref: unknown
- merge_strategy: merge
- conflict_owner: Main Agent

## Agent Worktrees

| agent id | task ids | tdd phase | branch | worktree path | base ref | status |
| --- | --- | --- | --- | --- | --- | --- |

## Branch And Path Rules

- Every subagent receives exactly one worktree path and one branch.
- Worker branch format: `eliteforge/<agent-id>/<feature-slug>`.
- Worker worktree path format: `../.eliteforge-worktrees/<repo-name>/eliteforge/<agent-id>/<feature-slug>/`.
- The `<agent-id>` segment must come before `<feature-slug>`.
- No two subagents share a worktree or branch.
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
