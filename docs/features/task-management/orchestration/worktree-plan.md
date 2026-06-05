# Worktree Plan: Task Management

## Main Agent

- feature_slug: task-management
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: hybrid
- main_branch: feature/xinlangtao/task-card-drag-status/base
- main_worktree_path: /Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/task-card-drag-status/base
- status: done

## Repository State

- default_branch: master
- base_ref: master
- current_branch: feature/xinlangtao/task-card-drag-status/base
- note: this evolution preserves the earlier delivered task-management evidence and adds AC-TASK-09 for kanban drag/drop status updates.

## Multi-Agent Base Branch

- branch: feature/xinlangtao/task-card-drag-status/base
- base_ref: master
- merge_strategy: worker branch merges back to the multi-agent base branch
- conflict_owner: Main Agent

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-TASK-01 | solo | lead | user explicitly approved direct implementation through `PLEASE IMPLEMENT THIS PLAN`, and current developer rules do not allow subagents without an explicit subagent request | none | done |
| AC-TASK-02 | solo | lead | user explicitly approved direct implementation through `PLEASE IMPLEMENT THIS PLAN`, and the kanban/list changes stay in the same bounded backend slice | none | done |
| AC-TASK-03 | solo | lead | user explicitly approved direct implementation through `PLEASE IMPLEMENT THIS PLAN`, and create/default-status logic stays in the same bounded backend slice | none | done |
| AC-TASK-04 | solo | lead | user explicitly approved direct implementation through `PLEASE IMPLEMENT THIS PLAN`, and parent-child rules plus child inheritance stay tightly coupled in one slice | none | done |
| AC-TASK-05 | solo | lead | user explicitly approved direct implementation through `PLEASE IMPLEMENT THIS PLAN`, and role-based validation spans the same domain, API, and smoke surface | none | done |
| AC-TASK-06 | solo | lead | user explicitly approved direct implementation through `PLEASE IMPLEMENT THIS PLAN`, and archive/restore fallback stays in the same lifecycle slice | none | done |
| AC-TASK-07 | solo | lead | user explicitly approved direct implementation through `PLEASE IMPLEMENT THIS PLAN`, and the frontend/browser adaptation is a bounded coupled slice | none | done |
| AC-TASK-08 | solo | lead | user explicitly approved direct implementation through `PLEASE IMPLEMENT THIS PLAN`, and reset logic plus mock parity are a bounded coupled slice | none | done |
| AC-TASK-09 | spawn | tdd-slice-worker | user explicitly requested multi-agent when parallel; frontend implementation and Playwright coverage share one bounded write scope, so a single worker owns RED -> GREEN -> focused VERIFY while lead owns docs, integration, and review | worker_ref=frontend/kanban-drag-update; runtime_agent_type=worker; codex_agent_id=019e9553-e1e7-77c3-a5f3-27a4d0e2ebed | done |

## Agent Worktrees

| git agent id | subtask name | codex agent id | codex agent name | ac_refs | tdd phase | branch | worktree path | base ref | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| frontend | kanban-drag-update | 019e9553-e1e7-77c3-a5f3-27a4d0e2ebed | Poincare | AC-TASK-09 | tdd | feature/xinlangtao/task-card-drag-status/frontend/kanban-drag-update | /Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/task-card-drag-status/frontend/kanban-drag-update | feature/xinlangtao/task-card-drag-status/base | done |

Worker write scope: `tron-frontend/src/domain/task/pages/TasksKanbanPage.tsx`, `tron-frontend/src/domain/task/api/taskApi.ts`, `tron-frontend/tests/task-management-smoke.spec.ts`.

## Branch And Path Rules

- Branch naming follows `eliteforge-git-feature-oriented-lite-spec`.
- No delegated worker branch or worktree is created unless the user explicitly asks for subagents.

## Merge Strategy

- Lead updates docs and final reports on `feature/xinlangtao/task-card-drag-status/base`.
- Worker changes merge from `feature/xinlangtao/task-card-drag-status/frontend/kanban-drag-update` into the base branch after RED/GREEN evidence is reviewed.

## Conflict Rules

- Lead owns `docs/**` and PRD updates in the base worktree.
- The frontend worker owns only the assigned task board/API/test files; lead must not duplicate those edits while the worker is active.
- The pre-existing debounced search change in `tron-frontend/src/domain/task/pages/TasksKanbanPage.tsx` remains in scope and must be preserved while integrating drag/drop.

## Cleanup Plan

- Keep the base and worker worktrees until implementation is validated and the user decides whether to merge or push.

## Open Decisions

- None.
