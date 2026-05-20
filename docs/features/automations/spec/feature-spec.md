# Feature Spec: Automations

## Initial Agent Draft

- Build the missing `automations` feature loop on top of the existing Tron frontend stubs and current Rust backend.
- Treat the current frontend pages as a visual starting point, but simplify the product scope to workspace-scoped scheduled prompt execution only.
- Add one missing detail page so list, create, update, status transitions, manual run, and execution history all close in one feature slice.

## Feature Identity And Context

- feature_name: Automations
- feature_slug: automations
- feature_type: feature_evolution
- dependency_features: auth, workspace-management, common-snowflake-id, init-rust-backend
- dependency_contracts: shell workspace switcher, uniform API response envelope, authenticated protected routes, SQLite persistence conventions
- primary_users: workspace operators, AI workflow maintainers
- business_goal: let an operator create, monitor, pause, archive, restore, and manually execute scheduled prompt automations inside the currently selected workspace

## Requirement

- The current frontend already exposes `/automations` and `/automations/new`, but the data is mock-only and there is no backend automation domain.
- Automations must belong to the workspace selected from the shell navigation switcher.
- This version excludes import and sync flows; related UI affordances must not be shown.
- Automation lifecycle status is `Active | Paused | Archived`, and only `Active` records are eligible for scheduling.
- Each automation stores a long-form prompt body similar to task descriptions.
- Each automation must expose the latest execution result status and a dedicated execution-instance history.

## Scope Boundary

- Deliver one workspace-scoped scheduled automation model, one execution-record model, authenticated CRUD-like APIs, scheduler wiring, and the corresponding frontend list/create/detail flows.
- Do not implement external app import, remote sync, distributed multi-node coordination, or external agent-runtime execution in this slice.

## Assumptions

- An in-process scheduler is sufficient for the current single-backend runtime.
- Cron input is entered in the common five-field format from the frontend and normalized for the Rust scheduler internally.
- Execution behavior in this slice records and publishes a prompt-dispatch result instead of invoking a remote agent runtime.
- Existing workspace access rules remain authoritative for whether a workspace is selectable in the shell.

## In Scope

- Workspace-scoped automation list, create, detail, update, activate, pause, archive, restore, and run-now flows.
- Backend automation persistence in SQLite.
- Backend execution-instance persistence in SQLite.
- In-process Active-only scheduling on backend startup and after lifecycle mutations.
- Latest execution rollup on automation records.
- Frontend execution history rendering and last-result rendering.
- Removal of unsupported import/sync controls from the automations surface.

## Out Of Scope

- Import automation definitions from files or third-party systems.
- Sync automation definitions to external schedulers.
- Event-driven, webhook-driven, or DAG-style automation triggers.
- Per-automation permission matrices beyond the current authenticated workspace shell context.
- Distributed locking or leader election for multi-instance scheduler ownership.

## Constraints

- Use the existing `Makefile` commands for build and test entry points.
- Keep the backend response envelope aligned with the project API rule: `code`, `message`, and `data`.
- Keep frontend behavior compatible with the current shell workspace switcher and active workspace semantics.
- Only `Active` automations may be registered with the scheduler.
- Archived automations remain read-visible through explicit filters but cannot be edited or scheduled.
- Long prompt text must be editable in a larger text area on create/detail views.

## Feature Model

- Workspace operator selects the active workspace in the shell header.
- `/automations` lists automations only for that active workspace, with search and status filters.
- Each automation owns:
  - automation code
  - workspace code
  - name
  - optional description
  - long prompt text
  - cron expression entered by the operator
  - normalized scheduler expression used by the backend
  - timezone label
  - lifecycle status
  - previous lifecycle status for restore
  - latest execution rollup fields
  - timestamps
- Each execution instance owns:
  - execution code
  - automation code
  - workspace code
  - trigger source
  - execution status
  - summary or error text
  - scheduled, started, and finished timestamps
- Manual run uses the same execution-record model as scheduled runs.
- Scheduler registration follows automation lifecycle:
  - create Active -> register
  - create Paused -> persist only
  - pause -> unregister
  - activate -> register
  - archive -> unregister
  - restore to Active -> register
  - restore to Paused -> persist only

## Page And Module Surface

- Frontend pages:
  - `/automations`: workspace-scoped management list with last execution summary, next run preview, and direct navigation to detail
  - `/automations/new`: creation flow with long prompt editor and lifecycle default
  - `/automations/detail?automationCode=<code>`: editable detail page with lifecycle actions, run-now action, and execution history
- Frontend modules:
  - `tron-frontend/src/domain/automation/*`
  - router registration and shell navigation surfaces
  - Playwright smoke coverage for the automation loop
- Backend modules:
  - `tron-backend/src/domain/automation.rs`
  - `tron-backend/src/application/automation.rs`
  - `tron-backend/src/adapters/outbound/persistence/sqlite/automation_repository.rs`
  - `tron-backend/src/adapters/inbound/http/automations/*`
  - backend startup wiring in `tron-backend/src/app.rs`

## Repository Findings

- The frontend already registers `automations.index` and `automations.new` routes and navigation entries.
- `AutomationsPage.tsx` is currently a mock table plus split inspector with non-persistent fields such as dependencies, logs, and traces.
- `CreateAutomationPage.tsx` is currently a mock-only form with unsupported concepts like webhook or event-driven trigger types.
- `CreateTaskPage.tsx`, `TaskDetailsPage.tsx`, and `VariableManagementPage.tsx` demonstrate the current frontend interaction patterns for workspace-scoped forms, editable detail panels, and mock/api fallback.
- The backend already has task and variable domains, SQLite repositories, authenticated Axum routes, and event bus publication patterns, but no automation domain exists yet.

## Open Decisions

- Default timezone for this slice is `UTC`; timezone selection UI is deferred until a real cross-timezone operator requirement appears.
- Execution records will represent prompt dispatch completion rather than downstream agent-runtime completion in this slice.
