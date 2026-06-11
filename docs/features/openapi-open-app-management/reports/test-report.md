# Test Report: OpenAPI Open-App Management

## Summary

Backend regression, frontend build, and real-browser smoke all passed for the platform open-app management slice.

## Evidence Rows

- AC-OAM-01 | red: before implementation there was no platform admin open-app lifecycle management route family. | green: `rtk make be-test` passed lifecycle assertions covering create, list, detail, update, disable or enable, and rotate secret boundaries. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAM-02 | red: before implementation the operator catalog could drift from runtime auth route grants. | green: `rtk make be-test` passed shared-catalog assertions proving one registry powers both admin output and runtime auth. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAM-03 | red: before implementation grant replacement and invalid-target management flow coverage were absent. | green: `rtk make be-test` passed explicit user and capability replacement assertions, including invalid user and invalid capability failures. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAM-04 | red: before implementation `/settings/openapi` had no page, no management flow, and no browser proof. | green: `rtk make fe-check` passed and `rtk make fe-test TEST=tests/openapi-management-smoke.spec.ts` passed `1/1`, proving the page renders, creates an open-app, reveals the key once, and saves user plus capability grants. | command_ref: CMD-BROWSER-OPENAPI | browser: clicks=signed in, created the open-app, saved grants, and refreshed the visible detail; inputs=`admin`, `TronAdminPass123!`, `Playwright Open App`, `Browser-driven lifecycle verification`, `USR-ADMIN`; network=`POST /api/v1/auth/login` 200, `POST /api/v1/admin/platform/open-apps` 201, grant-save PUT routes 200; feedback=reveal panel rendered once, grant-save toasts appeared, and the detail view refreshed with fingerprint-only metadata; artifacts=`docs/features/openapi-open-app-management/reports/browser-openapi-shell.png`, `docs/features/openapi-open-app-management/reports/browser-openapi-network.json` | status: done.
- AC-OAM-05 | red: before implementation there was no regression proof that `open-app` naming and rotate-only key lifecycle stayed intact. | green: `rtk make fe-test TEST=tests/openapi-management-smoke.spec.ts` passed dismiss and rotate assertions, and `rtk make be-test` stayed green across the HMAC-only backend contract. | command_ref: CMD-BROWSER-OPENAPI | browser: clicks=`Dismiss`, `Rotate key`, confirm `Rotate key`; inputs=none; network=`POST /api/v1/admin/platform/open-apps/{openAppCode}/rotate-key` 200; feedback=dismiss removed the reveal panel, fingerprint-only detail remained, and rotate restored a fresh reveal panel without exposing secrets in list/detail; artifacts=`docs/features/openapi-open-app-management/reports/browser-openapi-shell.png`, `docs/features/openapi-open-app-management/reports/browser-openapi-network.json` | status: done.

## Changed Paths

- `tron-frontend/packages/chat-core/src/openapi-signing.ts`
- `tron-frontend/packages/chat-openapi-adapter/src/index.ts`
- `tron-backend/src/adapters/inbound/http/admin/open_apps/`
- `tron-backend/src/adapters/inbound/open_api/auth.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/open_app_repository.rs`
- `tron-backend/src/application/open_app/`
- `tron-backend/src/domain/open_app/`
- `tron-backend/src/shared/crypto.rs`
- `tron-backend/src/shared/random.rs`
- `tron-backend/src/shared/time.rs`
- `tron-backend/tests/open_app_management_api.rs`
- `tron-backend/tests/open_api_chat_api.rs`
- `tron-frontend/src/domain/open-app/`
- `tron-frontend/tests/openapi-management-smoke.spec.ts`
- `docs/features/openapi-open-app-management/reports/browser-openapi-shell.png`
- `docs/features/openapi-open-app-management/reports/browser-openapi-network.json`

## Commands

- `rtk make be-check`
- `rtk make be-test`
- `rtk make fe-check`
- `rtk make fe-test TEST=tests/openapi-management-smoke.spec.ts`
- `rtk npm --prefix tron-frontend exec vitest run packages/chat-core/src/openapi-signing.test.ts packages/chat-openapi-adapter/src/index.test.ts`

## Blockers

None.
