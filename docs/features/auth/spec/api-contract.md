# API Contract: Auth

## API Overview

The auth feature introduces a small browser-oriented session API. Internal login and OAuth2 login converge on one authenticated session contract returned through the project uniform response envelope and HttpOnly cookies.

## Endpoints

| method | path | purpose |
| --- | --- | --- |
| GET | `/api/v1/auth/providers` | list enabled login providers for the frontend login surface |
| POST | `/api/v1/auth/login` | authenticate an internal operator with username-or-email plus password |
| GET | `/api/v1/auth/session` | return the current authenticated operator if cookies are valid |
| POST | `/api/v1/auth/refresh` | rotate access and refresh cookies for an existing refresh session |
| POST | `/api/v1/auth/logout` | revoke the current refresh session and clear auth cookies |
| GET | `/api/v1/auth/oauth2/authorize/{provider}` | start OAuth2 login and redirect the browser to the provider |
| GET | `/api/v1/auth/oauth2/callback/{provider}` | complete OAuth2 login, issue cookies, and redirect back to the frontend callback route |

## Auth And Permissions

- `GET /api/v1/auth/providers`, `POST /api/v1/auth/login`, and the OAuth2 authorize or callback endpoints are public.
- `GET /api/v1/auth/session`, `POST /api/v1/auth/refresh`, and `POST /api/v1/auth/logout` require auth cookies or refresh-session material.
- Existing admin user-management APIs become authenticated-only in this feature.
- Authentication proves operator identity only. Roles and permissions remain out of scope for API authorization.

## Error Contract

- Invalid credentials: HTTP 401 with `AUTH_INVALID_CREDENTIALS`.
- Archived or inactive operator: HTTP 403 with `AUTH_USER_INACTIVE`.
- Missing or expired session: HTTP 401 with `AUTH_UNAUTHORIZED`.
- Invalid or revoked refresh token: HTTP 401 with `AUTH_REFRESH_INVALID`.
- OAuth2 state mismatch or provider callback failure: HTTP 400 with stable `AUTH_OAUTH2_*` error codes.
- Unknown provider: HTTP 404 with `AUTH_PROVIDER_NOT_FOUND`.

## Frontend Backend Boundary

- The frontend never constructs or validates JWTs directly.
- The frontend learns auth state from `GET /api/v1/auth/session` and browser redirects.
- Login, logout, and refresh are same-origin calls that rely on browser cookies with `credentials: include`.
- The frontend may use mock domain data for non-auth business areas, but route protection and current-session truth come only from backend auth APIs.

## Database And Transaction Boundary

- Internal login reads the existing user store plus auth session persistence and writes a new refresh session row atomically enough for local SQLite use.
- OAuth2 callback may read or upsert one identity-link row and then create or rotate a refresh session row.
- Logout and refresh only touch auth-owned persistence, not user-management domain data.

## Mock Or Adapter Expectations

- Backend tests use the deterministic development OAuth2 provider adapter instead of external network calls.
- Frontend browser tests intercept auth API calls or development-provider redirects and assert visible redirect, feedback, and protected-route behavior.
- The callback route must tolerate real backend redirects and mocked browser-test redirects through the same frontend contract.
