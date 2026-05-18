# Backend Result: Workspace Management

## Summary

- Workspace backend now serves tree, detail, create, update, move, archive, restore, direct-user access, role access, effective-user, and statistics routes behind the uniform admin envelope.
- Workspace creation now generates unique random `WS-` workspace codes on the backend and ignores operator/name-derived code input.
- The 2026-05-18 selector evolution adds `/api/v1/admin/workspaces/options` and `/api/v1/admin/workspaces/switcher` as read-only projections for dropdowns and the top-left shell switcher.
- `rtk make be-test` passed on 2026-05-18 after the selector projection evolution.

## Evidence Rows

- AC-WS-02 | red: `master` had no workspace aggregate, service, repository, or API test coverage, so self-parent, cycle, archived-parent, and depth-four guards were absent. | green: `tron-backend/tests/workspace_management_api.rs` now proves level-four create and cycle move return `ADMIN_WORKSPACE_HIERARCHY_INVALID`; `rtk make be-test` passed. | command_ref: CMD-BE-TEST | browser: clicks=backend integration test helper invokes create and move routes for invalid hierarchy mutations; inputs=JSON payloads for `LEVEL_FOUR` create and `ENTERPRISE -> TRON_CORE` move; network=`POST /api/v1/admin/workspaces` and `POST /api/v1/admin/workspaces/ENTERPRISE/move`; feedback=asserted `400` envelopes with `ADMIN_WORKSPACE_HIERARCHY_INVALID`; artifacts=`tron-backend/tests/workspace_management_api.rs`, `tron-backend/src/application/workspace.rs`, `tron-backend/src/domain/workspace.rs` | status: done.
- AC-WS-05 | red: `master` had no effective-user read for the union of direct-user and role-derived grants. | green: effective-user reads now deduplicate users and preserve `Direct`, `Role`, and `DirectAndRole` source semantics plus owner propagation; `rtk make be-test` passed. | command_ref: CMD-BE-TEST | browser: clicks=backend integration test helper replaces direct-user and role grants before reading effective users; inputs=JSON payloads for `USR-LARGE-ID` direct access and `WORKSPACE_ADMIN` role access; network=`PUT /api/v1/admin/workspaces/PLATFORM/access/users`, `PUT /api/v1/admin/workspaces/PLATFORM/access/roles`, and `GET /api/v1/admin/workspaces/PLATFORM/access/effective-users`; feedback=asserted merged row source `DirectAndRole`, owner propagation, and matched role codes; artifacts=`tron-backend/tests/workspace_management_api.rs`, `tron-backend/src/application/workspace.rs`, `tron-backend/src/adapters/outbound/persistence/sqlite/workspace_repository.rs` | status: done.
- AC-WS-06 | red: `master` had no admin workspace router, repository wiring, or persistence coverage for hierarchy and grant routes. | green: workspace routes, repository wiring, and uniform-envelope responses now persist hierarchy, direct-user grants, role grants, and effective-user reads; `rtk make be-test` passed. | command_ref: CMD-BE-TEST | browser: clicks=backend integration test helper reads tree and statistics, then exercises grant and lifecycle routes; inputs=authenticated requests plus create, access-replacement, and archive payloads; network=`GET /api/v1/admin/workspaces`, `GET /api/v1/admin/workspaces/statistics/total-workspaces`, access routes, and lifecycle routes under `/api/v1/admin/workspaces/**`; feedback=asserted `SUCCESS` envelopes and persisted route availability through SQLite; artifacts=`tron-backend/tests/workspace_management_api.rs`, `tron-backend/src/adapters/inbound/http/admin/workspaces/routes.rs`, `tron-backend/src/adapters/outbound/persistence/sqlite/workspace_repository.rs` | status: done.
- AC-WS-08 | red: `master` had no subtree archive or restore lifecycle. | green: archive cascades through descendants and restore rejects archived-ancestor violations while preserving grant configuration; `rtk make be-test` passed. | command_ref: CMD-BE-TEST | browser: clicks=backend integration test helper archives `ENTERPRISE`, attempts child restore, then restores the root; inputs=no request body for archive or restore lifecycle calls; network=`POST /api/v1/admin/workspaces/ENTERPRISE/archive`, `POST /api/v1/admin/workspaces/TRON_CORE/restore`, and `POST /api/v1/admin/workspaces/ENTERPRISE/restore`; feedback=asserted subtree `affectedWorkspaceCount`, archived-parent rejection, and final `Active` status; artifacts=`tron-backend/tests/workspace_management_api.rs`, `tron-backend/src/application/workspace.rs`, `tron-backend/src/adapters/outbound/persistence/sqlite/workspace_repository.rs` | status: done.
- AC-WS-09 | red: `rtk make be-test` failed because creating a workspace without client-supplied `workspaceCode` returned `400`, proving the API still required client-controlled code. | green: `CreateWorkspaceRequest` no longer accepts `workspaceCode`; `WorkspaceService` generates a random `WS-` code and retries uniqueness against existing workspaces; `rtk make be-test` passed on 2026-05-18. | command_ref: CMD-BE-TEST | browser: clicks=backend integration test helper posts two create requests without `workspaceCode`; inputs=JSON bodies with `name` and optional `description` only; network=`POST /api/v1/admin/workspaces`; feedback=asserted `201` envelopes, generated `WS-` codes, codes not equal to `RISK`, and unique values across repeated names; artifacts=`tron-backend/tests/workspace_management_api.rs`, `tron-backend/src/application/workspace.rs`, `tron-backend/src/adapters/inbound/http/admin/workspaces/request.rs` | status: done.
- AC-WS-12 | red: task API-mode consumers had to fetch the full workspace tree and flatten it locally just to render one dropdown. | green: `/api/v1/admin/workspaces/options` now returns lineage-labeled selector records such as `Enterprise / Platform / Tron Core`; `rtk make be-test` passed on 2026-05-18. | command_ref: CMD-BE-TEST | browser: clicks=backend integration test helper reads the compact selector route; inputs=no request body; network=`GET /api/v1/admin/workspaces/options`; feedback=asserted labeled selector record for `TRON_CORE` without requiring the caller to flatten the full tree; artifacts=`tron-backend/tests/workspace_management_api.rs`, `tron-backend/src/application/workspace.rs`, `tron-backend/src/domain/workspace.rs`, `tron-backend/src/adapters/inbound/http/admin/workspaces/routes.rs` | status: done.
- AC-WS-13 | red: API mode had no grouped workspace projection for the top-left shell switcher, so the shell depended on hardcoded workspace-space mocks. | green: `/api/v1/admin/workspaces/switcher` now returns grouped `primarySpaces` and `secondarySpaces` derived from the active workspace tree; `rtk make be-test` passed on 2026-05-18. | command_ref: CMD-BE-TEST | browser: clicks=backend integration test helper reads the shell switcher route; inputs=no request body; network=`GET /api/v1/admin/workspaces/switcher`; feedback=asserted `ENTERPRISE` primary space and grouped `TRON_CORE` secondary space labeled under that root; artifacts=`tron-backend/tests/workspace_management_api.rs`, `tron-backend/src/application/workspace.rs`, `tron-backend/src/domain/workspace.rs`, `tron-backend/src/adapters/inbound/http/admin/workspaces/handler.rs` | status: done.

## Changed Paths

- `tron-backend/src/adapters/inbound/http/admin.rs`
- `tron-backend/src/adapters/inbound/http/admin/workspaces/error_mapper.rs`
- `tron-backend/src/adapters/inbound/http/admin/workspaces/handler.rs`
- `tron-backend/src/adapters/inbound/http/admin/workspaces/mod.rs`
- `tron-backend/src/adapters/inbound/http/admin/workspaces/request.rs`
- `tron-backend/Cargo.toml`
- `tron-backend/src/adapters/inbound/http/admin/workspaces/routes.rs`
- `tron-backend/src/app.rs`
- `tron-backend/src/application/workspace.rs`
- `tron-backend/src/domain/workspace.rs`
- `tron-backend/tests/workspace_management_api.rs`

## Commands

- `rtk make be-check` passed on 2026-05-16.
- `rtk make be-test` passed on 2026-05-16.
- `rtk make be-test` passed on 2026-05-18.
- `rtk make be-test` passed on 2026-05-18 after AC-WS-12 and AC-WS-13 implementation.

## Blockers

- None.
