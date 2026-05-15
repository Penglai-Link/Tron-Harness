# Database Design: Auth

## Database Decision

- status: approved
- approved_by: clouds3n
- approved_at: 2026-05-15
- rationale: the user explicitly requested frontend plus backend auth delivery, and this session lifecycle requires durable persistence tied to existing user-management users

## Existing Schema Impact

- Reuse the existing `users` table and current password hash fields from the user-management feature.
- Do not duplicate internal operator profile data in auth-owned tables.
- Existing password reset events remain unchanged and continue to record reset history independently of auth sessions.

## Proposed Schema

- Extend persistence with `auth_refresh_sessions`.
  - `id`: Snowflake ID.
  - `session_code`: stable external session handle.
  - `user_id`, `user_code`: internal operator reference.
  - `auth_method`: `internal` or `oauth2`.
  - `provider_code`: nullable for internal login, stable provider code for OAuth2.
  - `refresh_token_hash`: hash of the refresh JWT value or signature material.
  - `expires_at`, `created_at`, `updated_at`, `revoked_at`, `last_used_at`.
- Extend persistence with `user_oauth_identities`.
  - `id`: Snowflake ID.
  - `user_id`, `user_code`: internal operator reference.
  - `provider_code`: stable provider identifier.
  - `provider_subject`: stable external subject identifier.
  - `email`: last verified email returned by the provider.
  - `display_name`: last provider display name snapshot.
  - `linked_at`, `updated_at`.
- No plaintext access token, refresh token, or provider secret is stored in SQLite.

## Relationships And Cardinality

- One `users` row to many `auth_refresh_sessions`.
- One `users` row to many `user_oauth_identities`.
- One OAuth2 identity link belongs to exactly one internal user.
- Internal login may create many refresh sessions without any OAuth2 identity link.

## Migration Plan

- Add one migration for `auth_refresh_sessions`.
- Add one migration for `user_oauth_identities`.
- Seed one deterministic development auth fixture user with a known login password while preserving existing user-management seeds.
- Keep migrations additive and backward-compatible with the current SQLite file layout.

## Query And Performance Plan

- Unique index on `auth_refresh_sessions.session_code`.
- Index on `auth_refresh_sessions.user_code`.
- Index on `auth_refresh_sessions.expires_at` plus `revoked_at` for validation and cleanup.
- Unique composite index on `user_oauth_identities(provider_code, provider_subject)`.
- Supporting index on `user_oauth_identities(user_code)` for session reads and link updates.

## Data Governance

- Access tokens remain stateless JWTs and never persist server-side.
- Refresh token persistence uses hashed token material only.
- OAuth2 provider secrets stay in environment variables, not the database.
- Archived users keep historical auth rows, but new sessions cannot be created for them.

## Test Data And Fixtures

- Add one active internal auth fixture with a documented known password for internal-login proof.
- Add one archived auth fixture to prove archived-login rejection.
- Add one deterministic development OAuth2 profile for proof, and verify that first-login GitHub profiles can auto-provision an internal user when no matching email exists.

## Validation Commands

- `rtk make be-test`
- `rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/auth --phase spec --complete-spec`
