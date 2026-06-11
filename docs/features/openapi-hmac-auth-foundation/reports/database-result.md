# Database Report: OpenAPI HMAC Auth Foundation

## Summary

The SQLite foundation now persists platform open-app identities, one active credential, explicit grants, and replay-protection nonces behind the signed open-api auth path.

## Evidence Rows

- AC-OAH-01 | red: no repository or schema could persist platform open-app identities, active credentials, grants, or replay nonces before the foundation landed. | green: `rtk make be-test` passed repository and integration coverage proving migrations create the open-app tables, one active credential remains after rotation, explicit user and capability grants load into auth state, and duplicate app nonces are rejected. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.

## Changed Paths

- `tron-backend/migrations/20260611_0001_create_platform_open_apps.sql`
- `tron-backend/src/adapters/outbound/persistence/sqlite/open_app_repository.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/open_app_repository/tests.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`
- `tron-backend/src/domain/open_app/`
- `tron-backend/src/shared/crypto.rs`
- `tron-backend/src/shared/random.rs`
- `tron-backend/src/shared/time.rs`

## Commands

- `rtk make be-test`
- `rtk cargo test --manifest-path tron-backend/Cargo.toml open_app_repository -- --nocapture`

## Blockers

None.
