# API Contract: Auth

## API Overview

The auth feature introduces a browser-oriented token API with one strict authentication contract. Internal login and GitHub OAuth2 login converge on one authenticated session model returned through the project uniform response envelope; auth grant responses return the short-lived access JWT in the body, refresh state stays in HttpOnly cookies only, and protected APIs honor only `Authorization: Bearer <access-jwt>`.

## Endpoints

| method | path | purpose |
| --- | --- | --- |
| GET | `/api/v1/auth/providers` | list enabled login providers for the frontend login surface |
| POST | `/api/v1/auth/login` | authenticate an internal operator with username-or-email plus password |
| GET | `/api/v1/auth/session` | return the current authenticated operator for a valid bearer access JWT |
| POST | `/api/v1/auth/refresh` | rotate the refresh cookie and return a fresh bearer access JWT plus session payload |
| POST | `/api/v1/auth/logout` | revoke the current refresh session and clear auth cookies |
| GET | `/api/v1/auth/oauth2/authorize/{provider}` | start OAuth2 login and redirect the browser to the provider |
| GET | `/api/v1/auth/oauth2/callback/{provider}` | complete OAuth2 login, issue cookies, and redirect back to the frontend callback route |

## Auth And Permissions

- `GET /api/v1/auth/providers`, `POST /api/v1/auth/login`, and the OAuth2 authorize or callback endpoints are public.
- `POST /api/v1/auth/login` returns the authenticated session plus a short-lived access JWT in the response body and sets only the refresh cookie.
- `GET /api/v1/auth/session` accepts the short-lived access JWT from `Authorization: Bearer` only.
- `POST /api/v1/auth/refresh` remains cookie-oriented, rotates the refresh-session cookie, and returns a fresh access JWT in the response body.
- `POST /api/v1/auth/logout` revokes the current refresh session through the refresh cookie and clears auth cookies.
- Existing admin user-management APIs become authenticated-only in this feature and accept the standard bearer header only.
- Authentication proves operator identity only. Roles and permissions remain out of scope for API authorization.

## Error Contract

- Invalid credentials: HTTP 401 with `AUTH_INVALID_CREDENTIALS`.
- Archived or inactive operator: HTTP 403 with `AUTH_USER_INACTIVE`.
- Missing or expired session: HTTP 401 with `AUTH_UNAUTHORIZED`.
- Invalid or revoked refresh token: HTTP 401 with `AUTH_REFRESH_INVALID`.
- OAuth2 state mismatch or provider callback failure: HTTP 400 with stable `AUTH_OAUTH2_*` error codes.
- Unknown provider: HTTP 404 with `AUTH_PROVIDER_NOT_FOUND`.

## Frontend Backend Boundary

- The frontend never constructs or validates JWTs directly, but it does hold the current access JWT in runtime state and sends it through the `Authorization` header.
- The frontend learns auth state from login or refresh responses and may use `GET /api/v1/auth/session` only as a bearer-token-backed inspection endpoint.
- Login, refresh, logout, and OAuth callback bootstrap remain same-origin calls so the browser can receive or send the refresh cookie.
- Protected backend handlers can extract the authenticated operator from request context after bearer authentication succeeds.
- The frontend may use mock domain data for non-auth business areas, but route protection and current-session truth come only from backend auth APIs.

## Database And Transaction Boundary

- Internal login reads the existing user store plus auth session persistence and writes a new refresh session row atomically enough for local SQLite use.
- OAuth2 callback may create one internal user, upsert one identity-link row, and then create or rotate a refresh session row.
- Logout and refresh only touch auth-owned persistence, not user-management domain data.

## Mock Or Adapter Expectations

- Backend tests use a local GitHub OAuth mock server instead of live GitHub network calls.
- Frontend browser tests intercept auth API calls or GitHub-provider redirects and assert visible redirect, feedback, and protected-route behavior.
- The callback route must tolerate real backend redirects and mocked browser-test redirects through the same frontend contract.
