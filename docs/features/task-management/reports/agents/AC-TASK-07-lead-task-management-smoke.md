# Browser Verify Evidence: AC-TASK-07

## Interaction: AC-TASK-07

### Clicks
- Open `/tasks`.
- Select `Archived` in the board status filter.
- Click `Create Task`.
- Click create submit on `/tasks/new`.
- Click `Save Details`.
- Click `Update Status`.
- Click `Add Subtask`.
- Click `Execute Task`.
- Click `Archive`.
- Click `Restore`.

### Inputs
- Task title: `Playwright created task`.
- Task type: `BugFix`.
- Priority: `High`.
- Source type: `GitHub`.
- Source URL: `https://github.com/clouds3n/tron/issues/100`.
- Assignee user codes: `USR-AUTH-0001`.
- Agent codes: `agent-main, agent-reviewer`.
- App codes: `github, notion`.
- Description: `Created through the task-management smoke.`
- Status: `Testing`.
- Progress: `55`.
- New subtask title: `Smoke child task`.

### Network Assertions
- `GET /api/v1/tasks/options` returns backend-owned task enums.
- `GET /api/v1/tasks/kanban?workspaceCode=TRON_CORE` renders active board without archived starter.
- `GET /api/v1/tasks/kanban?workspaceCode=TRON_CORE&status=Archived&includeArchived=true` renders archived starter.
- `POST /api/v1/tasks` includes `taskType=BugFix`, `sourceType=GitHub`, and the GitHub source URL.
- `GET /api/v1/tasks/TSK-PW-0001` loads created detail.
- `PATCH /api/v1/tasks/TSK-PW-0001` sends detail update with `taskType=Refactor`.
- `PATCH /api/v1/tasks/TSK-PW-0001/status` sends `status=Testing` and `progress=55`.
- `POST /api/v1/tasks` for subtask sends `parentTaskCode=TSK-PW-0001` and `taskType=Etc`.
- `POST /api/v1/tasks/TSK-PW-0001/archive` is called.
- `POST /api/v1/tasks/TSK-PW-0001/restore` is called.

### Visible Feedback Assertions
- Board heading `Task Management` is visible.
- Default board shows `Design task management v1` and omits Import and Export actions.
- Archived filter shows `Archived task example`.
- Create flow navigates to `/tasks/detail?taskCode=TSK-PW-0001`.
- Toast `Task TSK-PW-0001 created` is visible.
- Toast `Task TSK-PW-0001 updated` is visible.
- Toast `Task TSK-PW-0001 status updated` is visible.
- Toast `Subtask TSK-PW-0002 created` is visible and subtask title is visible.
- Toast `Task execution is not available in v1` is visible.
- Toast `Task TSK-PW-0001 archived` is visible and `Restore` button appears.
- Toast `Task TSK-PW-0001 restored` is visible and `Archive` button appears.

### Artifacts
- command: `rtk npm --prefix tron-frontend exec playwright -- test tests/task-management-smoke.spec.ts --reporter=line`
- output: 1 passed, 8.9s.
- screenshot: not generated for this focused smoke run.
- trace: not generated for this focused smoke run.
- console_log: Playwright line output captured in terminal.
- video: not generated.

### Status: pass
