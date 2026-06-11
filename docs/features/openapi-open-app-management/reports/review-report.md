# Review Report: OpenAPI Open-App Management

## Summary

Review confirmed the management slice keeps the one-time secret boundary, shared capability catalog source of truth, explicit grant semantics, and platform-only `open-app` wording.

## Evidence Rows

- AC-OAM-01 | red: review started from a branch with no platform admin open-app management endpoints. | green: review confirmed create and rotate are the only responses that surface cleartext secrets, while list and detail remain fingerprint-only under passing backend regression coverage. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAM-02 | red: review found the old auth middleware kept route-grant mapping inline. | green: review confirmed the admin catalog endpoint and runtime auth middleware now reuse one application-level capability registry. | command_ref: CMD-BE-CHECK | browser:not applicable | status: done.
- AC-OAM-03 | red: review found the prior service only supported incremental grant insertion and had no replacement boundary. | green: review confirmed grant replacement validates active users and catalog-defined operations before repository mutation and persists each dimension transactionally. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAM-04 | red: review found no settings-shell open-app operator surface before this slice. | green: review reused the passing frontend build plus real browser smoke to confirm the page renders inside Settings, shows current grant state, and exposes the one-time reveal lifecycle clearly. | command_ref: CMD-BROWSER-OPENAPI | browser: clicks=created one open-app, saved user and capability grants, and refreshed the detail flow inside the settings shell; inputs=`admin`, `TronAdminPass123!`, `Playwright Open App`, `USR-ADMIN`; network=create plus grant-save admin endpoints returned success; feedback=Platform OpenAPI heading, reveal panel, and success toasts were visible; artifacts=`docs/features/openapi-open-app-management/reports/browser-openapi-shell.png`, `docs/features/openapi-open-app-management/reports/browser-openapi-network.json` | status: done.
- AC-OAM-05 | red: review required browser proof that app-management naming and legacy reusable raw-key assumptions did not leak back in. | green: review confirmed the visible page copy stays `open-app`, the grouped business capability surface stays visible, dismiss removes the current reveal, and rotate is the only path that returns a new cleartext key. | command_ref: CMD-BROWSER-OPENAPI | browser: clicks=`Dismiss`, `Rotate key`, confirm `Rotate key`; inputs=none after setup; network=rotate endpoint returned 200 with follow-up refreshes; feedback=reveal panel disappeared after dismiss and reappeared only after rotate; artifacts=`docs/features/openapi-open-app-management/reports/browser-openapi-shell.png`, `docs/features/openapi-open-app-management/reports/browser-openapi-network.json` | status: done.

## Changed Paths

- `tron-backend/src/adapters/inbound/http/admin/open_apps/error_mapper.rs`
- `tron-backend/src/application/open_app/catalog.rs`
- `tron-backend/src/application/open_app/service/`
- `tron-backend/src/domain/open_app/signatures.rs`
- `tron-frontend/packages/chat-core/src/openapi-signing.ts`
- `tron-frontend/packages/chat-openapi-adapter/src/index.ts`
- `tron-frontend/src/domain/open-app/pages/PlatformOpenAppsPage.tsx`
- `tron-frontend/tests/openapi-management-smoke.spec.ts`

## Commands

- `rtk make be-check`
- `rtk make be-test`
- `rtk make fe-check`
- `rtk make fe-test TEST=tests/openapi-management-smoke.spec.ts`
- `rtk npm --prefix tron-frontend exec vitest run packages/chat-core/src/openapi-signing.test.ts packages/chat-openapi-adapter/src/index.test.ts`

## Residual Risks

None after the bundled public adapter was migrated to the HMAC-signed `X-Tron-User-Code` contract.

## Blockers

None.
