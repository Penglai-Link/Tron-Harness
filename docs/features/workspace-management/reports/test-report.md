# Test Report: Workspace Management

## Summary

- The workspace-management proof plan is complete. Backend compile and test gates passed, frontend typecheck and build passed, and the dedicated Playwright smoke proved the hierarchy route, tree mutations, segmented access-grant editors, effective-user evidence, mode switches, and archive or restore lifecycle end to end.

## Evidence Rows

- AC-WS-01 | red: `master` had no `/admin/workspace` route or page entry. | green: `rtk make fe-check` passed and Playwright proved the page loaded, root and child creation worked, and detail state changed after selection without document reload. | command_ref: CMD-WS-BROWSER | browser: clicks=open `/admin/workspace`, select `PLATFORM`, create root `RISK`, create child `RISK_REVIEW`, select `RISK`; inputs=`workspaceCode`, `name`, and `description` for root and child creation; network=`GET /api/v1/admin/workspaces`, `POST /api/v1/admin/workspaces`, and `GET /api/v1/admin/workspaces/{workspaceCode}` via detail refresh; feedback=tree rows and detail inputs updated in place with success toasts; artifacts=`tron-frontend/tests/workspace-management-smoke.spec.ts`, `tron-frontend/src/domain/admin/pages/WorkspaceManagementPage.tsx` | status: done.
- AC-WS-02 | red: `master` had no workspace hierarchy contract tests or enforcement. | green: `tron-backend/tests/workspace_management_api.rs` proves level-four create and cycle move return `ADMIN_WORKSPACE_HIERARCHY_INVALID`, and `rtk make be-test` passed. | command_ref: CMD-BE-TEST | browser: clicks=backend test helper invokes invalid create and move mutations through the admin router; inputs=`LEVEL_FOUR` create payload and `ENTERPRISE -> TRON_CORE` move payload; network=`POST /api/v1/admin/workspaces` and `POST /api/v1/admin/workspaces/ENTERPRISE/move`; feedback=asserted `400` envelopes with stable hierarchy error codes; artifacts=`tron-backend/tests/workspace_management_api.rs`, `tron-backend/src/application/workspace.rs`, `tron-backend/src/domain/workspace.rs` | status: done.
- AC-WS-03 | red: `master` had no direct-user access management surface. | green: Playwright proved direct-user grants can be replaced, owner state can be toggled, and the saved rows refresh with toast feedback from the `Access` mode. | command_ref: CMD-WS-BROWSER | browser: clicks=select `PLATFORM`, switch to `Access`, add `USR-AUTH-0001`, toggle `Member -> Owner`, save direct-user access; inputs=direct-user combobox selection for `Auth Operator`; network=`PUT /api/v1/admin/workspaces/PLATFORM/access/users`; feedback=toast `Direct user access updated` and refreshed table rows; artifacts=`tron-frontend/tests/workspace-management-smoke.spec.ts`, `tron-frontend/src/domain/admin/api/adminApi.ts` | status: done.
- AC-WS-04 | red: `master` had no role-based access management surface. | green: Playwright proved role grants can be replaced and matched-user rows refresh with toast feedback from the `Access` mode. | command_ref: CMD-WS-BROWSER | browser: clicks=select `PLATFORM`, switch to `Access`, add role `VIEWER`, save role access; inputs=role combobox selection `VIEWER`; network=`PUT /api/v1/admin/workspaces/PLATFORM/access/roles`; feedback=toast `Role access updated` and refreshed role-access rows; artifacts=`tron-frontend/tests/workspace-management-smoke.spec.ts`, `tron-frontend/src/domain/admin/api/adminApi.ts` | status: done.
- AC-WS-05 | red: `master` had no effective-user union read for direct and role grants. | green: backend tests proved deduplicated effective users expose `DirectAndRole`, owner flags, and matched role codes; the browser smoke showed `Auth Operator`, `Chen Wei`, `DirectAndRole`, and `Viewer` in the effective-user table reached through the `Access` mode. | command_ref: CMD-BE-TEST | browser: clicks=switch to `Access` and review the effective-user table after direct-user and role saves; inputs=reused direct-user and role updates from AC-WS-03 and AC-WS-04; network=`GET /api/v1/admin/workspaces/PLATFORM/access/effective-users`; feedback=effective-user table renders merged access source labels and matched roles; artifacts=`tron-backend/tests/workspace_management_api.rs`, `tron-frontend/tests/workspace-management-smoke.spec.ts` | status: done.
- AC-WS-06 | red: `master` had no workspace routes, no schema, and no persistence proof. | green: focused API coverage plus `rtk make be-test` proved hierarchy, grant, effective-user, statistics, and archive or restore endpoints persist through SQLite behind the uniform admin envelope. | command_ref: CMD-BE-TEST | browser: clicks=backend test helper reads the workspace tree and statistics, replaces grants, and exercises lifecycle routes on a temp SQLite database; inputs=authenticated requests plus create, grant, and archive payloads; network=workspace hierarchy, access, effective-user, statistics, and lifecycle calls under `/api/v1/admin/workspaces/**`; feedback=asserted `SUCCESS` envelopes, persisted counts, and stable access records after writes; artifacts=`tron-backend/tests/workspace_management_api.rs`, `tron-backend/migrations/20260516_0006_create_workspace_management_workspaces.sql`, `tron-backend/migrations/20260516_0007_create_workspace_management_access_grants.sql` | status: done.
- AC-WS-07 | red: `master` had no workspace page, so shell persistence and in-place mode changes were absent. | green: Playwright proved `/admin/workspace` stayed on the same document URL while the mounted shell remained visible and the workspace studio switched modes without a reload. | command_ref: CMD-WS-BROWSER | browser: clicks=open `/admin/workspace`, select `PLATFORM`, switch to `Access`, move `RISK` under `COMPLIANCE`; inputs=move-parent selection `COMPLIANCE`; network=`GET /api/v1/admin/workspaces`, `GET /api/v1/admin/workspaces/{workspaceCode}`, plus `POST /api/v1/admin/workspaces/RISK/move`; feedback=URL stayed unchanged, access rows opened in place, and move toast `Workspace RISK moved` appeared without a document reload; artifacts=`tron-frontend/tests/workspace-management-smoke.spec.ts`, `tron-frontend/src/router/routes.tsx`, `tron-frontend/src/router/pageRegistry.ts` | status: done.
- AC-WS-08 | red: `master` had no subtree archive or restore lifecycle. | green: backend tests proved subtree archive and active-ancestor restore rules, and browser smoke proved archive then restore feedback on `RISK`; `rtk make be-test` and Playwright both passed. | command_ref: CMD-BE-TEST | browser: clicks=select `RISK`, archive workspace, restore workspace; inputs=none; network=`POST /api/v1/admin/workspaces/RISK/archive` and `POST /api/v1/admin/workspaces/RISK/restore`; feedback=toasts `RISK archived` and `RISK restored`, and action buttons flipped between Archive and Restore; artifacts=`tron-backend/tests/workspace_management_api.rs`, `tron-frontend/tests/workspace-management-smoke.spec.ts` | status: done.

## Commands

- `rtk make be-check` passed on 2026-05-16.
- `rtk make fe-check` passed on 2026-05-16.
- `rtk make be-test` passed on 2026-05-16.
- `rtk npm --prefix tron-frontend exec playwright -- test workspace-management-smoke --reporter=line` passed `1/1` on 2026-05-16.

## Changed Paths

- `docs/features/workspace-management/*`
- `tron-backend/migrations/20260516_0006_create_workspace_management_workspaces.sql`
- `tron-backend/migrations/20260516_0007_create_workspace_management_access_grants.sql`
- `tron-backend/src/adapters/inbound/http/admin/workspaces/*`
- `tron-backend/src/adapters/outbound/persistence/sqlite/workspace_repository.rs`
- `tron-backend/src/application/workspace.rs`
- `tron-backend/src/domain/workspace.rs`
- `tron-backend/tests/workspace_management_api.rs`
- `tron-frontend/src/domain/admin/api/adminApi.ts`
- `tron-frontend/src/domain/admin/model/AdminModel.ts`
- `tron-frontend/src/domain/admin/pages/WorkspaceManagementPage.tsx`
- `tron-frontend/src/router/pageRegistry.ts`
- `tron-frontend/src/router/routes.tsx`
- `tron-frontend/tests/workspace-management-smoke.spec.ts`

## Blockers

- None.
