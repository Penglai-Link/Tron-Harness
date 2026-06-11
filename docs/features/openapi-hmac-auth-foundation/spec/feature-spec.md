# Feature Spec: OpenAPI HMAC Auth Foundation

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: openapi-hmac-auth-foundation
- feature_name: OpenAPI HMAC Auth Foundation
- status: approved_with_defaults
- owner: clouds3n
- source_request: deliver the first bounded slice of Platform OpenAPI and External Access from `docs/prd.typ`
- main_agent: Codex lead agent on `feature/clouds3n/openapi-open-app-management/base`
- dependency_features: `auth`, `user-management-model`, `chat-management`
- dependency_contracts: `ApiResponse`, `RequestAuthContext`, `AuthenticatedAccess`, SQLite migration hooks, current open-api chat route registration

## Requirement

Replace the temporary public open-api header check with a reusable platform open-app foundation that persists open-app identities and grants, verifies `TRON-HMAC-SHA256-V1` signed requests, rejects bearer fallback, and produces a request auth context only after explicit acting-user and capability authorization succeed.

## Product Source Slices

- brainstorming_source: `docs/brainstorm/openapi-open-app-management/2026-06-11-12-30-27.md`
- prd_feature_headings: `Feature: Platform Integrations And External Access - Open-App Registry And Signed OpenAPI Access`
- prd_page_headings: `Page: /open-api/v1/* - Signed Agent Chat Integration`
- prd_rule_headings: `Rule: Platform Integrations And External Access - Open-App Key Lifecycle`, `Rule: Platform Integrations And External Access - Signed Request Verification`, `Rule: Platform Integrations And External Access - Acting User Authorization`, `Rule: Platform Integrations And External Access - Capability Authorization`, `Rule: Platform Integrations And External Access - Agent Chat Route Mapping`
- prd_exception_headings: `Exception: Platform Integrations And External Access - Invalid Or Replayed Signature`, `Exception: Platform Integrations And External Access - Unauthorized Acting User Or Operation`, `Exception: Platform Integrations And External Access - Rotated Or Disabled Open-App`

## Scope Boundary

This feature delivers the backend persistence, domain, and middleware foundation for signed public OpenAPI access. It does not yet add the `/settings/openapi` management UI, operator CRUD endpoints, or new public capability families beyond the existing agent-chat routes.

## Assumptions

- Platform admin and user directory foundations already exist and can supply active internal users by `userCode`.
- Existing public agent-chat routes remain the only external capability family in this version.
- HMAC verification requires reversible-at-rest secret storage, not one-way password hashing.
- Current route handlers can keep their business logic unchanged if middleware injects a valid `RequestAuthContext`.
- First-version grants remain platform-wide and explicit; no workspace-specific, role-wide, or dual-key overlap logic is required.

## In Scope

- New backend domain and application service for platform open-app records, active credentials, explicit user grants, capability-operation grants, and nonce replay protection.
- SQLite migrations and repository support for open-app storage.
- `TRON-HMAC-SHA256-V1` canonical request verification on `/open-api/v1/*`.
- Header contract change from `X-Tron-User-Id` to `X-Tron-User-Code`.
- Removal of bearer-token fallback and legacy `X-Tron-App-Key` direct auth from formal public open-api middleware.
- Route-to-capability metadata for the current public agent-chat surfaces.
- Backend automated tests that cover valid signatures, invalid signatures, replay/stale requests, denied users, denied operations, rotated or disabled open-apps, and CORS header updates.

## Out Of Scope

- `/settings/openapi` frontend page and operator UX.
- Open-app create, rotate, enable, disable, or grant-management HTTP admin routes.
- OAuth2 client credentials, asymmetric signatures, or role-wide acting-user grants.
- Dual-key grace periods during rotation.
- Additional public OpenAPI capability families beyond agent-chat.

## Constraints

- Public OpenAPI responses must keep the project-wide `{ code, message, data }` envelope.
- Formal external OpenAPI auth must accept only HMAC headers and must not accept bearer sessions.
- Acting user remains explicit via `X-Tron-User-Code`; signature success alone is insufficient.
- Capability authorization is evaluated by business capability plus operation, not raw path wildcards.
- Existing chat handlers should not gain open-app-specific business branching; middleware remains the enforcement point.
- SQLite migrations are hard-coded in repository constructors and must be wired explicitly.

## Feature Model

### Actors

- External integration caller: signs requests with `openAppCode`, current `appKey`, acting `userCode`, timestamp, and nonce.
- OpenAPI auth service: resolves open-app state, verifies HMAC, prevents replay, authorizes acting user and capability, and builds request auth context.
- Open-app repository: stores open-app identity, sealed current credential, grants, and nonce ledger.
- Public agent-chat routes: expose the current externally callable capability set.

### Core Entities

- `OpenApp`: platform-managed external integration identity with immutable public `openAppCode`, operator-facing metadata, and active or disabled status.
- `OpenAppCredential`: reversible-at-rest current HMAC secret plus fingerprint metadata; rotated credentials become unusable immediately.
- `OpenAppUserGrant`: explicit binding from one open-app to one active internal `userCode`.
- `OpenAppCapabilityGrant`: explicit binding from one open-app to one business capability plus operation pair.
- `OpenApiNonceRecord`: replay-protection ledger keyed by `openAppCode` plus nonce within the accepted timestamp window.
- `OpenApiRouteGrant`: in-memory mapping from current route metadata to capability-operation identity.

### Signed Request Flow

1. Middleware rejects requests that lack `X-Tron-App-Code`, `X-Tron-User-Code`, `X-Tron-Timestamp`, `X-Tron-Nonce`, `X-Tron-Signature-Version`, or `X-Tron-Signature`.
2. Middleware loads the active open-app and sealed current credential by `openAppCode`.
3. Middleware canonicalizes method, normalized path, normalized query, body hash, `openAppCode`, `userCode`, timestamp, and nonce.
4. Middleware verifies `TRON-HMAC-SHA256-V1` with constant-time comparison and enforces timestamp skew plus nonce uniqueness.
5. Middleware resolves the explicit acting `userCode`, then checks the open-app user grant and route capability-operation grant.
6. Only after both checks succeed does middleware build `RequestAuthContext` and forward the request to the handler.

## Page And Module Surface

- Backend middleware: `tron-backend/src/adapters/inbound/open_api/auth.rs`
- Public route metadata: `tron-backend/src/adapters/inbound/open_api/chats/*`
- New application service surface: `tron-backend/src/application/open_app/*`
- New domain surface: `tron-backend/src/domain/open_app/*`
- New SQLite repository surface: `tron-backend/src/adapters/outbound/persistence/sqlite/open_app_repository.rs`
- SQLite migrations: `tron-backend/migrations/*` and `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`
- No frontend page in this feature.

## Repository Findings

- Current middleware authenticates bearer access tokens first and only falls back to legacy open-api headers afterward.
- Legacy open-api header auth only checked non-empty legacy app headers, raw key headers, and `x-tron-user-id`, then resolved the user through `UserService::authenticate_open_api_user`.
- No current table stores open-app credentials, grants, or nonce replay state.
- Current public chat integration tests use the legacy header trio and will need signed-request fixtures.
- Existing backend response helpers already support the required uniform envelope shape.
- Repository-specific SQLite migrations are applied in per-repository constructors, not via one global migration runner.

## Open Decisions

- None. Build the backend foundation first, then layer management APIs and `/settings/openapi` UI in the next bounded feature.
