# Acceptance: Auth

## Acceptance Index

- AC-AUTH-01 [must] [business] Active internal users can sign in with username or email plus password and receive a reusable authenticated session; proof: `tron-backend/tests/auth_internal_login.rs` plus `tron-frontend/tests/auth-smoke.spec.ts` passed on 2026-05-15.
- AC-AUTH-02 [must] [business] OAuth2 sign-in resolves an active internal user through provider subject or verified email and lands in the same session model; proof: `tron-backend/tests/auth_oauth2_login.rs` plus Playwright OAuth redirect proof passed on 2026-05-15.
- AC-AUTH-03 [must] [interaction] Anonymous access to any protected route redirects to `/login` and preserves the intended destination for post-login return; proof: `tron-frontend/tests/auth-smoke.spec.ts` verified `/dashboard -> /login?redirect=%2Fdashboard -> /dashboard`.
- AC-AUTH-04 [must] [interaction] The login surface offers internal credentials, OAuth2 entry, loading states, and visible auth failure feedback without leaking secrets; proof: real `LoginPage` provider loading and internal/OAuth2 entry shipped in `tron-frontend/src/domain/auth/pages/LoginPage.tsx` and exercised in Playwright.
- AC-AUTH-05 [must] [api] Auth endpoints use the uniform envelope, HttpOnly cookies, refresh rotation, and stable session inspection and logout contracts; proof: `tron-backend/tests/auth_session_contract.rs` passed on 2026-05-15.
- AC-AUTH-06 [must] [database] Refresh sessions and OAuth2 identity links persist only opaque or hashed identifiers without storing plaintext session secrets; proof: `tron-backend/tests/auth_session_persistence.rs` plus migrations `20260515_0004` and `20260515_0005` passed on 2026-05-15.
- AC-AUTH-07 [must] [security] Archived users, bad passwords, invalid refresh tokens, and mismatched OAuth2 state are rejected with stable auth error codes; proof: `tron-backend/tests/auth_security_boundary.rs` passed on 2026-05-15.
- AC-AUTH-08 [should] [regression] Existing shell and user-management routes remain usable after sign-in and return to login after logout or session loss; proof: `tron-backend/tests/auth_user_management_boundary.rs`, migrated legacy backend tests, and Playwright logout regression proof passed on 2026-05-15.

## Open Decisions

- None.
