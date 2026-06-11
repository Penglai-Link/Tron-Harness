# Frontend Result: OpenAPI Open-App Management

## Summary

The settings shell now exposes `/settings/openapi` with platform-level open-app lifecycle, explicit user grants, grouped capability grants, and rotate-only key lifecycle UX backed by real browser proof.

## Evidence Rows

- AC-OAM-04 | red: `/settings/openapi` did not exist, and the shell had no open-app operator surface. | green: `rtk make fe-check` passed route, page, and API client wiring, and `rtk make fe-test TEST=tests/openapi-management-smoke.spec.ts` proved the page loads in the settings shell, creates an open-app, shows the one-time reveal, saves explicit user grants, and saves capability grants. | command_ref: CMD-BROWSER-OPENAPI | browser: clicks=opened `/settings/openapi`, signed in as bootstrap admin, clicked `New open-app`, clicked `Create open-app`, clicked the `Read Sessions` capability label, clicked `Save user grants`, clicked `Save capability grants`; inputs=`admin`, `TronAdminPass123!`, open-app name `Playwright Open App`, description `Browser-driven lifecycle verification`, user grant `USR-ADMIN`; network=`POST /api/v1/auth/login` 200, `GET /api/v1/admin/platform/open-apps` 200, `GET /api/v1/admin/platform/open-apps/catalog` 200, `POST /api/v1/admin/platform/open-apps` 201, `PUT /api/v1/admin/platform/open-apps/{openAppCode}/user-grants` 200, `PUT /api/v1/admin/platform/open-apps/{openAppCode}/capability-grants` 200; feedback=Platform OpenAPI heading rendered inside settings shell, one-time reveal panel rendered after create, `User grants updated` and `Capability grants updated` toasts rendered, refreshed detail kept fingerprint-only metadata fields; artifacts=`docs/features/openapi-open-app-management/reports/browser-openapi-shell.png`, `docs/features/openapi-open-app-management/reports/browser-openapi-network.json` | status: done.
- AC-OAM-05 | red: the page could still leak app-management naming, capability grouping could drift from backend, or secret lifecycle could behave like a reusable raw-key detail screen. | green: `rtk make fe-test TEST=tests/openapi-management-smoke.spec.ts` proved the page copy uses `open-app`, capability groups render from backend data, dismiss hides the reveal panel, and rotate is the only path that reveals a fresh key again. | command_ref: CMD-BROWSER-OPENAPI | browser: clicks=clicked `Dismiss`, clicked `Rotate key`, confirmed `Rotate key`; inputs=none after create/grant save steps; network=`POST /api/v1/admin/platform/open-apps/{openAppCode}/rotate-key` 200 plus follow-up list/catalog refreshes 200; feedback=reveal panel disappeared after dismiss, `Copy key` action disappeared with the panel, `Current Fingerprint` stayed visible in detail, and a fresh reveal panel returned only after rotate; artifacts=`docs/features/openapi-open-app-management/reports/browser-openapi-shell.png`, `docs/features/openapi-open-app-management/reports/browser-openapi-network.json` | status: done.

## Changed Paths

- `tron-frontend/packages/chat-core/src/openapi-signing.ts`
- `tron-frontend/packages/chat-openapi-adapter/src/index.ts`
- `tron-frontend/src/domain/open-app/`
- `tron-frontend/src/domain/settings/constants/settingsNavigation.ts`
- `tron-frontend/src/router/routes.tsx`
- `tron-frontend/src/router/pageRegistry.ts`
- `tron-frontend/src/i18n/locales/en-US/shell.ts`
- `tron-frontend/src/i18n/locales/zh-CN/shell.ts`
- `tron-frontend/tests/openapi-management-smoke.spec.ts`
- `docs/features/openapi-open-app-management/reports/browser-openapi-shell.png`
- `docs/features/openapi-open-app-management/reports/browser-openapi-network.json`

## Commands

- `rtk make fe-check`
- `rtk make fe-test TEST=tests/openapi-management-smoke.spec.ts`
- `rtk npm --prefix tron-frontend exec vitest run packages/chat-core/src/openapi-signing.test.ts packages/chat-openapi-adapter/src/index.test.ts`

## Blockers

None.
