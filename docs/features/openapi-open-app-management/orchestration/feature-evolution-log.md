# Feature Evolution Log: OpenAPI Open-App Management

## Summary

- feature_slug: openapi-open-app-management
- status: done

## Change History

- 2026-06-11: implemented platform admin lifecycle APIs, shared capability catalog reuse, grant replacement flows, and `/settings/openapi` UI on `feature/clouds3n/openapi-open-app-management/base`.
- 2026-06-11: added real Playwright smoke plus screenshot and network artifacts for create, explicit grants, dismiss, and rotate lifecycle proof.

## Downstream Artifact Migration

- `OpenAppService` expands from runtime auth and basic mutations to management lifecycle queries and grant replacement.
- Public open-api middleware route mapping should move behind a shared registry so operator catalog output and runtime auth cannot drift.
- Frontend settings shell gains one new platform page and navigation entry.

## Residual Risks

- 2026-06-11: migrated the bundled `@tron/chat-openapi-adapter` to `TRON-HMAC-SHA256-V1` signing and `X-Tron-User-Code`, removing its stale bearer and `X-Tron-User-Id` behavior.
