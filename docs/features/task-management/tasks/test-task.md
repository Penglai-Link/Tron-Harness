# Test Task: Task Management

## Execution Contract

- feature_slug: task-management
- status: done
- execution_mode: solo

## TDD Contract

Tests must provide RED evidence before implementation and GREEN/VERIFY evidence after implementation.

## Task Rows

- AC-TASK-01 | phase: tdd | owner: lead | scope: `tron-backend/tests/task_management_api.rs` options contract and role rules | command_ref: CMD-BE-TEST | status: done.
- AC-TASK-02 | phase: tdd | owner: lead | scope: `tron-backend/tests/task_management_api.rs` list and kanban behavior under the new status universe | command_ref: CMD-BE-TEST | status: done.
- AC-TASK-03 | phase: tdd | owner: lead | scope: create default-status validation in `task_management_api.rs` | command_ref: CMD-BE-TEST | status: done.
- AC-TASK-04 | phase: tdd | owner: lead | scope: parent-child rule coverage in `task_management_api.rs` | command_ref: CMD-BE-TEST | status: done.
- AC-TASK-05 | phase: tdd | owner: lead | scope: role-incompatible status and type-change rejection coverage in `task_management_api.rs` | command_ref: CMD-BE-TEST | status: done.
- AC-TASK-06 | phase: tdd | owner: lead | scope: archive/restore fallback and active-subtask guard coverage in `task_management_api.rs` | command_ref: CMD-BE-TEST | status: done.
- AC-TASK-07 | phase: tdd | owner: lead | scope: `tron-frontend/tests/task-management-smoke.spec.ts` role-aware UI and network assertions | command_ref: CMD-TASK-BROWSER | status: done.
- AC-TASK-08 | phase: tdd | owner: lead | scope: backend reset path and frontend mock/demo parity verification | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

Lead checkout: `/Users/clouds3n/Coding/open-source/ai/Tron` on branch `feature/clouds3n/task-management`.

## Integration Notes

Browser proof must assert task API requests, not only page text. Backend tests must prove the repository starts without old seed tasks after the reset path.

## Open Decisions

- None.
