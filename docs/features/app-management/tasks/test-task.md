# Test Task: App Management

## Execution Contract

- feature_slug: app-management
- status: complete
- execution_mode: solo

## TDD Contract

Verification reused focused backend and Playwright evidence instead of introducing a separate verification worker.

## Task Rows

- AC-APP-01 | phase: verify | owner: lead | scope: `tron-frontend/tests/app-management-smoke.spec.ts` list/filter/import-removal checks | command_ref: CMD-PW-APP | status: done.
- AC-APP-02 | phase: verify | owner: lead | scope: `tron-frontend/tests/app-management-smoke.spec.ts` create-page flow | command_ref: CMD-PW-APP | status: done.
- AC-APP-03 | phase: verify | owner: lead | scope: `tron-frontend/tests/app-management-smoke.spec.ts` detail/edit flow | command_ref: CMD-PW-APP | status: done.
- AC-APP-04 | phase: verify | owner: lead | scope: backend lifecycle coverage plus browser archive/restore flow | command_ref: CMD-PW-APP | status: done.
- AC-APP-05 | phase: verify | owner: lead | scope: `tron-backend/tests/app_management_api.rs` | command_ref: CMD-BE-TEST | status: done.
- AC-APP-06 | phase: verify | owner: lead | scope: `tron-backend/tests/task_management_api.rs`, `tron-frontend/tests/task-management-smoke.spec.ts` | command_ref: CMD-PW-TASK | status: done.

## Worktree Notes

- Verification ran in the base worktree after implementation completion.

## Integration Notes

- Playwright specs mock `GET /api/v1/admin/workspaces/switcher` because shell mount depends on it.

## Open Decisions

- None.
