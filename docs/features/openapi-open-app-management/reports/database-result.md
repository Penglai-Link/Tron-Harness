# Database Result: OpenAPI Open-App Management

## Summary

The management slice reuses the HMAC foundation schema and extends only repository queries plus transactional replacement helpers; no new DDL beyond the foundational migration is required for this feature.

## Evidence Rows

- AC-OAM-01 | red: the existing repository exposed only mutation-oriented open-app methods and lacked summary or detail reads for management. | green: `rtk make be-test` passed repository and API assertions proving list or detail reads surface app metadata, active fingerprint, and current grant counts without leaking cleartext secrets. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAM-03 | red: the repository could append grants incrementally but could not replace one dimension atomically for management edits. | green: `rtk make be-test` passed replacement coverage proving one open-app dimension is deleted and reinserted transactionally, validation failures do not persist invalid state, and the single-active-credential invariant remains intact. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.

## Changed Paths

- `tron-backend/migrations/20260611_0001_create_platform_open_apps.sql`
- `tron-backend/src/adapters/outbound/persistence/sqlite/open_app_repository.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/open_app_repository/tests.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`

## Commands

- `rtk make be-check`
- `rtk make be-test`

## Blockers

None.
