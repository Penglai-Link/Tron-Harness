# Feature Task: Role Matrix Management

## Execution Contract

- feature_slug: role-matrix-mgmt
- status: done
- execution_mode: solo

## TDD Contract

Default to one bounded `tdd` row per AC slice. A `tdd` row owns RED -> GREEN -> focused VERIFY for the AC inside one write scope.

Only split into multiple workers when contracts are stable, write scopes are disjoint, and splitting reduces wall-clock time.

## Task Rows

- AC-RM-01 | phase: tdd | owner: lead | scope: tron-frontend/src/domain/admin/pages/RoleMatrixPage.tsx tron-frontend/tests/role-matrix-smoke.spec.ts | command_ref: CMD-RM-BROWSER | status: done.
- AC-RM-02 | phase: tdd | owner: lead | scope: tron-frontend/src/domain/admin/pages/RoleMatrixPage.tsx tron-frontend/tests/role-matrix-smoke.spec.ts | command_ref: CMD-RM-BROWSER | status: done.
- AC-RM-03 | phase: tdd | owner: lead | scope: tron-frontend/src/domain/admin/pages/RoleMatrixPage.tsx tron-frontend/tests/role-matrix-smoke.spec.ts | command_ref: CMD-RM-BROWSER | status: done.
- AC-RM-04 | phase: tdd | owner: lead | scope: tron-frontend/src/domain/admin/pages/RoleMatrixPage.tsx tron-frontend/tests/role-matrix-smoke.spec.ts | command_ref: CMD-RM-BROWSER | status: done.
- AC-RM-05 | phase: tdd | owner: lead | scope: tron-frontend/src/domain/admin/pages/RoleMatrixPage.tsx tron-frontend/src/domain/admin/api/adminApi.ts tron-frontend/tests/role-matrix-smoke.spec.ts | command_ref: CMD-RM-BROWSER | status: done.
- AC-RM-06 | phase: tdd | owner: lead | scope: tron-backend/src/domain/role.rs tron-backend/src/application/role.rs tron-backend/src/adapters/inbound/http/admin/roles tron-backend/src/adapters/outbound/persistence/sqlite/role_repository.rs tron-backend/tests/role_matrix_api.rs | command_ref: CMD-BE-TEST | status: done.
- AC-RM-07 | phase: tdd | owner: lead | scope: docs/features/role-matrix-mgmt tron-frontend/src/domain/admin/pages/RoleMatrixPage.tsx tron-backend/src/app.rs | command_ref: CMD-RM-BROWSER | status: done.

## Worktree Notes

- All AC rows execute in the lead worktree `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/role-matrix-mgmt`.
- No worker worktrees are created because the feature is a single bounded full-stack slice and subagent dispatch is unnecessary.

## Integration Notes

- Preserve the fixed role-code contract already consumed by `user-management-model`.
- Keep the admin routes untouched outside `RoleMatrixPage.tsx` and the new admin role handlers.
- Browser tests own the interaction evidence; backend tests own the persistence proof; `make fe-check` owns frontend regression proof.

## Open Decisions

- None.
