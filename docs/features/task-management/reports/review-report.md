# Review Report: Task Management

## Summary

Review completed against the approved superpower status evolution and the kanban drag/drop status update evolution. The task product surface remains intentionally lightweight; strict adjacent transition graphs, touch-first drag behavior, and execution orchestration UI remain future scope.

## Evidence Rows

- AC-TASK-01 | red: frontend and backend enum ownership could drift after the evolution | green: backend options plus frontend task model/mock/browser evidence all align on the final role-aware contract | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-02 | red: archived filtering or workspace scoping could regress while resetting backend data | green: backend list/kanban tests and browser board requests verify scoped active records by default | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-03 | red: create could bypass role-default status ownership | green: backend create tests and browser create flow verify backend-owned `Open` defaults | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-04 | red: child-role inference could break parent detail and subtask flows | green: backend parent-child tests and browser child navigation keep the parent-child model intact | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-05 | red: invalid role/state combinations could slip through task updates | green: backend tests cover invalid feature-root, feature-child, and type-change cases with `TASK_STATUS_ROLE_INVALID` | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-06 | red: archive lifecycle could still restore to old fallback states | green: backend lifecycle tests prove fallback to `Open` and keep the active-subtask parent guard | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-07 | red: browser-visible task flow could still expose old enum values | green: Playwright verification proves only `Feature | Bug` plus role-filtered status labels are visible | command_ref: CMD-TASK-BROWSER | browser: clicks=refresh board, open create dialog, save task, open detail, save detail, add subtask, follow child-parent links, trigger archive guard, archive child, archive parent, restore parent, click execute; inputs=search text, priority select, type select, source/url, assignee/app links, progress values; network=`GET /api/v1/tasks/options`, `GET /api/v1/tasks/kanban`, `POST /api/v1/tasks`, `GET /api/v1/tasks/{taskCode}`, `PATCH /api/v1/tasks/{taskCode}`, `PATCH /api/v1/tasks/{taskCode}/status`, `POST /api/v1/tasks/{taskCode}/archive`; feedback=refresh success, create success, save success, subtask success, archive guard toast, child archive success, execute toast; artifacts=Playwright line output plus request-body assertions inside `task-management-smoke.spec.ts` | status: pass.
- AC-TASK-08 | red: reset logic could either fail to clear legacy task data or clear data on every startup | green: conditional legacy reset is proven in backend tests, while fresh-schema boots remain seed-free and frontend mock/demo parity still builds | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-09 | red: review confirmed the pre-change board had clickable cards but no drag/drop status update proof | green: worker RED/GREEN plus base `make fe-check` and `make fe-test TEST=task-management-smoke.spec.ts` prove POST-only drag/drop updates, invalid-drop blocking, rollback feedback, and click navigation preservation | command_ref: CMD-TASK-BROWSER | browser: clicks=card click to detail, valid drag from Open to Green, invalid child drag to Red, same-column drag to Open, failed drag to Green; inputs=drag source cards and status-column dropzones; network=valid and failed status updates used `POST /api/v1/tasks/{taskCode}/status`, valid payload omitted `progress`, invalid and same-column drops made no status request; feedback=success toast, invalid toast, backend rejection toast, visible card move or rollback; artifacts=`tron-frontend/src/domain/task/pages/TasksKanbanPage.tsx`, `tron-frontend/src/domain/task/api/taskApi.ts`, `tron-frontend/tests/task-management-smoke.spec.ts` | status: pass.

## Changed Paths

- See backend, database, frontend, and test reports.
- AC-TASK-09 changed `tron-frontend/src/domain/task/api/taskApi.ts`, `tron-frontend/src/domain/task/pages/TasksKanbanPage.tsx`, and `tron-frontend/tests/task-management-smoke.spec.ts`.

## Commands

- `rtk make be-test`: pass.
- `rtk make fe-check`: pass.
- `rtk npm --prefix tron-frontend exec playwright -- test tests/task-management-smoke.spec.ts --reporter=line`: pass.
- `make fe-check`: pass.
- `make fe-test TEST=task-management-smoke.spec.ts`: pass, 13 tests.

## Blockers

- None.
