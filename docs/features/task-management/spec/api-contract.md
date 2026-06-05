# API Contract: Task Management

## API Overview

Protected task APIs live under `/api/v1/tasks`, require the existing authenticated session, and use the project response envelope with `code`, `message`, and `data`.

## Endpoints

### GET /api/v1/tasks/options - Task options

- purpose: expose backend-owned task enums plus role-based status rules
- auth: authenticated
- acceptance_ids: AC-TASK-01, AC-TASK-07
- parameter_passing: request headers
- request: none
- response:
  - `statuses`
  - `priorities`
  - `taskTypes`
  - `sourceTypes`
  - `statusRules: { featureRoot, featureChild, bug }`
  - `defaultStatuses: { featureRoot, featureChild, bug }`
- wire values:
  - `taskTypes`: `Feature`, `Bug`
  - `statuses`: `Open`, `Designing`, `Red`, `Green`, `Regressioning`, `Review`, `InProgress`, `Reopen`, `Completed`, `Archived`
- client_impact: frontend task selectors and role-aware detail status dropdown consume this response in API mode
- compatibility: replaces the old v1 global-only enum payload

### GET /api/v1/tasks/kanban - Task kanban

- purpose: return status-grouped task cards for one workspace
- auth: authenticated
- acceptance_ids: AC-TASK-02, AC-TASK-07
- parameter_passing: query parameters
- request: `workspaceCode`, optional `search`, `status`, `priority`, `taskType`, `sourceType`, `assigneeUserCode`, `includeArchived`
- response: array of kanban columns with task cards
- client_impact: `/tasks` board consumes this endpoint in API mode and re-renders columns after successful drag/drop status updates
- compatibility: board columns now represent the new status enum set

### GET /api/v1/tasks - Task list

- purpose: return paged searchable task records
- auth: authenticated
- acceptance_ids: AC-TASK-02
- parameter_passing: query parameters
- request: `workspaceCode`, optional `search`, `status`, `priority`, `taskType`, `sourceType`, `assigneeUserCode`, `parentTaskCode`, `includeArchived`, `current`, `size`
- response: paged records with `current`, `size`, `total`, `records`
- client_impact: supports list/search reuse and browser verification
- compatibility: unchanged envelope; enum values change

### POST /api/v1/tasks - Create task

- purpose: create a root task or same-workspace child task
- auth: authenticated
- acceptance_ids: AC-TASK-03, AC-TASK-04
- parameter_passing: request body
- request: `workspaceCode`, `title`, `priority`, `taskType`, `sourceType`, optional `sourceUrl`, optional `description`, optional `parentTaskCode`, optional `assigneeUserCodes`, optional `agentCodes`, optional `appCodes`
- response: task detail
- backend rules:
  - task role is inferred from `taskType` and `parentTaskCode`
  - default status is `Open` for every role
  - feature child is determined only when `taskType=Feature` and `parentTaskCode` is present
- client_impact: `/tasks/new` and detail subtask controls consume this endpoint
- compatibility: old `BugFix` and `Refactor` values are no longer accepted

### GET /api/v1/tasks/{taskCode} - Task detail

- purpose: return task detail with parent summary, active direct subtasks, assignments, and string integration arrays
- auth: authenticated
- acceptance_ids: AC-TASK-04, AC-TASK-07
- parameter_passing: path variables
- request: `taskCode`
- response: task detail
- client_impact: `/tasks/detail?taskCode=...` consumes this endpoint
- compatibility: unchanged envelope; returned enum values change

### PATCH /api/v1/tasks/{taskCode} - Update task

- purpose: update editable task fields, assignments, and integration string links
- auth: authenticated
- acceptance_ids: AC-TASK-05
- parameter_passing: path variables and request body
- request: optional `title`, `description`, `priority`, `taskType`, `sourceType`, `sourceUrl`, `assigneeUserCodes`, `agentCodes`, `appCodes`
- response: task detail
- backend rules:
  - archived tasks cannot be updated
  - if `taskType` changes and the current status is not allowed for the new inferred role, the request is rejected with `TASK_STATUS_ROLE_INVALID`
- client_impact: detail edit controls
- compatibility: `taskType` values are now `Feature` and `Bug`

### POST /api/v1/tasks/{taskCode}/status - Update task status

- purpose: update lifecycle status and progress
- auth: authenticated
- acceptance_ids: AC-TASK-05, AC-TASK-09
- parameter_passing: path variables and request body
- request: `status`, optional `progress`
- response: task detail
- backend rules:
  - direct status update to `Archived` is invalid; archive still uses the lifecycle route
  - requested status must belong to the current task role, otherwise return `TASK_STATUS_ROLE_INVALID`
  - `Completed` forces progress to `100`
- client_impact: detail status dropdown and kanban drag/drop persist role-aware lifecycle changes; kanban drag/drop sends only `status` so non-Completed progress is preserved by the backend
- compatibility: replaces the old global status update contract; backend also accepts PATCH for existing detail compatibility

### POST /api/v1/tasks/{taskCode}/archive - Archive task

- purpose: move a task to `Archived` and store previous status
- auth: authenticated
- acceptance_ids: AC-TASK-06
- parameter_passing: path variables
- request: `taskCode`
- response: lifecycle result
- client_impact: detail archive transition
- compatibility: unchanged route

### POST /api/v1/tasks/{taskCode}/restore - Restore task

- purpose: restore an archived task to the stored previous status or role default `Open`
- auth: authenticated
- acceptance_ids: AC-TASK-06
- parameter_passing: path variables
- request: `taskCode`
- response: lifecycle result
- client_impact: detail restore transition
- compatibility: fallback status changes from `Backlog` to `Open`

## Auth And Permissions

All task endpoints require an authenticated user through the existing backend auth middleware. This slice does not add task-specific permission checks.

## Error Contract

- missing workspace -> 400 / `TASK_WORKSPACE_REQUIRED`; unified error response; AC-TASK-03
- workspace not found or archived -> 400 / `TASK_WORKSPACE_INVALID`; unified error response; AC-TASK-03
- invalid parent or cross-workspace parent -> 400 / `TASK_PARENT_INVALID`; unified error response; AC-TASK-04
- assignee not effective in workspace -> 400 / `TASK_ASSIGNEE_INVALID`; unified error response; AC-TASK-03
- archived task mutation -> 400 / `TASK_ARCHIVED_INVALID`; unified error response; AC-TASK-05, AC-TASK-06
- parent task has active subtasks during archive -> 400 / `TASK_ACTIVE_SUBTASK_EXISTS`; unified error response; AC-TASK-06
- requested status or type/status combination does not match the inferred task role -> 400 / `TASK_STATUS_ROLE_INVALID`; unified error response; AC-TASK-05
- invalid enum, progress, or malformed request -> 400 / `TASK_REQUEST_INVALID`; unified error response where request extraction reaches the handler; AC-TASK-01, AC-TASK-03, AC-TASK-05
- task not found -> 404 / `TASK_NOT_FOUND`; unified error response; AC-TASK-04
- storage failure -> 500 / `TASK_STORAGE_FAILED`; unified error response; AC-TASK-02, AC-TASK-08

## Frontend Backend Boundary

The backend owns enum values, task role inference, allowed status sets, default statuses, validation, persistence, taskCode generation, same-workspace parent checks, active-subtask archive guards, effective-user assignment checks, status/progress rules, archive/restore rules, and agent/app string-link persistence. The frontend owns interaction layout, workspace/user selector UX, read-only/edit detail mode, API-mode hooks, mock/demo parity, debounced search input, drag/drop affordances, local prevalidation against `statusRules`, optimistic board feedback, rollback on status-update failure, and shared Toast feedback.

## Database And Transaction Boundary

Create, update, status update, lifecycle update, and assignment/link replacement writes are persisted through the task repository. Legacy task schema reset is a one-time conditional backend action when the repository detects old task enum constraints.

## Mock Or Adapter Expectations

Frontend task mocks must mirror backend enum values, response shapes, task role inference, role-based status filtering, default `Open` creation, archive/restore fallback behavior, and agent/app string arrays. Browser tests may intercept `/api/v1/tasks/**` but must assert real request paths, payloads, and role-aware status lists.
