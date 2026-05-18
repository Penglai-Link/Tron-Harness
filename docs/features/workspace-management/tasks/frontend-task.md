# Frontend Task: Workspace Management

## Execution Contract

- feature_slug: workspace-management
- status: complete
- execution_mode: solo

## TDD Contract

Default to one bounded `tdd` row per AC slice. A `tdd` row owns RED -> GREEN -> focused VERIFY for the AC inside one write scope.

## Task Rows

- AC-WS-01 | phase: tdd | owner: lead | scope: tron-frontend/src/router/* tron-frontend/src/domain/admin/pages/WorkspaceManagementPage.tsx tron-frontend/src/domain/admin/api/adminApi.ts | command_ref: CMD-FE-CHECK | status: done.
- AC-WS-03 | phase: tdd | owner: lead | scope: tron-frontend/src/domain/admin/pages/WorkspaceManagementPage.tsx tron-frontend/src/domain/admin/api/adminApi.ts tron-frontend/tests/workspace-management-smoke.spec.ts | command_ref: CMD-WS-BROWSER | status: done.
- AC-WS-04 | phase: tdd | owner: lead | scope: tron-frontend/src/domain/admin/pages/WorkspaceManagementPage.tsx tron-frontend/src/domain/admin/api/adminApi.ts tron-frontend/tests/workspace-management-smoke.spec.ts | command_ref: CMD-WS-BROWSER | status: done.
- AC-WS-07 | phase: tdd | owner: lead | scope: tron-frontend/src/domain/admin/pages/WorkspaceManagementPage.tsx tron-frontend/src/shared/layout/Shell.tsx tron-frontend/tests/workspace-management-smoke.spec.ts | command_ref: CMD-WS-BROWSER | status: done.
- AC-WS-09 | phase: tdd | owner: lead | scope: tron-frontend/src/domain/admin/api/adminApi.ts tron-frontend/src/domain/admin/mock/data.ts tron-frontend/src/domain/admin/pages/WorkspaceManagementPage.tsx tron-frontend/tests/workspace-management-smoke.spec.ts | command_ref: CMD-WS-BROWSER | status: done.
- AC-WS-10 | phase: tdd | owner: lead | scope: tron-frontend/src/domain/admin/pages/WorkspaceManagementPage.tsx tron-frontend/tests/workspace-management-smoke.spec.ts | command_ref: CMD-WS-BROWSER | status: done.
- AC-WS-11 | phase: tdd | owner: lead | scope: tron-frontend/src/domain/admin/pages/WorkspaceManagementPage.tsx tron-frontend/tests/workspace-management-smoke.spec.ts | command_ref: CMD-WS-BROWSER | status: done.
- AC-WS-12 | phase: tdd | owner: lead | scope: tron-frontend/src/domain/admin/api/adminApi.ts tron-frontend/src/domain/task/hooks/useTaskWorkspaceOptions.ts tron-frontend/src/domain/task/pages/CreateTaskPage.tsx tron-frontend/src/domain/task/pages/TasksKanbanPage.tsx tron-frontend/src/shared/types/WorkspaceSpace.ts tron-frontend/src/shared/constants/workspaceSpaces.ts tron-frontend/tests/task-management-smoke.spec.ts | command_ref: CMD-TASK-BROWSER | status: done.
- AC-WS-13 | phase: tdd | owner: lead | scope: tron-frontend/src/domain/admin/api/adminApi.ts tron-frontend/src/shared/layout/Shell.tsx tron-frontend/src/shared/hooks/useWorkspaceSpaces.ts tron-frontend/src/shared/constants/workspaceSpaces.ts tron-frontend/src/shared/types/WorkspaceSpace.ts tron-frontend/src/domain/dashboard/pages/DashboardPage.tsx tron-frontend/tests/workspace-management-smoke.spec.ts | command_ref: CMD-WS-BROWSER | status: done.

## Worktree Notes

- Lead executes frontend work directly in `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/workspace-management`.

## Integration Notes

- Reuse shared toast, buttons, data-table, and shell primitives.
- `/admin/workspace` is the canonical write surface for workspace-access grants introduced here.

## Open Decisions

- None.
