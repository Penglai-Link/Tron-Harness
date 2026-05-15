# Test Task: Auth

## Execution Contract

- feature_slug: auth
- status: complete
- execution_mode: solo

## TDD Contract

Default to one bounded `tdd` row per AC slice. A `tdd` row owns RED -> GREEN -> focused VERIFY for the AC inside one write scope.

Only split into multiple workers when contracts are stable, write scopes are disjoint, and splitting reduces wall-clock time.

## Task Rows

- AC-AUTH-01 | phase: verify | owner: lead | scope: tron-backend/tests/auth_internal_login.rs; tron-frontend/tests/auth-smoke.spec.ts | command_ref: CMD-BE-TEST | status: done.
- AC-AUTH-02 | phase: verify | owner: lead | scope: tron-backend/tests/auth_oauth2_login.rs; tron-frontend/tests/auth-smoke.spec.ts | command_ref: CMD-BE-TEST | status: done.
- AC-AUTH-03 | phase: verify | owner: lead | scope: tron-frontend/tests/auth-smoke.spec.ts | command_ref: CMD-FE-AUTH-BROWSER | status: done.
- AC-AUTH-04 | phase: verify | owner: lead | scope: tron-frontend/tests/auth-smoke.spec.ts | command_ref: CMD-FE-AUTH-BROWSER | status: done.
- AC-AUTH-05 | phase: verify | owner: lead | scope: tron-backend/tests/auth_session_contract.rs | command_ref: CMD-BE-TEST | status: done.
- AC-AUTH-06 | phase: verify | owner: lead | scope: tron-backend/tests/auth_session_persistence.rs | command_ref: CMD-BE-TEST | status: done.
- AC-AUTH-07 | phase: verify | owner: lead | scope: tron-backend/tests/auth_security_boundary.rs | command_ref: CMD-BE-TEST | status: done.
- AC-AUTH-08 | phase: verify | owner: lead | scope: tron-frontend/tests/auth-smoke.spec.ts; tron-backend/tests/auth_user_management_boundary.rs | command_ref: CMD-FE-AUTH-BROWSER | status: done.

## Worktree Notes

Lead keeps RED and VERIFY evidence in the main auth worktree because execution mode is solo.

## Integration Notes

Frontend browser proof must cover both anonymous blocking and successful access to existing user-management routes after authentication.

## Open Decisions

- None.
