# Frontend Task: Task Management

## Execution Contract

- feature_slug: task-management
- status: done
- execution_mode: hybrid

## TDD Contract

Frontend work follows the backend API contract and reuses shared Toast for visible feedback.

## Task Rows

- AC-TASK-07 | phase: tdd | owner: lead | scope: `tron-frontend/src/domain/task/**` role-aware task model, create/detail pages, and smoke assertions | command_ref: CMD-TASK-BROWSER | status: done.
- AC-TASK-08 | phase: tdd | owner: lead | scope: frontend mock/demo parity with the new task types, statuses, `statusRules`, and `defaultStatuses` | command_ref: CMD-FE-CHECK | status: done.
- AC-TASK-09 | phase: tdd | owner: tdd-slice-worker | scope: kanban card drag/drop status update in `TasksKanbanPage.tsx`, status endpoint method in `taskApi.ts`, and browser proof in `task-management-smoke.spec.ts` | command_ref: CMD-TASK-BROWSER | status: done.

## Worktree Notes

Lead checkout: `/Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/task-card-drag-status/base` on branch `feature/xinlangtao/task-card-drag-status/base`.

Worker checkout: `/Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/task-card-drag-status/frontend/kanban-drag-update` on branch `feature/xinlangtao/task-card-drag-status/frontend/kanban-drag-update`.

## Integration Notes

Frontend task pages must keep mock mode usable while API mode uses `/api/v1/tasks/options` as the source of truth for role-aware statuses. Kanban drag/drop must use `POST /api/v1/tasks/{taskCode}/status`, send only `{ status }`, preserve click-to-detail, and show Toast feedback for invalid or failed drops.

## Open Decisions

- None.
