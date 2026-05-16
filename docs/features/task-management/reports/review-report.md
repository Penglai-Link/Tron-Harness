# Review Report: Task Management

## Summary

Review completed against the final v1 plan. Execution engine, agent management, app management, and import/export features remain out of scope.

## Evidence Rows

- AC-TASK-01 | red: final enum ownership could drift between frontend and backend | green: backend options endpoint and frontend options hook share the final enum contract | command_ref: CMD-BE-TEST | browser: clicks=create/edit/save/add subtask/child-parent links/archive guard/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive guard/archive/restore asserted; feedback=created/saved/subtask/archive guard/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-02 | red: kanban/list could return cross-workspace or archived records by default | green: workspace-scoped list and archived filter behavior are verified | command_ref: CMD-BE-TEST | browser: clicks=create/edit/save/add subtask/child-parent links/archive guard/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive guard/archive/restore asserted; feedback=created/saved/subtask/archive guard/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-03 | red: create could bypass workspace or assignee rules | green: active workspace and effective-user assignee validation are verified | command_ref: CMD-BE-TEST | browser: clicks=create/edit/save/add subtask/child-parent links/archive guard/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive guard/archive/restore asserted; feedback=created/saved/subtask/archive guard/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-04 | red: parent/child modeling could allow cross-workspace parent links or count archived subtasks | green: detail, active-subtask filtering, and same-workspace parent creation are verified | command_ref: CMD-BE-TEST | browser: clicks=create/edit/save/add subtask/child-parent links/archive guard/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive guard/archive/restore asserted; feedback=created/saved/subtask/archive guard/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-05 | red: detail edits could expose a separate status-only UI path | green: frontend uses one edit/save flow while backend field and status/progress updates remain verified | command_ref: CMD-BE-TEST | browser: clicks=create/edit/save/add subtask/child-parent links/archive guard/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive guard/archive/restore asserted; feedback=created/saved/subtask/archive guard/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-06 | red: archive could lose previous status, allow normal mutation, or archive parents with active subtasks | green: previous-status restore, archived mutation guard, and active-subtask parent guard are verified | command_ref: CMD-BE-TEST | browser: clicks=create/edit/save/add subtask/child-parent links/archive guard/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive guard/archive/restore asserted; feedback=created/saved/subtask/archive guard/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-07 | red: UI flow could remain mock-only | green: API-backed browser workflow verified visible feedback and network assertions | command_ref: CMD-TASK-BROWSER | browser: clicks=create/edit/save/add subtask/child-parent links/archive guard/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive guard/archive/restore asserted; feedback=created/saved/subtask/archive guard/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-08 | red: task board could still expose v1-out-of-scope import/export actions | green: normal board actions expose only refresh and create | command_ref: CMD-FE-CHECK | browser: clicks=board refresh/create actions inspected; inputs=workspace/search/priority filter; network=options/kanban asserted; feedback=refresh Toast and no import/export controls; artifacts=Playwright line output | status: pass.

## Changed Paths

- See backend, database, frontend, and test reports.

## Commands

- `rtk make be-check`: pass.
- `rtk make be-test`: pass.
- `rtk make fe-check`: pass.
- `rtk npm --prefix tron-frontend exec playwright -- test tests/task-management-smoke.spec.ts --reporter=line`: pass.

## Blockers

- None.
