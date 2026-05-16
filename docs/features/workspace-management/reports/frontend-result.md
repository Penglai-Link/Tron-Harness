# Frontend Result: Workspace Management

## Summary

- `/admin/workspace` now resolves to a dedicated workspace management page inside the authenticated shell with hierarchy navigation, a simplified segmented right-side `Workspace Studio`, stacked metadata editing, full-width direct-user and role access rows with right-aligned actions, and effective-user readout.
- `rtk make fe-check` and Playwright smoke passed on 2026-05-16.

## Evidence Rows

- AC-WS-01 | red: `master` reserved the nav destination but had no `/admin/workspace` route, page-registry entry, or workspace-management surface. | green: route wiring plus `WorkspaceManagementPage` now load the tree, simplified overview panel, and create flows; `rtk make fe-check` and browser smoke passed. | command_ref: CMD-WS-BROWSER | browser: clicks=open `/admin/workspace`, select `PLATFORM`, create root `RISK`, create child `RISK_REVIEW`; inputs=create `workspaceCode`, `name`, and `description` values for both new nodes; network=`GET /api/v1/admin/workspaces`, `POST /api/v1/admin/workspaces`, and detail refresh under `/api/v1/admin/workspaces/{workspaceCode}`; feedback=tree rows and overview fields refreshed in place with success toasts; artifacts=`tron-frontend/tests/workspace-management-smoke.spec.ts`, `tron-frontend/src/domain/admin/pages/WorkspaceManagementPage.tsx`, `tron-frontend/src/router/routes.tsx` | status: done.
- AC-WS-03 | red: `master` had no direct-user access editor, owner toggle flow, or save path. | green: the page now replaces direct-user grants, toggles owner state, refreshes visible rows, and shows toast feedback from the segmented `Access` mode; Playwright smoke passed. | command_ref: CMD-WS-BROWSER | browser: clicks=select `PLATFORM`, switch to `Access`, add `Auth Operator`, toggle `Member -> Owner`, save direct-user access; inputs=direct-user combobox selection `USR-AUTH-0001`; network=`PUT /api/v1/admin/workspaces/PLATFORM/access/users`; feedback=toast `Direct user access updated` and refreshed direct-user table rows; artifacts=`tron-frontend/tests/workspace-management-smoke.spec.ts`, `tron-frontend/src/domain/admin/api/adminApi.ts`, `tron-frontend/src/domain/admin/pages/WorkspaceManagementPage.tsx` | status: done.
- AC-WS-04 | red: `master` had no role-based access editor or matched-user refresh. | green: the page now replaces role grants, refreshes matched-user counts, and shows toast feedback from the segmented `Access` mode; Playwright smoke passed. | command_ref: CMD-WS-BROWSER | browser: clicks=select `PLATFORM`, switch to `Access`, add role `VIEWER`, save role access; inputs=role combobox selection `VIEWER`; network=`PUT /api/v1/admin/workspaces/PLATFORM/access/roles`; feedback=toast `Role access updated` and refreshed role-access rows; artifacts=`tron-frontend/tests/workspace-management-smoke.spec.ts`, `tron-frontend/src/domain/admin/api/adminApi.ts`, `tron-frontend/src/domain/admin/pages/WorkspaceManagementPage.tsx` | status: done.
- AC-WS-07 | red: `master` had no workspace page, so shell persistence and in-place mode changes were absent. | green: `/admin/workspace` keeps the authenticated shell mounted and reveals overview, access, and create work through explicit mode switches instead of one overloaded right column; Playwright smoke passed. | command_ref: CMD-WS-BROWSER | browser: clicks=open `/admin/workspace`, select `PLATFORM`, switch to `Access`, move `RISK` under `COMPLIANCE`, archive and restore `RISK`; inputs=move-parent selection `COMPLIANCE`; network=`GET /api/v1/admin/workspaces`, `GET /api/v1/admin/workspaces/{workspaceCode}`, plus move and lifecycle routes; feedback=URL stayed unchanged, shell remained mounted, and success toasts appeared without a document reload while the inspector changed modes in place; artifacts=`tron-frontend/tests/workspace-management-smoke.spec.ts`, `tron-frontend/src/router/routes.tsx`, `tron-frontend/src/router/pageRegistry.ts` | status: done.

## Changed Paths

- `tron-frontend/src/domain/admin/api/adminApi.ts`
- `tron-frontend/src/domain/admin/mock/data.ts`
- `tron-frontend/src/domain/admin/mock/index.ts`
- `tron-frontend/src/domain/admin/model/AdminModel.ts`
- `tron-frontend/src/domain/admin/pages/WorkspaceManagementPage.tsx`
- `tron-frontend/src/router/pageRegistry.ts`
- `tron-frontend/src/router/routes.tsx`
- `tron-frontend/tests/workspace-management-smoke.spec.ts`

## Commands

- `rtk make fe-check` passed on 2026-05-16.
- `rtk npm --prefix tron-frontend exec playwright -- test workspace-management-smoke --reporter=line` passed `1/1` on 2026-05-16.

## Blockers

- None.
