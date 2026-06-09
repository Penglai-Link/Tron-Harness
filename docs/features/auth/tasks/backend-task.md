# Backend Task: Auth

## Execution Contract

- feature_slug: auth
- status: complete
- execution_mode: solo

## TDD Contract

Default to one bounded `tdd` row per AC slice. A `tdd` row owns RED -> GREEN -> focused VERIFY for the AC inside one write scope.

Only split into multiple workers when contracts are stable, write scopes are disjoint, and splitting reduces wall-clock time.

## Task Rows

- AC-AUTH-01 | phase: tdd | owner: lead | scope: tron-backend/src/application/auth/*; tron-backend/src/adapters/inbound/http/auth/*; tron-backend/tests/auth_internal_login.rs | command_ref: CMD-BE-TEST | status: done.
- AC-AUTH-02 | phase: tdd | owner: lead | scope: tron-backend/.env.example; tron-backend/application.yaml; tron-backend/src/application/user/service/*; tron-backend/src/config.rs; tron-backend/src/adapters/inbound/http/auth/*; tron-backend/tests/auth_oauth2_login.rs | command_ref: CMD-BE-TEST | status: done.
- AC-AUTH-05 | phase: tdd | owner: lead | scope: tron-backend/src/adapters/inbound/http/auth/*; tron-backend/src/application/user/service.rs; tron-backend/src/config.rs; tron-backend/tests/auth_session_contract.rs; tron-backend/tests/auth_bearer_context.rs | command_ref: CMD-BE-TEST | status: done.
- AC-AUTH-07 | phase: tdd | owner: lead | scope: tron-backend/src/adapters/inbound/http/auth/*; tron-backend/src/application/user/service/auth_ldap.rs; tron-backend/src/config.rs; tron-backend/tests/auth_security_boundary.rs | command_ref: CMD-BE-TEST | status: done.
- AC-AUTH-08 | phase: verify | owner: lead | scope: tron-backend/src/app.rs; tron-backend/src/adapters/inbound/http/admin/users/*; tron-backend/src/adapters/inbound/http/admin/roles/*; tron-backend/tests/auth_user_management_boundary.rs; tron-backend/tests/auth_bearer_context.rs | command_ref: CMD-BE-TEST | status: done.
- AC-AUTH-10 | phase: tdd | owner: lead | scope: tron-backend/src/app.rs; tron-backend/src/adapters/outbound/persistence/sqlite/bootstrap.rs; tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs; tron-backend/migrations/20260526_0017_bootstrap_platform_admin.sql; tron-backend/tests/bootstrap_admin.rs | command_ref: CMD-BE-BOOTSTRAP-ADMIN | status: done.

## Worktree Notes

Lead executes backend auth, session-contract, and LDAP or GitHub provider evolution directly in `feature/clouds3n/auth-ldap-login/base`.

## Integration Notes

Backend auth must protect user-management routes without introducing role-based authorization or a second user store, issue access JWTs only through auth grant responses, keep refresh state in `HttpOnly` cookies, expose request-scoped operator identity whenever protected handlers need save or update ownership, and let LDAP credential login reuse the same provider-subject-link and auto-provision path as GitHub.

## Open Decisions

- None.
