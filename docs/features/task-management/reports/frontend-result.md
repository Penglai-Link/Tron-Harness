# Frontend Report: Task Management

## Summary

Frontend task pages consume the final task API contract in API mode while keeping mock parity. Import/export task entry points are absent from v1.

## Evidence Rows

- AC-TASK-07 | red: task pages did not prove API-backed create/detail/status/subtask/archive/restore flow | green: `rtk npm --prefix tron-frontend exec playwright -- test tests/task-management-smoke.spec.ts --reporter=line` passed | command_ref: CMD-TASK-BROWSER | browser: clicks=create/save/update status/add subtask/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive/restore asserted; feedback=created/updated/status/subtask/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-08 | red: legacy task import route and board action existed before cleanup | green: `rtk make fe-check` passed after route and board action cleanup | command_ref: CMD-FE-CHECK | browser: clicks=board sync/create actions inspected; inputs=workspace/search/status filter; network=options/kanban asserted; feedback=refresh Toast and no import/export controls; artifacts=Playwright line output | status: pass.

## Changed Paths

- `tron-frontend/src/domain/task/api/taskApi.ts`
- `tron-frontend/src/domain/task/model/TaskModel.ts`
- `tron-frontend/src/domain/task/hooks/**`
- `tron-frontend/src/domain/task/mock/data.ts`
- `tron-frontend/src/domain/task/pages/TasksKanbanPage.tsx`
- `tron-frontend/src/domain/task/pages/CreateTaskPage.tsx`
- `tron-frontend/src/domain/task/pages/TaskDetailsPage.tsx`
- `tron-frontend/src/router/routes.tsx`
- `tron-frontend/src/router/pageRegistry.ts`
- `tron-frontend/tests/task-management-smoke.spec.ts`

## Commands

- `rtk make fe-check`: pass.
- `rtk npm --prefix tron-frontend exec playwright -- test tests/task-management-smoke.spec.ts --reporter=line`: pass.

## Blockers

- None.
