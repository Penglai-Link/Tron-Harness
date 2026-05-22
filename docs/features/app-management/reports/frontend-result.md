# Frontend Report: App Management

## Summary

Frontend now has real app list, create, detail, edit, archive, and restore flows, and task app selection is registry-backed. Highest-risk delta was replacing free-text task app editing without breaking legacy task visibility.

## Evidence Rows

- AC-APP-01 | red: `/apps` showed placeholder source and path fields plus import actions | green: app smoke proves workspace list filters, pagination state, and no import button | command_ref: CMD-PW-APP | browser: clicks=`/apps`, runtime-role filter, status filter, pagination controls; inputs=search text; network=`GET /api/v1/apps/workspaces/TRON_CORE*`; feedback=workspace-scoped rows only and no import action; artifacts=Playwright report | status: done.
- AC-APP-02 | red: no `/apps/new` route existed | green: create flow lands on detail after save | command_ref: CMD-PW-APP | browser: clicks=`Create App`, `Create App`; inputs=name, runtime role, repository URL, default branch, tags, description; network=`POST /api/v1/apps/workspaces/TRON_CORE`; feedback=detail page visible after save; artifacts=Playwright report | status: done.
- AC-APP-03 | red: no explicit detail or edit pages existed | green: edit flow updates metadata and returns to detail | command_ref: CMD-PW-APP | browser: clicks=detail row, `Edit`, `Save`; inputs=updated name, runtime role, branch, tags, description; network=`PATCH /api/v1/apps/APP-PW-0001`; feedback=detail reflects updated values; artifacts=Playwright report | status: done.
- AC-APP-04 | red: frontend had no archive or restore lifecycle | green: smoke proves archive, archived filter view, detail restore, and return to active list | command_ref: CMD-PW-APP | browser: clicks=`Archive`, archived row reopen, `Restore`; inputs=status filter; network=`POST /api/v1/apps/{appCode}/archive`, `POST /api/v1/apps/{appCode}/restore`; feedback=archived badge then active row; artifacts=Playwright report | status: done.
- AC-APP-06 | red: task create and detail used free-text `App Codes` inputs | green: task smoke proves active app button selection, legacy app chips, and no unrelated resubmission of legacy codes | command_ref: CMD-PW-TASK | browser: clicks=task app buttons, save action, legacy remove action; inputs=task fields and app selection; network=`GET /api/v1/apps/workspaces/TRON_CORE/options`, task create and patch bodies; feedback=toasts plus preserved legacy chips; artifacts=Playwright report | status: done.

## Changed Paths

- `tron-frontend/src/domain/app/*`
- `tron-frontend/src/domain/task/hooks/useTaskWorkspaceOptions.ts`
- `tron-frontend/src/domain/task/components/TaskAppSelectionField.tsx`
- `tron-frontend/src/domain/task/pages/CreateTaskPage.tsx`
- `tron-frontend/src/domain/task/pages/TaskDetailsPage.tsx`
- `tron-frontend/src/router/routes.tsx`
- `tron-frontend/src/router/pageRegistry.ts`

## Commands

- @ref:spec/test-plan.md#command-index: `CMD-FE-CHECK`, `CMD-PW-APP`, and `CMD-PW-TASK` passed.

## Blockers

- None.
