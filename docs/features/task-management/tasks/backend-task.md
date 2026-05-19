# Backend Task: Task Management

## Execution Contract

- feature_slug: task-management
- status: done
- execution_mode: solo

## TDD Contract

Backend task rows use bounded AC slices with RED before GREEN. Lead executes locally because current runtime policy disallows spawning subagents unless the user explicitly asks for subagents or parallel agents.

## Task Rows

- AC-TASK-01 | phase: tdd | owner: lead | scope: `tron-backend/src/domain/task.rs` task enums, role rules, and `/api/v1/tasks/options` payload shape | command_ref: CMD-BE-TEST | status: done.
- AC-TASK-02 | phase: tdd | owner: lead | scope: `tron-backend/src/application/task.rs` list and kanban projection under the new status universe | command_ref: CMD-BE-TEST | status: done.
- AC-TASK-03 | phase: tdd | owner: lead | scope: create validation and role-default `Open` status assignment | command_ref: CMD-BE-TEST | status: done.
- AC-TASK-04 | phase: tdd | owner: lead | scope: detail and parent-child role inference rules | command_ref: CMD-BE-TEST | status: done.
- AC-TASK-05 | phase: tdd | owner: lead | scope: role-aware status update validation, task-type edit validation, and `TASK_STATUS_ROLE_INVALID` mapping | command_ref: CMD-BE-TEST | status: done.
- AC-TASK-06 | phase: tdd | owner: lead | scope: archive/restore lifecycle fallback to `Open` and active-subtask guard retention | command_ref: CMD-BE-TEST | status: done.
- AC-TASK-08 | phase: tdd | owner: lead | scope: legacy task schema reset detection and backend seed removal | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

Lead checkout: `/Users/clouds3n/Coding/open-source/ai/Tron` on branch `feature/clouds3n/task-management`.

## Integration Notes

Task service integrates with existing workspace and user services for active workspace and assignee validation. Legacy reset must stay conditional because repository bootstrap does not track applied migrations.

## Open Decisions

- None.
