# Database Task: Auth

## Execution Contract

- feature_slug: auth
- status: complete
- execution_mode: solo

## TDD Contract

Default to one bounded `tdd` row per AC slice. A `tdd` row owns RED -> GREEN -> focused VERIFY for the AC inside one write scope.

Only split into multiple workers when contracts are stable, write scopes are disjoint, and splitting reduces wall-clock time.

## Task Rows

- AC-AUTH-06 | phase: tdd | owner: lead | scope: tron-backend/src/adapters/outbound/persistence/sqlite/*; tron-backend/tests/auth_session_persistence.rs | command_ref: CMD-BE-TEST | status: done.
- AC-AUTH-07 | phase: support | owner: lead | scope: tron-backend/src/adapters/outbound/persistence/sqlite/*; tron-backend/src/adapters/inbound/http/auth/* | command_ref: CMD-BE-TEST | status: done.
- AC-AUTH-10 | phase: tdd | owner: lead | scope: tron-backend/migrations/20260526_0017_bootstrap_platform_admin.sql; tron-backend/src/adapters/outbound/persistence/sqlite/bootstrap.rs; tron-backend/tests/bootstrap_admin.rs | command_ref: CMD-BE-BOOTSTRAP-ADMIN | status: done.

## Worktree Notes

Lead executes database and persistence changes directly in `feature/clouds3n/auth`.

## Integration Notes

Auth schema changes must remain additive and reuse the existing user-management `users` table as the identity source.

## Open Decisions

- None.
