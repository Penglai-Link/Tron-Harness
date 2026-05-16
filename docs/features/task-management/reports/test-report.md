# Test Report: Task Management

## Summary

Backend, frontend, and browser verification passed for the final task-management v1 contract.

## Evidence Rows

- AC-TASK-01 | red: missing options contract | green: `rtk cargo test --manifest-path tron-backend/Cargo.toml --test task_management_api` passed | command_ref: CMD-BE-TEST | browser: clicks=create/save/update status/add subtask/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive/restore asserted; feedback=created/updated/status/subtask/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-02 | red: missing kanban/list contract | green: task API test and `rtk make be-test` passed | command_ref: CMD-BE-TEST | browser: clicks=create/save/update status/add subtask/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive/restore asserted; feedback=created/updated/status/subtask/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-03 | red: create validation lacked final type/source/sourceUrl fields | green: task API create test and `rtk make be-test` passed | command_ref: CMD-BE-TEST | browser: clicks=create/save/update status/add subtask/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive/restore asserted; feedback=created/updated/status/subtask/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-04 | red: subtask same-workspace validation missed final model | green: task API subtask test and `rtk make be-test` passed | command_ref: CMD-BE-TEST | browser: clicks=create/save/update status/add subtask/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive/restore asserted; feedback=created/updated/status/subtask/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-05 | red: status/progress update route was missing | green: task API lifecycle test and `rtk make be-test` passed | command_ref: CMD-BE-TEST | browser: clicks=create/save/update status/add subtask/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive/restore asserted; feedback=created/updated/status/subtask/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-06 | red: archive/restore lifecycle was missing | green: task API lifecycle test and `rtk make be-test` passed | command_ref: CMD-BE-TEST | browser: clicks=create/save/update status/add subtask/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive/restore asserted; feedback=created/updated/status/subtask/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-07 | red: task UI lacked API-backed interaction proof | green: Playwright task workflow passed | command_ref: CMD-TASK-BROWSER | browser: clicks=create/save/update status/add subtask/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive/restore asserted; feedback=created/updated/status/subtask/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-08 | red: import/export controls were present in task navigation before cleanup | green: board action assertions and `rtk make fe-check` passed | command_ref: CMD-FE-CHECK | browser: clicks=board sync/create actions inspected; inputs=workspace/search/status filter; network=options/kanban asserted; feedback=refresh Toast and no import/export controls; artifacts=Playwright line output | status: pass.

## Changed Paths

- `tron-backend/**/task*`
- `tron-backend/migrations/20260516_0008_create_task_management_tasks.sql`
- `tron-frontend/src/domain/task/**`
- `tron-frontend/src/router/routes.tsx`
- `tron-frontend/src/router/pageRegistry.ts`
- `tron-frontend/tests/task-management-smoke.spec.ts`

## Commands

- `rtk make be-check`: pass.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test task_management_api`: pass.
- `rtk make be-test`: pass.
- `rtk make fe-check`: pass.
- `rtk npm --prefix tron-frontend exec playwright -- test tests/task-management-smoke.spec.ts --reporter=line`: pass.

## Blockers

- None.
