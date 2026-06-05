# Worktree Report: Task Management

## Summary

Feature branch `feature/xinlangtao/task-card-drag-status/base` completed the kanban drag/drop status update evolution after merging frontend worker branch `feature/xinlangtao/task-card-drag-status/frontend/kanban-drag-update`.

## Evidence Rows

- AC-TASK-01 | red: feature branch started without role-aware options | green: backend options contract verified | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-02 | red: feature branch started without empty-seed kanban/list verification | green: workspace-scoped kanban/list verified after seed removal | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-03 | red: feature branch started without role-default `Open` creation | green: create validation and defaults verified | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-04 | red: feature branch started without updated parent-child role handling | green: parent-child detail and subtask inheritance verified | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-05 | red: feature branch started without role-incompatible status rejection | green: invalid status/type combinations verified through backend tests | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-06 | red: feature branch started without restore fallback evolution | green: archive guard and restore-to-open behavior verified | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-07 | red: feature branch started with browser-visible old enums | green: browser workflow passed with only the new visible enums | command_ref: CMD-TASK-BROWSER | browser: clicks=refresh board, open create dialog, save task, open detail, save detail, add subtask, follow child-parent links, trigger archive guard, archive child, archive parent, restore parent, click execute; inputs=search text, priority select, type select, source/url, assignee/app links, progress values; network=`GET /api/v1/tasks/options`, `GET /api/v1/tasks/kanban`, `POST /api/v1/tasks`, `GET /api/v1/tasks/{taskCode}`, `PATCH /api/v1/tasks/{taskCode}`, `PATCH /api/v1/tasks/{taskCode}/status`, `POST /api/v1/tasks/{taskCode}/archive`; feedback=refresh success, create success, save success, subtask success, archive guard toast, child archive success, execute toast; artifacts=Playwright line output plus request-body assertions inside `task-management-smoke.spec.ts` | status: pass.
- AC-TASK-08 | red: feature branch started with legacy task schema/data and backend seed rows | green: conditional legacy reset plus seed removal verified | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-09 | red: worker branch proved the board lacked drag/drop status behavior before implementation | green: worker commit `dbe20cd` merged into base; base `make fe-check` and `make fe-test TEST=task-management-smoke.spec.ts` passed | command_ref: CMD-TASK-BROWSER | browser: clicks=click card, drag valid card to Green, drag invalid child card to Red, drag failure fixture to Green; inputs=HTML5 drag/drop source cards and target status columns; network=POST status request with `{status}` and no progress for valid drop, no request for invalid/same-column drops, failed POST request returned backend error; feedback=success, invalid, and backend-error toasts plus visible move/rollback; artifacts=worker branch diff, merge commit, Playwright smoke output | status: pass.

## Changed Paths

- `docs/features/task-management/**`
- `tron-backend/**/task*`
- `tron-backend/migrations/20260516_0008_create_task_management_tasks.sql`
- `tron-backend/migrations/20260519_0010_reset_task_management_superpower.sql`
- `tron-frontend/src/domain/task/**`
- `tron-frontend/tests/task-management-smoke.spec.ts`
- `docs/brainstorm/task-card-drag-status/2026-06-05-09-10-25.md`
- `docs/prd.typ`
- `docs/prd.pdf`

## Commands

- `rtk make be-test`: pass.
- `rtk make fe-check`: pass.
- `rtk npm --prefix tron-frontend exec playwright -- test tests/task-management-smoke.spec.ts --reporter=line`: pass.
- `make fe-install`: pass.
- `make fe-check`: pass.
- `make fe-test TEST=task-management-smoke.spec.ts`: pass, 13 tests.

## Blockers

- None.
