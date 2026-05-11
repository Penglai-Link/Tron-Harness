# Multi-Agent Git Worktree Orchestration

Use this reference when the Main Agent dispatches subagents. Use `references/git-worktree-orchestration.md` for branch layout, worktree paths, creation commands, integration, conflict handling, and cleanup.

## Ownership

The Main Agent owns planning, confirmation gates, worktree creation, task dispatch, integration, conflict resolution, canonical reports, validation, and final user communication.

Every subagent gets exactly one git worktree and one branch. No subagent may write to the Main Agent worktree, and no two subagents may share a worktree or branch.

## Dispatch Order

Use this order for the same acceptance IDs:

```text
explore/support -> RED -> integrate RED -> GREEN -> integrate GREEN -> refactor/verify -> review
```

Do not run dependent GREEN tasks before RED tests are integrated or before a documented legacy-coverage exception exists.

## Report Fragments

Each subagent writes or returns a report fragment under:

```text
docs/features/<feature-slug>/reports/agents/<task-id>-<agent-id>.md
```

The Main Agent merges fragments into canonical reports.

## Scope Guard

Before integration, the Main Agent checks the worker diff against assigned write scope. Out-of-scope changes require explanation and approval before merge.
