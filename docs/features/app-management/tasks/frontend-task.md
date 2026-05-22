# Frontend Task: App Management

## Execution Contract

- feature_slug: app-management
- status: complete
- execution_mode: solo

## TDD Contract

Frontend execution replaced the placeholder app area, removed import flows, and migrated task app editing in one integrated UI slice.

## Task Rows

- AC-APP-01 | phase: tdd | owner: lead | scope: `tron-frontend/src/domain/app/pages/AppsPage.tsx`, app list filters, pagination, and workspace scoping | command_ref: CMD-FE-CHECK | status: done.
- AC-APP-02 | phase: tdd | owner: lead | scope: `tron-frontend/src/domain/app/pages/CreateAppPage.tsx`, `tron-frontend/src/domain/app/components/AppFormFields.tsx` | command_ref: CMD-FE-CHECK | status: done.
- AC-APP-03 | phase: tdd | owner: lead | scope: `tron-frontend/src/domain/app/pages/AppDetailsPage.tsx`, `tron-frontend/src/domain/app/pages/EditAppPage.tsx` | command_ref: CMD-FE-CHECK | status: done.
- AC-APP-04 | phase: tdd | owner: lead | scope: archive/restore actions in app list and detail surfaces | command_ref: CMD-FE-CHECK | status: done.
- AC-APP-06 | phase: tdd | owner: lead | scope: `tron-frontend/src/domain/task/hooks/useTaskWorkspaceOptions.ts`, `tron-frontend/src/domain/task/components/TaskAppSelectionField.tsx`, task create/detail pages | command_ref: CMD-FE-CHECK | status: done.

## Worktree Notes

- Implemented in the base worktree because route, app model, and task selector changes shared the same frontend write scope.

## Integration Notes

- Removed `apps.import.github` and `apps.import.local` routes and related import-default concepts.
- Added explicit create/detail/edit pages because the user later required missing app pages to be filled in.

## Open Decisions

- None.
