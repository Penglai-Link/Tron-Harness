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
- main_agent: lead agent on `feature/clouds3n/auth`
- dependency_features: `user-management-model`
- dependency_contracts: `AGENTS.md` uniform response envelope; existing `tron-frontend` route registry and shell; existing `tron-backend` user repository, bcrypt password model, and SQLite persistence

## Requirement

Create a real authentication feature for Tron so every non-auth page requires a valid session, internal operators can sign in with existing user-management accounts, and enterprise OAuth2 sign-in, starting with GitHub OAuth, can land in the same session model without inventing a second user source.

## Scope Boundary

This slice owns backend auth/session APIs, protected-route behavior, login and callback pages, cookie-based JWT session lifecycle, OAuth2 identity linking, and user-management integration at the authentication boundary only. It does not introduce authorization policy beyond authenticated access.

## Assumptions

- Existing user-management users are the source of truth for internal operators.
- Local development should stay usable on plain HTTP, so cookie security flags must be configurable.
- OAuth2 provider details can be driven by backend environment variables, with GitHub as the first live provider and any deterministic dev provider reserved for automated proof only.
- Existing role and space assignments remain managed data, not API authorization gates, in this slice.

## In Scope

- Internal username-or-email plus password login against the persisted user store.
- Short-lived access JWT plus rotating refresh JWT delivered through HttpOnly cookies.
- Backend session bootstrap, refresh, logout, and current-session inspection endpoints.
- OAuth2 authorize and callback endpoints, with GitHub as the first live provider and provider-subject reuse, verified-email matching, or first-login internal-user auto-provision.
- Persistent refresh-session storage and persistent OAuth2 identity links.
- Frontend route guarding, login bootstrap, logout flow, callback completion page, and redirect preservation.
- Backend protection for user-management APIs that now require an authenticated session.

## Out Of Scope

- Self-service signup, invitation acceptance, password change, MFA, or device trust.
- Role-based API authorization, permission matrices, or tenancy isolation changes.
- Admin configuration UI for OAuth2 providers.
- Invitation approval workflows, domain allowlists, or admin review gates for auto-provisioned OAuth2 accounts.
- Non-browser clients or token exchange flows outside the existing frontend console.

## Constraints

- Reuse the existing user-management persistence model instead of creating a second operator table.
- Keep the backend response envelope aligned with `code`, `message`, and `data`.
- Keep frontend auth state derived from backend session inspection instead of duplicating long-lived tokens in page-local state.
- Preserve current mock-or-api data-source conventions while ensuring auth failures do not silently bypass route protection.

## Feature Model

- Actors:
  - Anonymous operator.
  - Authenticated internal operator.
  - Authenticated OAuth2-linked operator.
- Core objects:
  - `AuthSession`: refresh-session record plus cookie-backed JWT lifecycle.
  - `AuthenticatedOperator`: stable user-management identity exposed to the frontend session model.
  - `OAuth2IdentityLink`: provider subject mapped to one internal user.
- Rules:
  - Only `Active` users can authenticate.
  - Archived users are rejected before a session is issued.
  - Frontend protected pages derive access from `GET /api/v1/auth/session`.
  - Internal and OAuth2 login must converge on the same session and user payload.
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
  - auth HTTP handlers and middleware.
  - auth application service, token codec, provider adapters, and persistence extensions.
  - protected user-management API boundary reused by the existing admin frontend.

## Repository Findings

- `tron-frontend/src/domain/auth/pages/LoginPage.tsx` is a static stub that only navigates to `/dashboard` and has no real auth integration.
- `tron-frontend/src/app/App.tsx` treats `/login` as special but has no authenticated-route guard or session bootstrap.
- `tron-frontend/src/shared/api/serviceConfig.ts` does not send browser credentials and currently swallows API errors back into mock data.
- `tron-backend` already has Axum routing, SQLite persistence, bcrypt password hashing, and active versus archived user state in the user-management domain.
- Seeded users exist under `tron-backend/src/adapters/outbound/persistence/sqlite/seed.rs`, but the feature currently lacks any login contract or known auth fixture conventions.

## Open Decisions

- None.
