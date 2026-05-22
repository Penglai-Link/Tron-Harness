# Backend Task: App Management

## Execution Contract

- feature_slug: app-management
- status: complete
- execution_mode: solo

## TDD Contract

Backend execution covered the new app domain, HTTP handlers, and task app validation inside one bounded slice because the write scope was shared.

## Task Rows

- AC-APP-04 | phase: tdd | owner: lead | scope: `tron-backend/src/domain/app.rs`, `tron-backend/src/application/app.rs`, archive/restore handlers | command_ref: CMD-BE-TEST | status: done.
- AC-APP-05 | phase: tdd | owner: lead | scope: `tron-backend/src/adapters/inbound/http/apps/*`, `tron-backend/src/app.rs`, workspace-scoped app routes | command_ref: CMD-BE-TEST | status: done.
- AC-APP-06 | phase: tdd | owner: lead | scope: `tron-backend/src/application/task.rs`, `tron-backend/src/domain/task.rs`, task error mapping | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

- Implemented directly in the base branch worktree because no delegated worker was requested in this turn.

## Integration Notes

- `TaskService` now depends on `AppService`.
- `TASK_APP_INVALID` is the shared task-domain error for invalid app selection.

## Open Decisions

- None.
