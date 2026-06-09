# Acceptance: Auth

## Acceptance Index

- AC-AUTH-01 [must] [business] Active internal users can sign in with username or email plus password and receive a reusable authenticated session; proof: `tron-backend/tests/auth_internal_login.rs` plus `tron-frontend/tests/auth-smoke.spec.ts` passed on 2026-05-15.
- AC-AUTH-02 [must] [business] GitHub OAuth2 and LDAP sign-in resolve to the same local user by subject link, active email match, or first-login auto-provision, then issue the standard authenticated session; proof: `tron-backend/tests/auth_oauth2_login.rs`, `tron-backend/src/application/user/service/auth_ldap.rs` tests, and Playwright login-surface proof were updated on 2026-06-09.
- AC-AUTH-03 [must] [interaction] Anonymous access to any protected route redirects to `/login` and preserves the intended destination for post-login return; proof: `tron-frontend/tests/auth-smoke.spec.ts` verified `/dashboard -> /login?redirect=%2Fdashboard -> /dashboard`.
- AC-AUTH-04 [must] [interaction] The login surface offers internal credentials, a GitHub OAuth2 entry, an LDAP entry below GitHub, loading states, and visible auth failure feedback without leaking secrets; proof: real `LoginPage` provider loading and internal, GitHub, or LDAP entries shipped in `tron-frontend/src/domain/auth/pages/LoginPage.tsx`; `rtk make fe-check` and `rtk npm --prefix tron-frontend exec playwright -- test auth-smoke --reporter=line` passed with 8 tests on 2026-06-09.
- AC-AUTH-05 [must] [api] Auth endpoints keep the uniform envelope, issue access JWTs in auth responses, accept protected access only through bearer headers, and preserve refresh-cookie rotation plus logout contracts; proof: `tron-backend/tests/auth_session_contract.rs` plus `tron-backend/tests/auth_bearer_context.rs` passed on 2026-05-15, and the frontend renewal regressions in `tron-frontend/tests/auth-smoke.spec.ts` passed on 2026-05-22.
- AC-AUTH-06 [must] [database] Refresh sessions and external identity links for GitHub OAuth2 or LDAP persist only opaque or hashed identifiers without storing plaintext session secrets; proof: `tron-backend/tests/auth_session_persistence.rs`, `tron-backend/src/application/user/service/auth_ldap.rs` tests, and migrations `20260515_0004` plus `20260515_0005` cover the persistence contract as of 2026-06-09.
- AC-AUTH-07 [must] [security] Archived users, bad internal or LDAP passwords, invalid refresh tokens, unavailable LDAP upstreams, and mismatched OAuth2 state are rejected with stable auth error codes; proof: `tron-backend/tests/auth_security_boundary.rs`, `tron-backend/src/application/user/service/auth_ldap.rs` tests, and `tron-backend/src/adapters/inbound/http/auth/error_mapper.rs` were updated on 2026-06-09.
- AC-AUTH-08 [should] [regression] Protected shell and admin routes survive idle expiry, reload, logout, and refresh-session loss without regressing the operator flow; proof: `tron-frontend/tests/auth-smoke.spec.ts` passed 4 auth lifecycle scenarios on 2026-05-22.
- AC-AUTH-09 [must] [interaction] Authenticated users without assigned workspace spaces see an authorization-request page instead of shell or business content; proof: `tron-frontend/tests/auth-smoke.spec.ts` passed on 2026-05-26.
- AC-AUTH-10 [must] [database] Empty production databases bootstrap a platform admin account with `ADMIN` role and root workspace access, while platform admins bypass the no-space authorization page; proof: `tron-backend/tests/bootstrap_admin.rs` and `tron-frontend/tests/auth-smoke.spec.ts` passed on 2026-05-26.

## Open Decisions

- None.
