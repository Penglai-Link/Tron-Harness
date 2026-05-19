# Browser Verify Evidence: AC-TASK-07

## Interaction: AC-TASK-07

### Clicks
- Open `/tasks`.
- Click `Refresh`.
- Select `High` in the board priority filter.
- Click `Create Task`.
- Click create submit on `/tasks/new`.
- Click `Edit Details`.
- Click `Save Details`.
- Click `Add Subtask`.
- Click the child task link.
- Click the parent task link.
- Click `Execute`.
- Drive archive and restore through the status dropdown.

### Inputs
- Task title: `Playwright created task`.
- Task type: `Feature`.
- Priority: `High`.
- Source type: `GitHub`.
- Source URL: `https://github.com/clouds3n/tron/issues/100`.
- Assignee user codes: `USR-AUTH-0001`.
- Agent codes: `agent-reviewer, agent-builder`.
- App codes: `github, notion`.
- Description: `Created through the task-management smoke.`
- Root feature status: `Green`.
- Progress: `55`.
- New subtask title: `Smoke child task`.

### Network Assertions
- `GET /api/v1/tasks/options` returns `Feature | Bug`, the new status universe, `statusRules`, and `defaultStatuses`.
- `GET /api/v1/tasks/kanban?workspaceCode=TRON_CORE` renders the active board without archived tasks.
- `GET /api/v1/tasks/kanban?workspaceCode=TRON_CORE&priority=High` renders the seeded bug board task.
- `POST /api/v1/tasks` includes `taskType=Feature`, `sourceType=GitHub`, and the GitHub source URL.
- `GET /api/v1/tasks/TSK-PW-0001` loads created detail.
- `PATCH /api/v1/tasks/TSK-PW-0001` sends detail update with `sourceType=GitLab` and the updated title.
- `PATCH /api/v1/tasks/TSK-PW-0001/status` sends `status=Green` and `progress=55`.
- `POST /api/v1/tasks` for subtask sends `parentTaskCode=TSK-PW-0001` and `taskType=Feature`.
- `POST /api/v1/tasks/TSK-PW-0001/archive` first returns `TASK_ACTIVE_SUBTASK_EXISTS`.
- `GET /api/v1/tasks/TSK-PW-0002` loads child detail with parent link and feature-child status choices.
- `POST /api/v1/tasks/TSK-PW-0002/archive` is called before parent archive succeeds.
- `GET /api/v1/tasks/TSK-PW-0001` after child archive omits archived subtasks.
- The restore leg is protected by a mock `/status` rejection for archived tasks, so the final `Open` state proves the frontend restored before updating status.

### Visible Feedback Assertions
- Board heading `Task Management` is visible.
- Board header omits the old `Kanban` tag, shows `Refresh`, and omits `Sync`.
- Default board omits Import and Export actions.
- Bug task type and source type badges use different visual classes.
- Create form task type options show only `Feature` and `Bug`.
- Detail root-feature status dropdown shows only `Open`, `Designing`, `Red`, `Green`, `Regressioning`, `Review`, `Completed`, and `Archived`.
- Child detail status dropdown shows only `Open`, `In Progress`, `Completed`, and `Archived`.
- Toast `Task board refreshed` is visible.
- Toast `Task TSK-PW-0001 updates saved` is visible after priority and status updates.
- Toast `Task TSK-PW-0001 details saved` is visible after detail edits.
- Toast `Subtask TSK-PW-0002 created` is visible.
- Parent archive with active child shows `archive active subtasks before archiving the parent task`.
- Toast `Task TSK-PW-0002 archived` is visible.
- Final restored parent status returns to `Open`.
- Toast `Task execution is not available in v1` is visible.

### Artifacts
- command: `rtk npm --prefix tron-frontend exec playwright -- test tests/task-management-smoke.spec.ts --reporter=line`
- output: 9 passed, 15.3s.
- screenshot: not generated for this focused smoke run.
- trace: not generated for this focused smoke run.
- console_log: Playwright line output captured in terminal.
- video: not generated.

### Status: pass
