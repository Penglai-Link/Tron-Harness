# Acceptance: Auth

## Acceptance Index

- AC-AUTH-01 [must] [business] Active internal users can sign in with username or email plus password and receive a reusable authenticated session; proof: `tron-backend/tests/auth_internal_login.rs` plus `tron-frontend/tests/auth-smoke.spec.ts` passed on 2026-05-15.
- AC-AUTH-02 [must] [business] GitHub OAuth2 sign-in reuses an existing provider-subject link when present, otherwise matches by verified email or auto-provisions one internal user, and then lands in the same session model; proof: `tron-backend/tests/auth_oauth2_login.rs` plus Playwright GitHub redirect proof passed on 2026-05-15.
- AC-AUTH-03 [must] [interaction] Anonymous access to any protected route redirects to `/login` and preserves the intended destination for post-login return; proof: `tron-frontend/tests/auth-smoke.spec.ts` verified `/dashboard -> /login?redirect=%2Fdashboard -> /dashboard`.
- AC-AUTH-04 [must] [interaction] The login surface offers internal credentials, a GitHub OAuth2 entry, loading states, and visible auth failure feedback without leaking secrets; proof: real `LoginPage` provider loading and internal or GitHub entry shipped in `tron-frontend/src/domain/auth/pages/LoginPage.tsx` and exercised in Playwright.
- AC-AUTH-05 [must] [api] Auth endpoints keep the uniform envelope, issue access JWTs in auth responses, accept protected access only through bearer headers, and preserve refresh-cookie rotation plus logout contracts; proof: `tron-backend/tests/auth_session_contract.rs` plus `tron-backend/tests/auth_bearer_context.rs` passed on 2026-05-15, and the frontend renewal regressions in `tron-frontend/tests/auth-smoke.spec.ts` passed on 2026-05-22.
- AC-AUTH-06 [must] [database] Refresh sessions and OAuth2 identity links persist only opaque or hashed identifiers without storing plaintext session secrets; proof: `tron-backend/tests/auth_session_persistence.rs` plus migrations `20260515_0004` and `20260515_0005` passed on 2026-05-15.
- AC-AUTH-07 [must] [security] Archived users, bad passwords, invalid refresh tokens, and mismatched OAuth2 state are rejected with stable auth error codes; proof: `tron-backend/tests/auth_security_boundary.rs` passed on 2026-05-15.
- AC-AUTH-08 [should] [regression] Protected shell and admin routes survive idle expiry, reload, logout, and refresh-session loss without regressing the operator flow; proof: `tron-frontend/tests/auth-smoke.spec.ts` passed 4 auth lifecycle scenarios on 2026-05-22.

## Open Decisions

- None.
