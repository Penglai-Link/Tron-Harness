# Frontend Report: Task Management

## Summary

Frontend task pages now consume the role-aware options contract in API mode, keep mock/demo parity, preserve the debounced board search, and stop surfacing old task enums.

## Evidence Rows

- AC-TASK-07 | red: task pages still exposed old task types/statuses or global status choices | green: `rtk npm --prefix tron-frontend exec playwright -- test tests/task-management-smoke.spec.ts --reporter=line` passed with create/detail/subtask/archive flows under the new role-aware contract | command_ref: CMD-TASK-BROWSER | browser: clicks=refresh board, open create dialog, save task, open detail, save detail, add subtask, follow child-parent links, trigger archive guard, archive child, archive parent, restore parent, click execute; inputs=search text, priority select, type select, source/url, assignee/app links, progress values; network=`GET /api/v1/tasks/options`, `GET /api/v1/tasks/kanban`, `POST /api/v1/tasks`, `GET /api/v1/tasks/{taskCode}`, `PATCH /api/v1/tasks/{taskCode}`, `PATCH /api/v1/tasks/{taskCode}/status`, `POST /api/v1/tasks/{taskCode}/archive`; feedback=refresh success, create success, save success, subtask success, archive guard toast, child archive success, execute toast; artifacts=Playwright line output plus request-body assertions inside `task-management-smoke.spec.ts` | status: pass.
- AC-TASK-08 | red: frontend mock/demo data still mirrored the legacy task status/type set | green: `rtk make fe-check` passed after updating the task model, mock data, and browser fixtures to the new contract | command_ref: CMD-FE-CHECK | browser:not applicable | status: pass.

## Changed Paths

- `tron-frontend/src/domain/task/api/taskApi.ts`
- `tron-frontend/src/domain/task/constants/taskPageOptions.ts`
- `tron-frontend/src/domain/task/model/TaskModel.ts`
- `tron-frontend/src/domain/task/mock/data.ts`
- `tron-frontend/src/domain/task/pages/TaskDetailsPage.tsx`
- `tron-frontend/src/domain/task/pages/TasksKanbanPage.tsx`
- `tron-frontend/tests/task-management-smoke.spec.ts`

## Commands

- `rtk make fe-check`: pass.
- `rtk npm --prefix tron-frontend exec playwright -- test tests/task-management-smoke.spec.ts --reporter=line`: pass.

## Blockers

- None.
