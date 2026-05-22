# API Contract: App Management

## API Overview

App management is a workspace-scoped registry API under `/api/v1/apps`. The backend owns app persistence, workspace scoping, lifecycle changes, and task-assignment validation. The frontend owns route presentation, workspace-selected queries, and preserving legacy task app codes until the operator explicitly changes app selection.

All responses use the project envelope defined in `AGENTS.md`. Paged list responses use the nested `data.current`, `data.size`, `data.total`, and `data.records` structure.

## Endpoints

### GET /api/v1/apps/workspaces/{workspaceCode} - Workspace App List

- purpose: list apps for the selected workspace with pagination and filters
- auth: current admin shell session; no feature-specific permission branch added in this slice
- acceptance_ids: AC-APP-01, AC-APP-05
- parameter_passing: path variables and query parameters
- request: `workspaceCode` path plus optional `search`, `runtimeRole`, `status`, `includeArchived`, `current`, `size`
- response: paged `AppSummary` records with `appCode`, `workspaceCode`, `name`, `description`, `runtimeRole`, `repositoryUrl`, `defaultBranch`, `tags`, `status`, `updatedAt`
- client_impact: replaces placeholder `/apps` frontend table source
- compatibility: default behavior returns active apps when no explicit archived filter is supplied

### POST /api/v1/apps/workspaces/{workspaceCode} - Create App

- purpose: create a new app inside the current workspace
- auth: current admin shell session; no feature-specific permission branch added in this slice
- acceptance_ids: AC-APP-02, AC-APP-05
- parameter_passing: path variables and request body
- request: `CreateAppRequest` with `name`, `description`, `runtimeRole`, `repositoryUrl`, `defaultBranch`, `tags`
- response: single `AppDetail`
- client_impact: powers `/apps/new`
- compatibility: `runtimeRole` only accepts `Frontend` and `Backend`

### GET /api/v1/apps/workspaces/{workspaceCode}/options - Active App Options

- purpose: return active app options for task create/detail pages
- auth: current admin shell session; no feature-specific permission branch added in this slice
- acceptance_ids: AC-APP-04, AC-APP-06
- parameter_passing: path variables and query parameters
- request: `workspaceCode` path plus optional `search`, `runtimeRole`
- response: list of lightweight option DTOs with `appCode`, `name`, `runtimeRole`
- client_impact: replaces free-text task app entry
- compatibility: archived apps are excluded even if they still exist on tasks

### GET /api/v1/apps/{appCode} - App Detail

- purpose: fetch one app by business code
- auth: current admin shell session; no feature-specific permission branch added in this slice
- acceptance_ids: AC-APP-03, AC-APP-05
- parameter_passing: path variables
- request: `appCode` path
- response: single `AppDetail`
- client_impact: powers `/apps/detail`
- compatibility: archived apps remain readable

### PATCH /api/v1/apps/{appCode} - Update App

- purpose: update mutable app metadata
- auth: current admin shell session; no feature-specific permission branch added in this slice
- acceptance_ids: AC-APP-03, AC-APP-05
- parameter_passing: path variables and request body
- request: `UpdateAppRequest` with mutable fields only
- response: single `AppDetail`
- client_impact: powers `/apps/edit`
- compatibility: workspace ownership and `appCode` are immutable

### POST /api/v1/apps/{appCode}/archive - Archive App

- purpose: move an active app to archived status
- auth: current admin shell session; no feature-specific permission branch added in this slice
- acceptance_ids: AC-APP-04, AC-APP-05
- parameter_passing: path variables
- request: `appCode` path only
- response: single `AppDetail`
- client_impact: removes the app from default list and task options after success
- compatibility: archived rows stay restorable

### POST /api/v1/apps/{appCode}/restore - Restore App

- purpose: move an archived app back to active status
- auth: current admin shell session; no feature-specific permission branch added in this slice
- acceptance_ids: AC-APP-04, AC-APP-05
- parameter_passing: path variables
- request: `appCode` path only
- response: single `AppDetail`
- client_impact: returns the app to active list and task options
- compatibility: restores the original record and business code

### POST /api/v1/tasks - Task Create With App Validation

- purpose: preserve task create shape while validating `appCodes[]` against the active same-workspace registry
- auth: current admin shell session; no feature-specific permission branch added in this slice
- acceptance_ids: AC-APP-06
- parameter_passing: request body
- request: existing task create DTO plus optional `appCodes[]`
- response: existing task detail envelope
- client_impact: task create page now sends selected app codes from app options
- compatibility: request shape is unchanged; only validation becomes stricter

### PATCH /api/v1/tasks/{taskCode} - Task Update With App Validation

- purpose: preserve task update shape while validating changed `appCodes[]` against the active same-workspace registry
- auth: current admin shell session; no feature-specific permission branch added in this slice
- acceptance_ids: AC-APP-06
- parameter_passing: path variables and request body
- request: existing task patch DTO plus optional `appCodes[]`
- response: existing task detail envelope
- client_impact: task detail omits `appCodes` on unrelated edits to preserve legacy selections
- compatibility: legacy codes remain visible on read and are only rejected once the operator explicitly changes selection

## Auth And Permissions

- This slice stays under the existing authenticated admin shell.
- No new role matrix or per-workspace permission policy is introduced here.
- Workspace scoping is enforced by request path plus backend lookup rules, not by trusting client-side workspace state alone.

## Error Contract

Format: `condition -> status/code; response; AC id`

- missing workspace path -> 400 / `APP_WORKSPACE_REQUIRED`; uniform error response; AC-APP-05
- unknown workspace -> 404 / `APP_WORKSPACE_INVALID`; uniform error response; AC-APP-05
- missing app name -> 400 / `APP_NAME_REQUIRED`; uniform error response; AC-APP-05
- duplicate app name in same workspace -> 409 / `APP_NAME_DUPLICATE`; uniform error response; AC-APP-05
- unsupported runtime role -> 400 / `APP_RUNTIME_ROLE_INVALID`; uniform error response; AC-APP-05
- unknown app code -> 404 / `APP_NOT_FOUND`; uniform error response; AC-APP-05
- invalid lifecycle transition or archived edit attempt -> 409 / `APP_ARCHIVED_INVALID`; uniform error response; AC-APP-04
- task `appCodes[]` contains unknown, archived, or foreign-workspace app -> 400 / `TASK_APP_INVALID`; uniform error response; AC-APP-06

## Frontend Backend Boundary

- Frontend responsibilities:
  - read current workspace from shell state
  - query workspace-scoped app list and app options
  - present `/apps`, `/apps/new`, `/apps/detail`, `/apps/edit`
  - keep legacy task app codes visible without resubmitting them on unrelated edits
- Backend responsibilities:
  - own app persistence and business-code generation
  - enforce workspace scoping and duplicate-name protection
  - enforce archive/restore lifecycle
  - validate task app selections against active same-workspace apps
- Shared boundary:
  - `AppDetail`, `AppSummary`, and task app option DTOs are the only frontend-consumed registry shapes
  - import/sync/source/path/worktree fields are removed from the shared boundary in this version

## Database And Transaction Boundary

- App create, update, archive, and restore each run as single app-repository transactions.
- Tag replacement for create/update is owned by the app repository boundary.
- Task create/update keeps its existing task transaction boundary and calls `AppService` only for pre-persistence validation.
- `task_app_links` persistence is unchanged; only the validation gate changes.

## Mock Or Adapter Expectations

- Frontend app mocks must expose the same public fields as the backend DTOs.
- Task mocks must derive selectable apps from active workspace app fixtures instead of accepting arbitrary free-text values.
- Playwright/browser tests must satisfy `GET /api/v1/admin/workspaces/switcher` before shell-mounted `/apps` and `/tasks/*` pages remain mounted.
