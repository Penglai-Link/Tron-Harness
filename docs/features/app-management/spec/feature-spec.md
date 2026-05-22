# Feature Spec: App Management

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved

App management is implemented as a workspace-scoped registry under `/apps`, backed by the shell workspace selector and real backend persistence. This slice removes import and sync affordances, limits `runtimeRole` to `Frontend | Backend`, and migrates task app selection from free-text codes to active app options in the current workspace while preserving legacy codes until the selection is explicitly changed.

## Feature Identity And Context

- feature_slug: app-management
- feature_name: App Management
- status: implemented
- owner: clouds3n
- source_request: `使用 [$eliteforge-codex-superpower] 完成app-management feature` plus follow-up constraints for `runtimeRole` and explicit create/detail/update pages
- main_agent: current-thread
- dependency_features: task-management, workspace-management
- dependency_contracts: shell workspace switcher, uniform API response envelope, task link persistence

## Requirement

- user_story: As an operator working in one selected workspace, I need a real app registry so I can create, inspect, update, archive, and assign apps to tasks without relying on import placeholders or free-text codes.
- intent: Replace placeholder app UI and mock-only concepts with a stable workspace-scoped app registry that the task feature can trust.
- outcome: `/apps` becomes a complete CRUD + lifecycle feature and task pages use active apps from the selected workspace.

## Scope Boundary

### Assumptions

- The selected workspace always comes from `TronShell` and remains the tenancy boundary for app management.
- Import, sync, local filesystem paths, worktree paths, deployment health, and job/runtime telemetry stay out of scope in this version.
- Existing `task_app_links` remain the persistence join table for task-to-app assignment.

### In Scope

- Real backend app persistence, API routes, and validation.
- Frontend app list, create page, detail page, edit page, archive, and restore flows.
- Task create/detail migration from free-text `appCodes` to active workspace app options.
- Legacy task app codes remaining visible until the user explicitly changes app selection.

### Out Of Scope

- GitHub import, local import, sync, and related buttons or routes.
- Additional runtime roles beyond `Frontend` and `Backend`.
- Hard delete for apps.
- Cross-workspace app browsing from the app page itself.

### Constraints

- API response envelopes must follow project `AGENTS.md`.
- `runtimeRole` is only `Frontend | Backend`.
- Archived apps cannot be edited or assigned to tasks.
- Task app validation must reject unknown, archived, or foreign-workspace app codes.

## Feature Model

`App` is a workspace-owned aggregate identified by `appCode` and containing name, description, runtime role, optional repository metadata, tags, lifecycle status, and audit timestamps. `AppStatus` transitions between `Active` and `Archived`; restore returns the same app to `Active` without issuing a new code. `AppTag` remains feature-local metadata linked by `appCode`.

Task assignment depends on the active app registry. Task writes accept only app codes whose app records are active and belong to the same workspace as the task. Legacy task data may still contain historic codes that no longer map to registry rows; those codes remain visible in task detail until the app selection changes, at which point the user must remove or replace them.

## Page And Module Surface

- UI routes:
  - `/apps`
  - `/apps/new`
  - `/apps/detail?appCode=...`
  - `/apps/edit?appCode=...`
- Updated task routes:
  - `/tasks/new`
  - `/tasks/detail?taskCode=...`
- Frontend modules:
  - `tron-frontend/src/domain/app/*`
  - `tron-frontend/src/domain/task/hooks/useTaskWorkspaceOptions.ts`
  - `tron-frontend/src/domain/task/components/TaskAppSelectionField.tsx`
  - `tron-frontend/src/domain/task/pages/CreateTaskPage.tsx`
  - `tron-frontend/src/domain/task/pages/TaskDetailsPage.tsx`
- Backend modules:
  - `tron-backend/src/domain/app.rs`
  - `tron-backend/src/application/app.rs`
  - `tron-backend/src/adapters/inbound/http/apps/*`
  - `tron-backend/src/adapters/outbound/persistence/sqlite/app_repository.rs`
  - `tron-backend/src/application/task.rs`
  - `tron-backend/migrations/20260519_0011_create_app_management_apps.sql`

## Repository Findings

- `tron-frontend/src/domain/app/pages/AppsPage.tsx` was a placeholder table over `source/path/worktree` fields and exposed a dead import action.
- `tron-frontend/src/router/routes.tsx` and `pageRegistry.ts` still registered `apps.import.github` and `apps.import.local`.
- `tron-frontend/src/domain/task/pages/CreateTaskPage.tsx` and `TaskDetailsPage.tsx` previously used plain text inputs for `appCodes`.
- `tron-backend` had no app domain or HTTP routes before this slice; task validation only checked assignees, parent/child scope, and status rules.
- Shell-mounted frontend pages depend on `GET /api/v1/admin/workspaces/switcher`; this affects browser verification for `/apps` and task pages.
- Strong feature dependencies: task-management and workspace-management only; no separate feature-spec dependency under `docs/features/` existed yet for apps.

## Open Decisions

- Common capability extraction: no candidate found. App selection logic is currently bounded to task pages and app registry pages; it is not reused widely enough yet to justify a broader shared workspace-form library.
