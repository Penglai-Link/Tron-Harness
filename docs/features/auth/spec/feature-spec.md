# Feature Spec: Auth

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: auth
- feature_name: Auth
- status: approved_with_defaults
- owner: clouds3n
- source_request: use `eliteforge-codex-superpower` to create and implement the `auth` feature with frontend and backend, related to `user-management`, supporting internal user login and OAuth2 login
- main_agent: lead agent on `feature/clouds3n/auth/base`
- dependency_features: `user-management-model`
- dependency_contracts: `AGENTS.md` uniform response envelope; existing `tron-frontend` route registry and shell; existing `tron-backend` user repository, bcrypt password model, and SQLite persistence

## Requirement

Create a real authentication feature for Tron so every non-auth page requires a valid JWT-backed session, internal operators can sign in with existing user-management accounts, protected backend APIs authenticate only through `Authorization: Bearer <access-jwt>`, refresh uses HttpOnly cookies only, the frontend silently renews access before idle expiry and retries the first expired-request 401 once after refresh, and enterprise OAuth2 sign-in, starting with GitHub OAuth, lands in the same session model without inventing a second user source.

## Scope Boundary

This slice owns backend auth/session APIs, protected-route behavior, login and callback pages, JWT session lifecycle with access-token response bodies plus refresh-cookie bootstrap, OAuth2 identity linking, and user-management integration at the authentication boundary only. It does not introduce authorization policy beyond authenticated access.

## Assumptions

- Existing user-management users are the source of truth for internal operators.
- Local development should stay usable on plain HTTP, so refresh-cookie security flags must remain configurable for the dev environment.
- OAuth2 provider details can be driven by backend environment variables, with GitHub as the first live provider and any deterministic dev provider reserved for automated proof only.
- Existing role and space assignments remain managed data, not API authorization gates, in this slice.

## In Scope

- Internal username-or-email plus password login against the persisted user store.
- Short-lived access JWT returned in auth response bodies plus rotating refresh JWT delivered through HttpOnly cookies only.
- `Authorization: Bearer <access-jwt>` parsing for every protected backend route, with no access-cookie fallback.
- Backend login, refresh, logout, OAuth callback completion, and access-token-backed current-session inspection endpoints.
- Frontend bootstrap that exchanges the refresh cookie for a fresh access token before protected pages render.
- Frontend session lifecycle that silently renews the in-memory access token before idle expiry, retries the first expired protected request after a successful refresh, and avoids redundant refresh churn when a newer bearer token is already available in memory.
- Request-scoped authenticated-user context propagation so protected handlers can reuse operator identity during save or update operations.
- OAuth2 authorize and callback endpoints, with GitHub as the first live provider and provider-subject reuse, verified-email matching, or first-login internal-user auto-provision.
- Persistent refresh-session storage and persistent OAuth2 identity links.
- Frontend route guarding, login bootstrap, logout flow, callback completion page, and redirect preservation.
- Backend protection for user-management APIs that now require an authenticated session.

## Out Of Scope

- Self-service signup, invitation acceptance, password change, MFA, or device trust.
- Role-based API authorization, permission matrices, or tenancy isolation changes.
- Admin configuration UI for OAuth2 providers.
- Invitation approval workflows, domain allowlists, or admin review gates for auto-provisioned OAuth2 accounts.
- New login flows or token issuance contracts outside the existing frontend console.

## Constraints

- Reuse the existing user-management persistence model instead of creating a second operator table.
- Keep the backend response envelope aligned with `code`, `message`, and `data`.
- Keep frontend auth state derived from backend-issued session or refresh responses instead of duplicating long-lived tokens in page-local state.
- Keep refresh-session material in cookies only, while protected APIs honor only the standard `Authorization` header for access-token authentication.
- Preserve current mock-or-api data-source conventions while ensuring auth failures do not silently bypass route protection.

## Feature Model

- Actors:
  - Anonymous operator.
  - Authenticated internal operator.
  - Authenticated OAuth2-linked operator.
- Core objects:
  - `AuthSession`: refresh-session record plus access-token grant lifecycle.
  - `AuthenticatedOperator`: stable user-management identity exposed to the frontend session model.
  - `OAuth2IdentityLink`: provider subject mapped to one internal user.
- Rules:
  - Only `Active` users can authenticate.
  - Archived users are rejected before a session is issued.
  - Frontend protected pages derive access from a freshly issued access token obtained via login, refresh, or OAuth callback bootstrap.
  - Protected backend routes accept only `Authorization: Bearer <access-jwt>`, and successful auth must expose the operator identity to downstream handlers.
  - Internal and OAuth2 login must converge on the same session and user payload.
  - Refresh cookies are used only by auth lifecycle endpoints and must not be required by general business APIs.
  - First-time GitHub sign-in may auto-provision one internal user with minimal default assignments, and later sign-ins must reuse the durable provider subject link.
- State transitions:
  - Anonymous -> internal login success -> authenticated.
  - Anonymous -> OAuth2 authorize -> callback success -> authenticated.
  - Authenticated -> refresh -> authenticated.
  - Authenticated -> logout or invalid refresh -> anonymous.

## Page And Module Surface

- `/login`: login surface for internal credentials, GitHub OAuth provider actions, busy-state feedback, and visible failure feedback.
- `/auth/callback`: transient callback route that finalizes OAuth2 login and resumes the intended protected destination.
- Protected shell routes: every existing route except `/login` and `/auth/callback` waits for session bootstrap and redirects anonymous users.
- Backend modules:
  - auth HTTP handlers, bearer-auth middleware, auth grant responses, and request auth context extractor.
  - auth application service, token codec, provider adapters, and persistence extensions.
  - protected user-management API boundary reused by the existing admin frontend.

## Repository Findings

- `tron-frontend/src/domain/auth/pages/LoginPage.tsx` is a static stub that only navigates to `/dashboard` and has no real auth integration.
- `tron-frontend/src/app/App.tsx` treats `/login` as special but has no authenticated-route guard or session bootstrap.
- `tron-frontend/src/shared/api/httpClient.ts` and `serviceConfig.ts` still assume cookie-oriented browser auth and need to be narrowed so business APIs depend on bearer headers while refresh remains auth-endpoint-specific.
- `tron-backend` already has Axum routing, SQLite persistence, bcrypt password hashing, and active versus archived user state in the user-management domain.
- The current auth delivery still mixes bearer and cookie access-token paths; the next evolution should remove access-token cookies entirely and keep only the refresh cookie for bootstrap or renewal.
- Seeded users exist under `tron-backend/src/adapters/outbound/persistence/sqlite/seed.rs`, but the feature currently lacks any login contract or known auth fixture conventions.
- The current frontend delivery refreshes only during bootstrap and marks the session unauthenticated on any protected-route 401, so long-lived admin forms can still lose in-progress edits unless access renewal and expired-request retry become proactive.

## Open Decisions

- None.
