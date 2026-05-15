# Feature Report: Role Matrix Management

## Summary

Role Matrix is now a working admin workspace with real filters, a matrix-only editing surface, backend-backed catalog persistence, and user-management-aware role context. Backend regression, frontend regression, and browser interaction evidence all passed.

## Evidence Rows

- AC-RM-01 | red: original page exposed only placeholder summary state, no assignment context, and no durable role draft baseline | green: rebuilt matrix renders assignment counts, permission legend, and dirty indicators directly in role rows while the redundant summary cards stay absent | command_ref: CMD-RM-BROWSER | browser: clicks=navigate to `/admin/roles` and inspect role rows; inputs=none beyond initial page load; network=observed read requests for `/api/v1/admin/roles`, `/api/v1/admin/permission-domains`, and `/api/v1/admin/users`; feedback=role rows, linked-user counts, and saved/unsaved badges rendered visibly in the matrix without summary cards or a separate sidebar; artifacts=tron-frontend/tests/role-matrix-smoke.spec.ts line reporter output | status: done.
- AC-RM-02 | red: search and filter controls were placeholders, and the page still rendered a redundant `Users / Role Matrix / Audit Logs` selector in the header | green: debounced search, role-type filter, domain filter, and changed-only toggle all changed visible state, and browser verification confirmed the redundant cross-page selector no longer renders | command_ref: CMD-RM-BROWSER | browser: clicks=changed-only toggle; inputs=search `workspace`, role-type select `Custom`, domain select `Skills`; network=observed `/api/v1/admin/users`, `/api/v1/admin/users/statistics/*`, and role read requests during filtering; feedback=filtered matrix rows after debounce settles, focused domain column, and no header tab selector for `Users` or `Audit Logs`; artifacts=tron-frontend/tests/role-matrix-smoke.spec.ts line reporter output | status: done.
- AC-RM-03 | red: matrix cells and sidebar inspector metadata could not drive a meaningful role draft update | green: role selection, inline custom-role renaming, and permission toggles now update the working catalog directly from the matrix, while browser verification confirmed the `INSPECTOR` sidebar, detail editor card, and `n/4 permissions` labels are all gone | command_ref: CMD-RM-BROWSER | browser: clicks=select custom role and toggle matrix permissions; inputs=inline role-name edit on the selected custom row; network=read requests plus later persisted write path in the same scenario; feedback=selected role changed, custom role name updated in place, permission chips toggled, unsaved state became visible, and no removed editor surfaces remained; artifacts=tron-frontend/tests/role-matrix-smoke.spec.ts line reporter output | status: done.
- AC-RM-04 | red: Add/Clone/Reset/Delete actions were not backed by concrete state transitions, had inconsistent emphasis, and delete did not require a second confirmation | green: the top action row now uses simplified equal-width buttons, styles delete in red, suppresses rapid duplicate clicks, requires a confirm step before removal, and still creates rows, clones a role, exposes an inline rename field for the cloned row, resets saved changes, removes custom clones, and emits shared toast feedback | command_ref: CMD-RM-BROWSER | browser: clicks=rapid Add Role, matrix permission toggle, Clone, Reset, Delete, Confirm; inputs=inline rename of the cloned role to `Release Manager Copy`; network=frontend-local mutation only after initial reads and later persisted save; feedback=top-right toasts for create, clone, reset, delete-confirm, and delete completion plus visible row add/remove behavior from the main workspace without duplicate rapid-fire actions; artifacts=tron-frontend/tests/role-matrix-smoke.spec.ts line reporter output | status: done.
- AC-RM-05 | red: reload would lose role edits and discard could not restore the last committed baseline | green: Save issues `PUT /api/v1/admin/roles`, reload restores the saved role from backend state, and Discard reverts unsaved permission edits while changed-only mode empties correctly | command_ref: CMD-RM-BROWSER | browser: clicks=Save, browser reload, Show changed only, Discard, Clear Filters; inputs=custom-role creation plus matrix permission toggles; network=observed `PUT /api/v1/admin/roles` and subsequent `GET /api/v1/admin/roles`; feedback=save and discard toasts, saved custom role returned after reload, and empty-state recovery after discard; artifacts=tron-frontend/tests/role-matrix-smoke.spec.ts line reporter output | status: done.
- AC-RM-06 | red: backend had no role-matrix routes and could not persist a saved catalog | green: `rtk make be-test` passed with permission-domain read, seeded role-catalog read, and whole-catalog replacement persistence proven through `tron-backend/tests/role_matrix_api.rs` | command_ref: CMD-BE-TEST | browser: clicks=not applicable; inputs=not applicable; network=not applicable; feedback=backend contract validated through test assertions rather than UI feedback; artifacts=tron-backend/tests/role_matrix_api.rs plus successful `CMD-BE-TEST` output | backend: routes=`GET /api/v1/admin/permission-domains`, `GET /api/v1/admin/roles`, `PUT /api/v1/admin/roles`; persistence=fresh router against the same SQLite database still returns the saved custom role; artifacts=tron-backend/tests/role_matrix_api.rs plus successful `CMD-BE-TEST` output | status: done.
- AC-RM-07 | red: the rebuilt page or backend wiring could have broken `/admin/roles` rendering or user-management compatibility when the header badge, summary cards, and right-side inspector were removed | green: browser proof confirmed `/admin/roles` still works in API mode while preserving fixed user-management role codes, and `rtk make fe-check` passed on the same implementation | command_ref: CMD-RM-BROWSER | browser: clicks=none required beyond role-matrix interactions already covered in the same run; inputs=none required beyond the existing matrix interaction flow; network=observed admin read requests continue succeeding in API mode after the page loads; feedback=page headings, persisted role state, and fixed role-code rows remained intact without the `Role Matrix` badge, summary cards, or `INSPECTOR` sidebar; artifacts=tron-frontend/tests/role-matrix-smoke.spec.ts line reporter output plus successful `rtk make fe-check` output | status: done.

## Changed Paths

- `docs/features/role-matrix-mgmt/**`
- `tron-backend/migrations/20260515_0004_create_role_management_roles.sql`
- `tron-backend/migrations/20260515_0005_create_role_management_permissions.sql`
- `tron-backend/src/adapters/inbound/http/admin/roles/**`
- `tron-backend/src/adapters/outbound/persistence/sqlite/role_repository.rs`
- `tron-backend/src/application/role.rs`
- `tron-backend/src/domain/role.rs`
- `tron-backend/tests/role_matrix_api.rs`
- `tron-frontend/src/domain/admin/api/adminApi.ts`
- `tron-frontend/src/domain/admin/pages/RoleMatrixPage.tsx`
- `tron-frontend/tests/role-matrix-smoke.spec.ts`

## Commands

- @ref:spec/test-plan.md#command-index: `CMD-BE-TEST`, `CMD-FE-CHECK`, and `CMD-RM-BROWSER` executed successfully.

## Blockers

- None.
