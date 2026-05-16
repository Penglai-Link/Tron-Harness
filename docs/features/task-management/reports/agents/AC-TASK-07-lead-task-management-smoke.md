# Browser Verify Evidence: AC-TASK-07

## Interaction: AC-TASK-07

### Clicks
- Open `/tasks`.
- Click `Refresh`.
- Select `High` in the board priority filter.
- Click `Create Task`.
- Click create submit on `/tasks/new`.
- Click `Edit`.
- Click `Save`.
- Click `Add Subtask`.
- Click the child task link.
- Click the parent task link.
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
- Agent codes: `agent-reviewer, agent-builder`.
- App codes: `github, notion`.
- Description: `Created through the task-management smoke.`
- Status: `Testing`.
- Progress: `55`.
- New subtask title: `Smoke child task`.

### Network Assertions
- `GET /api/v1/tasks/options` returns backend-owned task enums.
- `GET /api/v1/tasks/kanban?workspaceCode=TRON_CORE` renders active board without archived starter.
- `GET /api/v1/tasks/kanban?workspaceCode=TRON_CORE&priority=High` renders high-priority board tasks.
- `POST /api/v1/tasks` includes `taskType=BugFix`, `sourceType=GitHub`, and the GitHub source URL.
- `GET /api/v1/tasks/TSK-PW-0001` loads created detail.
- `PATCH /api/v1/tasks/TSK-PW-0001` sends detail update with `taskType=Refactor`.
- `PATCH /api/v1/tasks/TSK-PW-0001/status` sends `status=Testing` and `progress=55`.
- `POST /api/v1/tasks` for subtask sends `parentTaskCode=TSK-PW-0001` and `taskType=Feature`.
- `POST /api/v1/tasks/TSK-PW-0001/archive` first returns `TASK_ACTIVE_SUBTASK_EXISTS`.
- `GET /api/v1/tasks/TSK-PW-0002` loads child detail with parent link.
- `POST /api/v1/tasks/TSK-PW-0002/archive` is called before parent archive succeeds.
- `GET /api/v1/tasks/TSK-PW-0001` after child archive omits archived subtasks.
- `POST /api/v1/tasks/TSK-PW-0001/archive` is called after active subtasks are archived.
- `POST /api/v1/tasks/TSK-PW-0001/restore` is called.

### Visible Feedback Assertions
- Board heading `Task Management` is visible.
- Board header omits the old `Kanban` tag, shows `Refresh`, and omits `Sync`.
- Default board shows `Design task management v1` and omits Import and Export actions.
- Task type and source type badges use different visual classes.
- Create form task type options omit `Etc`.
- Create form description textarea uses full-width styling.
- Priority filter shows `Verify task smoke flow`.
- Toast `Task board refreshed` is visible.
- Create flow navigates to `/tasks/detail?taskCode=TSK-PW-0001`.
- Task detail defaults to read-only data, has an `Edit` button, has no task detail tags, and has no `Update Status` button.
- Toast `Task TSK-PW-0001 created` is visible.
- Toast `Task TSK-PW-0001 saved` is visible.
- Toast `Subtask TSK-PW-0002 created` is visible and subtask title is visible.
- Parent archive with active child shows `archive active subtasks before archiving the parent task`.
- Child detail links back to parent detail, and archived child is hidden from the parent subtask list.
- Toast `Task TSK-PW-0002 archived` is visible.
- Toast `Task execution is not available in v1` is visible.
- Toast `Task TSK-PW-0001 archived` is visible and `Restore` button appears.
- Toast `Task TSK-PW-0001 restored` is visible and `Archive` button appears.

### Artifacts
- command: `rtk npm --prefix tron-frontend exec playwright -- test tests/task-management-smoke.spec.ts --reporter=line`
- output: 1 passed, 15.0s.
- screenshot: not generated for this focused smoke run.
- trace: not generated for this focused smoke run.
- console_log: Playwright line output captured in terminal.
- video: not generated.

### Status: pass
