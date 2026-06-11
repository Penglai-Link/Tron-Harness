# Worktree Report: OpenAPI HMAC Auth Foundation

## Summary

The entire HMAC auth foundation was completed inside the selected Lite Spec base worktree `feature/clouds3n/openapi-open-app-management/base`, and the same worktree now also carries the dependent open-app management slice.

## Evidence Rows

- AC-OAH-01 | red: the selected base worktree started with no open-app persistence layer for credentials, grants, or nonce replay protection. | green: the current worktree contains the platform open-app migration, SQLite repository, rotation-safe credential history, and duplicate-nonce rejection coverage. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAH-02 | red: the selected base worktree started with public open-api bearer fallback and no standard HMAC verification path. | green: the current worktree contains HMAC-only middleware and passing bearer-rejection plus invalid-signature coverage on the public chat routes. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAH-03 | red: the selected base worktree started with user lookup only and no explicit open-app grant model. | green: the current worktree contains explicit user-grant and capability-grant authorization that gates open-api auth context creation. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAH-04 | red: the selected base worktree started with no stable business capability mapping for current public agent-chat routes. | green: the current worktree contains shared route-grant mapping and stable uniform denial responses before handler execution. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAH-05 | red: the selected base worktree started with legacy public-route fixtures and preflight expectations. | green: the current worktree contains focused open-api chat regression coverage for signed headers, preflight, stream, replay, and current route wiring. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.

## Changed Paths

- `docs/features/openapi-hmac-auth-foundation/`
- `tron-frontend/packages/chat-core/src/openapi-signing.ts`
- `tron-frontend/packages/chat-openapi-adapter/src/index.ts`
- `tron-backend/migrations/20260611_0001_create_platform_open_apps.sql`
- `tron-backend/src/adapters/inbound/open_api/auth.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/open_app_repository.rs`
- `tron-backend/src/application/open_app/`
- `tron-backend/src/domain/open_app/`
- `tron-backend/src/shared/crypto.rs`
- `tron-backend/src/shared/random.rs`
- `tron-backend/src/shared/time.rs`
- `tron-backend/tests/open_api_chat_api.rs`

## Commands

- `rtk git status --short`
- `rtk make be-check`
- `rtk make be-test`
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test open_api_chat_api -- --nocapture`
- `rtk npm --prefix tron-frontend exec vitest run packages/chat-core/src/openapi-signing.test.ts packages/chat-openapi-adapter/src/index.test.ts`

## Blockers

None.
