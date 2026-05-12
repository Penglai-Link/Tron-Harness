# Multi-Agent Git Worktree Orchestration

Use this reference when the Main Agent dispatches subagents. Read `references/git-worktree-orchestration.md` and the selected `eliteforge-git-feature-oriented-*` skill before assigning any branch, worktree path, main-agent name, or subagent name.

## Ownership

The Main Agent owns planning, confirmation gates, git-spec selection, worktree creation, task dispatch, integration, conflict resolution, canonical reports, validation, and final user communication.

Every subagent gets exactly one git worktree and one selected-git-spec branch. A Git branch can be checked out by only one worktree, so no two subagents may share the same full branch or worktree path. `agentId` and subtask naming must follow the selected git skill; do not invent feature-skill execution ids.

## Dispatch Order

Use this order for the same acceptance IDs:

```text
explore/support -> RED -> integrate RED into selected-git-spec base branch -> GREEN -> integrate GREEN into selected-git-spec base branch -> refactor/verify -> review
```

Do not run dependent GREEN tasks before RED tests are integrated or before a documented legacy-coverage exception exists.

## Report Fragments

Each subagent writes or returns a report fragment under:

```text
docs/features/<feature-slug>/reports/agents/<task-id>-<agentId>-<subtaskName>.md
```

The Main Agent merges fragments into canonical reports.

## Scope Guard

Before integration, the Main Agent checks the worker diff against assigned write scope. Out-of-scope changes require explanation and approval before merge.
