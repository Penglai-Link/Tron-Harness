# Review Task: Auth

## Execution Contract

- feature_slug: auth
- status: complete
- execution_mode: solo

## TDD Contract

Default to one bounded `tdd` row per AC slice. A `tdd` row owns RED -> GREEN -> focused VERIFY for the AC inside one write scope.

Only split into multiple workers when contracts are stable, write scopes are disjoint, and splitting reduces wall-clock time.

## Task Rows

- AC-AUTH-01 | phase: review | owner: lead | scope: docs/features/auth/*; tron-backend/src/application/auth/*; tron-backend/tests/auth_internal_login.rs | command_ref: none | status: done.
- AC-AUTH-02 | phase: review | owner: lead | scope: docs/features/auth/*; tron-backend/src/application/user/service/auth_oauth.rs; tron-backend/src/application/user/service/auth_ldap.rs; tron-backend/src/config.rs; tron-backend/application.yaml; tron-backend/.env.example; tron-backend/tests/auth_oauth2_login.rs | command_ref: none | status: done.
- AC-AUTH-03 | phase: review | owner: lead | scope: docs/features/auth/*; tron-frontend/src/app/App.tsx; tron-frontend/tests/auth-smoke.spec.ts | command_ref: none | status: done.
- AC-AUTH-04 | phase: review | owner: lead | scope: docs/features/auth/*; tron-frontend/src/domain/auth/pages/LoginPage.tsx; tron-frontend/src/domain/auth/api/authApi.ts; tron-frontend/src/store/authStore.ts; tron-frontend/tests/auth-smoke.spec.ts | command_ref: none | status: done.
- AC-AUTH-05 | phase: review | owner: lead | scope: docs/features/auth/*; tron-backend/src/adapters/inbound/http/auth/*; tron-backend/tests/auth_session_contract.rs | command_ref: none | status: done.
- AC-AUTH-06 | phase: review | owner: lead | scope: docs/features/auth/*; tron-backend/src/adapters/outbound/persistence/sqlite/*; tron-backend/src/application/user/service/auth_ldap.rs; tron-backend/tests/auth_session_persistence.rs | command_ref: none | status: done.
- AC-AUTH-07 | phase: review | owner: lead | scope: docs/features/auth/*; tron-backend/src/adapters/inbound/http/auth/*; tron-backend/src/application/user/service/auth_ldap.rs; tron-backend/tests/auth_security_boundary.rs | command_ref: none | status: done.
- AC-AUTH-08 | phase: review | owner: lead | scope: docs/features/auth/*; tron-frontend/src/app/App.tsx; tron-backend/src/app.rs; tron-frontend/tests/auth-smoke.spec.ts | command_ref: none | status: done.
- AC-AUTH-09 | phase: review | owner: lead | scope: docs/features/auth/*; tron-frontend/src/app/App.tsx; tron-frontend/src/domain/auth/pages/NoWorkspaceAccessPage.tsx; tron-frontend/tests/auth-smoke.spec.ts | command_ref: none | status: done.
- AC-AUTH-10 | phase: review | owner: lead | scope: docs/features/auth/*; tron-backend/src/app.rs; tron-backend/migrations/20260526_0017_bootstrap_platform_admin.sql; tron-backend/tests/bootstrap_admin.rs; tron-frontend/tests/auth-smoke.spec.ts | command_ref: none | status: done.

## Worktree Notes

Final review stays in the main auth worktree on `feature/clouds3n/auth-ldap-login/base` because no delegated workers exist in solo mode.

## Integration Notes

Review must compare protected-route behavior, auth cookie handling, LDAP provider ordering, and shared external-identity reuse against the accepted auth feature docs.

## Open Decisions

- None.
