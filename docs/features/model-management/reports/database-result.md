# Database Report: model-management

## Summary

Database work is complete. Model-management persistence now treats legacy restore or shadow rows as cleanup-only data and excludes them from active runtime reads. Startup migration `20260522_0014_remove_explicit_workspace_platform_restore.sql` deletes obsolete workspace rows that still use old relation kinds and removes stale workspace default assignments that referenced those deleted rows.

## Evidence Rows

- AC-MODEL-04 | red: the database still preserved enabled legacy workspace platform-shadow rows and default assignments that pointed to those obsolete row codes. | green: migration `20260522_0014_remove_explicit_workspace_platform_restore.sql` deletes legacy workspace `source_kind in ('platform', 'workspace_inherited')` rows and removes stale workspace defaults instead of rewriting compatibility data; `rtk cargo test --manifest-path tron-backend/Cargo.toml --test model_management_api -- --nocapture` passed `24 passed`. | command_ref: CMD-MODEL-BE-TEST | browser: clicks=none_persistence_focus; inputs=legacy row insertion happens in the backend migration test; network=not applicable because cleanup runs before HTTP handling; feedback=database state contains only the latest relation kinds after startup; artifacts=`tron-backend/migrations/20260522_0014_remove_explicit_workspace_platform_restore.sql`, `tron-backend/src/adapters/outbound/persistence/sqlite/model_repository.rs`, `tron-backend/tests/model_management_api.rs` | status: done.

## Changed Paths

- `tron-backend/migrations/20260522_0014_remove_explicit_workspace_platform_restore.sql`
- `tron-backend/src/adapters/outbound/persistence/sqlite/model_repository.rs`
- `tron-backend/tests/model_management_api.rs`

## Commands

- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test model_management_api -- --nocapture` passed with `24 passed` on 2026-05-24.

## Blockers

- None.
