# Frontend Task: Task Management

## Execution Contract

- feature_slug: task-management
- status: done
- execution_mode: solo

## TDD Contract

Frontend work follows the backend API contract and reuses shared Toast for visible feedback.

## Task Rows

- AC-TASK-07 | phase: tdd | owner: lead | scope: `tron-frontend/src/domain/task/**` role-aware task model, create/detail pages, and smoke assertions | command_ref: CMD-TASK-BROWSER | status: done.
- AC-TASK-08 | phase: tdd | owner: lead | scope: frontend mock/demo parity with the new task types, statuses, `statusRules`, and `defaultStatuses` | command_ref: CMD-FE-CHECK | status: done.

## Worktree Notes

Lead checkout: `/Users/clouds3n/Coding/open-source/ai/Tron` on branch `feature/clouds3n/task-management`.

## Integration Notes

Frontend task pages must keep mock mode usable while API mode uses `/api/v1/tasks/options` as the source of truth for role-aware statuses.

## Open Decisions

- None.
