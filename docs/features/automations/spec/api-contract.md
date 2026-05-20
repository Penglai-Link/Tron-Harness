# API Contract: Automations

## API Overview

- Base domain: `/api/v1/automations`
- Envelope:
  - success: `{ code: "SUCCESS", message: "success", data: ... }`
  - error: `{ code: "<DOMAIN_ERROR>", message: "<human readable>", data: {} }`
- Paginated list shape follows the project `current | size | total | records` convention inside `data`.
- All routes are authenticated protected routes under the existing Axum auth middleware.

## Endpoints

- `GET /api/v1/automations/options`
  - returns automation statuses and execution statuses for frontend rendering
- `GET /api/v1/automations`
  - query: `workspaceCode`, `current`, `size`, `search`, `status`, `includeArchived`
  - returns paged workspace-scoped automation summaries
- `POST /api/v1/automations`
  - body: `workspaceCode`, `name`, `description`, `prompt`, `cronExpression`, optional `status`
  - returns created automation detail
- `GET /api/v1/automations/{automationCode}`
  - returns automation detail with latest execution rollup
- `PATCH /api/v1/automations/{automationCode}`
  - body: editable metadata fields only
  - updates scheduling registration when cron or status-affecting fields change
- `POST` or `PATCH /api/v1/automations/{automationCode}/status`
  - body: `status`
  - supports `Active` and `Paused`
- `POST /api/v1/automations/{automationCode}/archive`
  - archives and unregisters scheduler job
- `POST /api/v1/automations/{automationCode}/restore`
  - restores to previous status or default `Paused`
- `POST /api/v1/automations/{automationCode}/execute`
  - creates a manual execution instance and updates latest execution rollup
- `GET /api/v1/automations/{automationCode}/executions`
  - query: `current`, `size`
  - returns paged execution-instance history ordered newest first

## Auth And Permissions

- Requests require the existing authenticated Tron session.
- Workspace scoping is enforced by the request payload or query `workspaceCode` and the stored automation workspace relation.
- This slice does not introduce per-automation ACLs beyond the current authenticated operator model.

## Error Contract

- `AUTOMATION_NOT_FOUND`
- `AUTOMATION_WORKSPACE_REQUIRED`
- `AUTOMATION_WORKSPACE_INVALID`
- `AUTOMATION_NAME_REQUIRED`
- `AUTOMATION_PROMPT_REQUIRED`
- `AUTOMATION_CRON_INVALID`
- `AUTOMATION_STATUS_INVALID`
- `AUTOMATION_ARCHIVED_INVALID`
- `AUTOMATION_EXECUTION_FAILED`
- `AUTOMATION_STORAGE_FAILED`
- `AUTOMATION_EXECUTION_NOT_FOUND` is not needed in this slice because execution detail routing is not exposed separately.

## Frontend Backend Boundary

- Frontend owns workspace selection, form state, optimistic button loading, and route navigation.
- Backend owns automation code generation, cron normalization, lifecycle validation, execution-record creation, and Active-only scheduler registration.
- Frontend sends five-field cron text as authored by the operator; backend validates and normalizes it for the scheduler.
- Frontend should not render import or sync actions for this slice.

## Database And Transaction Boundary

- Create, update, archive, restore, pause, activate, and execute operations persist to SQLite first.
- Scheduler registration or deregistration runs after successful persistence so runtime state mirrors durable state.
- Manual and scheduled execution writes must create a history record and then update the parent automation rollup in the same repository workflow.

## Mock Or Adapter Expectations

- Frontend mock adapters must mirror the backend route surface, lifecycle behavior, and latest-execution rollup shape.
- Mock execution records should demonstrate `Manual` and `Scheduler` trigger sources.
- Mock list behavior must stay workspace-scoped and hide archived records by default unless explicitly requested.
