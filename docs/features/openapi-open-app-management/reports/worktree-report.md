# Worktree Report: OpenAPI Open-App Management

## Summary

This feature completed entirely inside the selected Lite Spec base worktree `feature/clouds3n/openapi-open-app-management/base`, including backend lifecycle APIs, frontend settings route, regression coverage, and real browser proof artifacts.

## Evidence Rows

- AC-OAM-01 | red: the selected base worktree started with only the HMAC auth foundation and no management lifecycle surface. | green: the current worktree contains admin routes, application service methods, repository reads, and passing lifecycle tests for create, list, detail, update, and rotate. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAM-02 | red: the selected base worktree started with inline runtime route mapping only. | green: the current worktree contains one shared capability catalog consumed by both admin APIs and runtime auth, with passing regression coverage. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAM-03 | red: the selected base worktree started with append-only grant helpers and no explicit replacement flow. | green: the current worktree contains replacement commands, validation, repository transactions, and passing invalid-target coverage. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAM-04 | red: the selected base worktree started with no `/settings/openapi` route or operator UI. | green: the current worktree contains route wiring, settings navigation, page implementation, mock parity, and passing real-browser flow coverage. | command_ref: CMD-BROWSER-OPENAPI | browser: clicks=signed in, created the open-app, saved grants, and observed refresh in the same worktree runtime; inputs=`admin`, `TronAdminPass123!`, `Playwright Open App`, `USR-ADMIN`; network=frontend build plus real admin API calls succeeded; feedback=page rendered in settings shell and browser artifacts were written under the feature reports directory; artifacts=`docs/features/openapi-open-app-management/reports/browser-openapi-shell.png`, `docs/features/openapi-open-app-management/reports/browser-openapi-network.json` | status: done.
- AC-OAM-05 | red: the selected base worktree had no browser regression proof for wording and rotate-only lifecycle. | green: the current worktree contains a dedicated Playwright smoke that proves `open-app` wording, grouped capability display, dismiss behavior, and rotate-only re-reveal. | command_ref: CMD-BROWSER-OPENAPI | browser: clicks=`Dismiss`, `Rotate key`, confirm `Rotate key`; inputs=none after setup; network=rotate admin endpoint 200; feedback=reveal panel hid and reappeared only on rotate; artifacts=`docs/features/openapi-open-app-management/reports/browser-openapi-shell.png`, `docs/features/openapi-open-app-management/reports/browser-openapi-network.json` | status: done.

## Changed Paths

- `docs/features/openapi-open-app-management/`
- `tron-backend/migrations/20260611_0001_create_platform_open_apps.sql`
- `tron-backend/src/adapters/inbound/http/admin/open_apps/`
- `tron-backend/src/adapters/inbound/open_api/auth.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/open_app_repository.rs`
- `tron-backend/src/application/open_app/`
- `tron-backend/src/domain/open_app/`
- `tron-frontend/src/domain/open-app/`
- `tron-frontend/tests/openapi-management-smoke.spec.ts`

## Commands

- `rtk git status --short`
- `rtk make be-check`
- `rtk make be-test`
- `rtk make fe-check`
- `rtk make fe-test TEST=tests/openapi-management-smoke.spec.ts`

## Blockers

None.
