# Database Result: Workspace Management

## Summary

- SQLite migrations and repository wiring now persist the workspace tree plus direct-user and role-based access grants inside the same backend database used by admin users and roles.
- Persistence coverage passed under `rtk make be-test` on 2026-05-16.

## Evidence Rows

- AC-WS-06 | red: `master` had no workspace tables or access-grant persistence. | green: migrations `20260516_0006_create_workspace_management_workspaces.sql` and `20260516_0007_create_workspace_management_access_grants.sql` plus the SQLite workspace repository now persist hierarchy, direct-user grants, role grants, and statistics-backed counts; `rtk make be-test` passed. | command_ref: CMD-BE-TEST | browser: clicks=backend integration test helper initializes a temp SQLite database and exercises list, grant, and statistics routes; inputs=create and grant-replacement payloads plus authenticated route reads; network=workspace hierarchy, access, effective-user, and statistics requests persisted through SQLite-backed handlers; feedback=asserted `SUCCESS` envelopes and non-empty persisted records after writes; artifacts=`tron-backend/migrations/20260516_0006_create_workspace_management_workspaces.sql`, `tron-backend/migrations/20260516_0007_create_workspace_management_access_grants.sql`, `tron-backend/src/adapters/outbound/persistence/sqlite/workspace_repository.rs` | status: done.
- AC-WS-08 | red: `master` had no persisted subtree lifecycle state for archive or restore flows. | green: persisted subtree status now archives descendants together and restores them only when active ancestors exist while preserving prior grants; `rtk make be-test` passed. | command_ref: CMD-BE-TEST | browser: clicks=backend integration test helper archives `ENTERPRISE`, rejects restoring `TRON_CORE` first, then restores `ENTERPRISE`; inputs=no request body for lifecycle calls; network=SQLite-backed archive and restore routes under `/api/v1/admin/workspaces/**`; feedback=asserted subtree counts, archived-parent rejection, and restored `Active` status after persistence reload; artifacts=`tron-backend/tests/workspace_management_api.rs`, `tron-backend/src/adapters/outbound/persistence/sqlite/workspace_repository.rs`, `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs` | status: done.

## Changed Paths

- `tron-backend/migrations/20260516_0006_create_workspace_management_workspaces.sql`
- `tron-backend/migrations/20260516_0007_create_workspace_management_access_grants.sql`
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/workspace_repository.rs`

## Commands

- `rtk make be-test` passed on 2026-05-16.

## Blockers

- None.
