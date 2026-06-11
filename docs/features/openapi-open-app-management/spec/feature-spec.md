# Feature Spec: OpenAPI Open-App Management

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: openapi-open-app-management
- feature_name: OpenAPI Open-App Management
- status: approved_with_defaults
- owner: clouds3n
- source_request: deliver the platform-level `open-app` management page and admin APIs after the HMAC auth foundation
- main_agent: Codex lead agent on `feature/clouds3n/openapi-open-app-management/base`
- dependency_features: `openapi-hmac-auth-foundation`, `platform-settings-shell`, `user-management-model`
- dependency_contracts: `ApiResponse`, `OpenAppService`, current settings route registry, current admin HTTP route wiring

## Requirement

Add a platform-level `open-app` management surface under `/settings/openapi` so operators can create external integration identities, reveal a newly generated `appKey` exactly once on create or rotate, explicitly bind allowed acting users, explicitly bind allowed business openapi capability-operation pairs, and inspect the current system capability catalog grouped for management use.

## Product Source Slices

- brainstorming_source: `docs/brainstorm/openapi-open-app-management/2026-06-11-12-30-27.md`
- prd_feature_headings: `Feature: Platform Integrations And External Access - Open-App Registry And Signed OpenAPI Access`
- prd_page_headings: `Page: /settings/openapi - Open-App Management`
- prd_rule_headings: `Rule: Platform Integrations And External Access - Open-App Key Lifecycle`, `Rule: Platform Integrations And External Access - Explicit Acting User Binding`, `Rule: Platform Integrations And External Access - Capability Authorization`, `Rule: Platform Integrations And External Access - Platform-Level Management Boundary`
- prd_exception_headings: `Exception: Platform Integrations And External Access - One-Time Key Reveal`, `Exception: Platform Integrations And External Access - Invalid Grant Target`

## Scope Boundary

This feature delivers the platform admin HTTP APIs, service extensions, route-capability catalog reuse, `/settings/openapi` frontend pages, and the public `openAppCode` contract alignment required to manage current `open-app` records. It reuses the HMAC auth foundation and keeps signed adapter semantics aligned with the formal header contract.

## Assumptions

- The HMAC foundation feature remains the single source of truth for signing, credential storage, and runtime authorization.
- First-version open-app grants remain platform-wide and explicit; there is no role-wide or workspace-scoped acting-user grant model.
- The first system capability family remains `agent-chat`, but the management surface must present it as a categorized catalog rather than a hard-coded form-only string list.
- One active credential per open-app is sufficient; rotation revokes the previous secret immediately with no overlap window.
- Operators can manage explicit user bindings by user code in v1; rich user search or directory-picker UX is optional.

## In Scope

- Platform admin endpoints to list, create, inspect, update status and metadata, rotate keys, and replace user or capability grants for open-apps.
- A shared openapi capability catalog and route-grant registry that the auth middleware and management APIs both consume.
- `/settings/openapi` route, navigation entry, frontend API client, and management page.
- One-time reveal UX for freshly created or rotated `appKey`.
- Explicit visibility of current user grants and capability-operation grants per open-app.
- Frontend and backend tests for create, rotate, replace-grant, and settings-page flows.

## Out Of Scope

- Multiple concurrent active keys or grace-period rotation.
- Role-based, department-based, or workspace-based user authorization.
- Additional public openapi capability families beyond the currently exposed agent-chat routes.
- Audit log or secret re-download.

## Constraints

- Terminology must use `open-app`, not the existing app-management module naming.
- Only create and rotate responses may include the cleartext `appKey`; list and detail responses must never reveal it again.
- Capability grants are managed by business capability plus operation, not by raw route wildcard strings.
- The page is platform-level only and must live in the settings shell.
- Shared crypto, random, and time helpers remain under `shared` if new generic support is required.

## Feature Model

### Actors

- Platform operator: manages external integration records and grants.
- Open-app management service: owns lifecycle, grant replacement, and capability catalog output.
- Openapi capability catalog: describes the current internally supported external route families and operations.
- Settings frontend: displays open-apps, active grants, catalog groupings, and one-time secret reveal.

### Core Entities

- `ManagedOpenApp`: operator-facing open-app summary with status, public `openAppCode`, last rotation metadata, and secret fingerprint only.
- `ManagedOpenAppDetail`: summary plus current explicit user grants and capability-operation grants.
- `OpenApiCapabilityCatalogGroup`: grouped catalog entry such as `agent-chat`, with operator-facing description and child operations.
- `OpenApiCapabilityCatalogOperation`: one grantable business operation with stable `capabilityCode`, `operationCode`, method summary, and route list.
- `OpenAppSecretReveal`: one-time response payload containing `openAppCode`, generated `appKey`, and fingerprint immediately after create or rotate.

### Management Flow

1. Operator opens `/settings/openapi` and loads the current open-app list and capability catalog.
2. Operator creates an open-app with name and optional description; backend generates immutable `openAppCode` and current `appKey`.
3. Frontend shows the cleartext `appKey` exactly once in a reveal panel and warns that later views only allow rotate/reset.
4. Operator edits user grants by explicit `userCode` list and capability grants by selecting operations grouped under the current catalog.
5. Operator can rotate the key, which immediately revokes the prior secret and returns a fresh one-time reveal payload.
6. Operator can disable or re-enable the open-app without losing the grant configuration.

## Page And Module Surface

- Backend admin routes: `tron-backend/src/adapters/inbound/http/admin/open_apps/*`
- Backend capability registry reuse: `tron-backend/src/application/open_app/*` and `tron-backend/src/adapters/inbound/open_api/auth.rs`
- Frontend settings route: `tron-frontend/src/router/routes.tsx`, `tron-frontend/src/router/pageRegistry.ts`
- Frontend navigation: `tron-frontend/src/domain/settings/constants/settingsNavigation.ts`
- Frontend page and API client: `tron-frontend/src/domain/open-app/*`
- Frontend signed adapter alignment: `tron-frontend/packages/chat-core/*`, `tron-frontend/packages/chat-openapi-adapter/*`

## Repository Findings

- The current auth middleware embeds the route-to-capability mapping inline, so management cannot yet display the same catalog without duplicate definitions.
- The current `OpenAppService` supports create, rotate, disable, and incremental grant insert only; list/detail/update/replace flows are still missing.
- The settings shell already hosts platform-level pages for models, MCP servers, variables, tools, agents, and skills, so `/settings/openapi` can follow the same route family.
- The frontend signed OpenAPI adapter must stay aligned with the HMAC contract and use `openAppCode` rather than any separate public id field.
- Current shared UI primitives are sufficient for a split list/new/detail/edit management surface with form fields, badges, tables, and a confirm dialog.

## Open Decisions

- None. Build the platform admin management slice now, then handle the external adapter migration in the next bounded feature.
