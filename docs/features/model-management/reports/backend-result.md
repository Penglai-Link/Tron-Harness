# Backend Report: model-management

## Summary

Backend model management is complete. Workspace catalog resolution now treats platform models and inheritable workspace-local models as slot roots, resolves current-workspace overlays through explicit workspace overrides and workspace status patches, and returns only `LOCAL`, `INHERITED`, or `OVERRIDE` in workspace views. Workspace defaults are slot-based, inherited rows cannot be deleted, override rows can be reverted, and latest-only cleanup removes obsolete restore/shadow rows instead of preserving compatibility state.

## Evidence Rows

- AC-MODEL-01 | red: workspace catalog responses still exposed platform-derived rows as `PLATFORM` or treated parent overrides as if they were current-workspace overrides. | green: backend resolver now returns `LOCAL`, `INHERITED`, or `OVERRIDE` for workspace rows and keeps nearest-upstream workspace lineage in `sourceWorkspaceCode` or `sourceWorkspaceName`; `rtk cargo test --manifest-path tron-backend/Cargo.toml --test model_management_api -- --nocapture` passed `24 passed`. | command_ref: CMD-MODEL-BE-TEST | browser: clicks=none_contract_focus; inputs=create parent local roots and parent overrides in API tests; network=`GET /api/v1/models/workspaces/TRON_CORE` proved inherited parent rows and current local rows resolve with the correct source kinds; feedback=workspace catalog source classification matches the latest contract; artifacts=`tron-backend/src/application/model.rs`, `tron-backend/src/domain/model.rs`, `tron-backend/tests/model_management_api.rs` | status: done.
- AC-MODEL-02 | red: inherited rows could still be deleted, disabling an inherited row forced the workspace API to relabel the row as `Override`, and detail-page verify still only accepted saved records. | green: inherited delete is rejected, inherited disable keeps the row visible with `sourceKind = INHERITED`, local delete stays available, override delete reverts to inherited, and both platform and workspace verify endpoints now accept an optional unsaved draft payload without persisting it; `rtk cargo test --manifest-path tron-backend/Cargo.toml --test model_management_api -- --nocapture` passed `24 passed`. | command_ref: CMD-MODEL-BE-TEST | browser: clicks=none_contract_focus; inputs=disable inherited platform rows, delete local rows, delete override rows, and send unsaved draft verify payloads for platform plus workspace detail routes in API tests; network=`POST /api/v1/models/workspaces/TRON_CORE/{modelCode}/disable`, `DELETE /api/v1/models/workspaces/TRON_CORE/{modelCode}`, `POST /api/v1/admin/platform/models/{modelCode}/verify`, and `POST /api/v1/models/workspaces/TRON_CORE/{modelCode}/verify` returned the expected latest-only semantics; feedback=workspace mutations and draft verification now align with the new lifecycle rules; artifacts=`tron-backend/src/application/model.rs`, `tron-backend/tests/model_management_api.rs` | status: done.
- AC-MODEL-03 | red: workspace defaults were still tied to mutable override records, so inherited rows could not be selected safely and override or revert flows could silently break the default. | green: defaults are stored and resolved by visible slot `modelCode`, inherited and local rows can both be selected, and current-workspace defaults do not mutate platform defaults; `rtk cargo test --manifest-path tron-backend/Cargo.toml --test model_management_api -- --nocapture` passed `24 passed`. | command_ref: CMD-MODEL-BE-TEST | browser: clicks=none_contract_focus; inputs=set platform defaults, workspace defaults, and current-workspace overrides in API tests; network=workspace catalog default rows stayed stable while slot definitions changed; feedback=default resolution follows current workspace, nearest ancestor, then platform fallback; artifacts=`tron-backend/src/application/model.rs`, `tron-backend/tests/model_management_api.rs` | status: done.
- AC-MODEL-04 | red: repository startup still tolerated legacy workspace `source_kind = platform` or `workspace_inherited` rows as active behavior. | green: runtime logic now uses only `platform`, `workspace_local`, `workspace_override`, and `workspace_status_patch` persistence semantics, and migration `20260522_0014_remove_explicit_workspace_platform_restore.sql` deletes obsolete legacy rows and stale defaults; `rtk cargo check --manifest-path tron-backend/Cargo.toml` and `rtk cargo test --manifest-path tron-backend/Cargo.toml --test model_management_api -- --nocapture` passed. | command_ref: CMD-MODEL-BE-TEST | browser: clicks=none_cleanup_focus; inputs=legacy cleanup is exercised by the migration test rather than UI input; network=repository startup runs the cleanup before workspace-model reads; feedback=deprecated restore/shadow compatibility is gone from active backend behavior; artifacts=`tron-backend/src/adapters/outbound/persistence/sqlite/model_repository.rs`, `tron-backend/migrations/20260522_0014_remove_explicit_workspace_platform_restore.sql`, `tron-backend/tests/model_management_api.rs` | status: done.

## Changed Paths

- `tron-backend/src/application/model.rs`
- `tron-backend/src/domain/model.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/model_repository.rs`
- `tron-backend/migrations/20260522_0014_remove_explicit_workspace_platform_restore.sql`
- `tron-backend/tests/model_management_api.rs`

## Commands

- `rtk make be-check` passed on 2026-05-23.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test model_management_api -- --nocapture` passed with `24 passed` on 2026-05-24.

## Blockers

- None.
