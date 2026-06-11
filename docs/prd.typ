= PRD: Tron AI Workspace Operations Console

#import "@preview/mmdr:0.2.2": mermaid

== Summary
- Product: Tron is an internal AI workspace operations console for enterprise R&D teams.
- Problem: teams need one authenticated control plane to govern users, workspaces, AI assets, task delivery, scheduled agent workflows, and controlled external API access without splitting context across separate tools.
- Target users: platform admins, workspace operators, AI workflow maintainers, engineering leads, and approved external integration maintainers.
- Outcome: a workspace-scoped console that combines secure access, hierarchical governance, reusable AI configuration, delivery workflows, runtime visibility, and signed external access in one backend-served web app.

== Background
- Why now: existing open-api chat routes already expose public endpoints, but the current auth path still accepts bearer tokens, treats app headers as non-empty strings, and lacks app-level lifecycle, user grants, capability grants, and replay protection.
- Current pain: platform admins cannot create or rotate formal external credentials, cannot explicitly authorize which internal users or public API operations an external caller may use, and cannot rely on the current open-api path as a stable integration contract.
- Business context: Tron is expanding from internal operator workflows into controlled external integration scenarios, starting with agent-chat public access, while preserving the same admin shell, uniform API envelope, and backend-owned governance model.

== Goals
- Unify authentication, workspace selection, and operator workflows inside one persistent console shell.
- Enforce workspace-scoped governance for users, roles, apps, tasks, automations, models, skills, prompts, variables, and data surfaces.
- Support inheritance-aware configuration where platform or parent workspace definitions can be reused, overridden, or localized.
- Give operators a practical delivery loop for app registration, task planning, assignment, and automation scheduling.
- Give platform admins one governed external-access surface for creating open-app identities, delegating approved users, and authorizing approved public API operations.
- Require formal HMAC-signed open-api access for external callers instead of bearer-token fallback.
- Keep the product deployable as one integrated backend-driven runtime artifact.
- Ship locale-aware UI copy and runtime formatting, with `en-US` as fallback and `zh-CN` as a required production locale.

== Non-Goals
- Public self-service signup, customer-facing tenancy, billing, or marketplace monetization.
- Full enterprise IAM administration such as MFA policy, SCIM, device trust, or external IdP management UI.
- Distributed scheduler coordination, multi-node leader election, or large-scale runtime orchestration in this version.
- Deep external import and sync flows for apps, automations, or registries beyond the scoped feature specs.
- Fine-grained policy engines or deny-rule conflict resolution beyond the current authenticated admin and workspace model.
- OAuth2 client credentials, role-wide external grants, and overlapping dual-key rotation windows for the first external-access release.

== Scope
- In scope:
  - authenticated console access with internal credentials and GitHub OAuth
  - user, role, and workspace administration
  - workspace-scoped app, task, and automation workflows
  - AI asset governance for models, prompts, skills, variables, and resources
  - platform-level open-app management for signed public API access
  - HMAC-only public agent-chat access with explicit acting-user and API authorization
  - memory, knowledge, and observability surfaces aligned to workspace operations
  - integrated frontend-plus-backend runtime with SQLite-backed persistence for the current version
- Out of scope:
  - public onboarding and customer account lifecycle
  - multi-cloud deployment control plane
  - external billing, quota purchasing, or marketplace distribution
  - production-grade distributed execution fabric
  - additional public capability families beyond the first agent-chat external-access release

== Key Requirements
- Every non-auth management route must require a valid authenticated session and preserve shell state during internal navigation.
- Authenticated users without assigned workspace spaces must be stopped on a dedicated authorization-request page and must not operate shell navigation or business pages until a responsible owner grants space access.
- A fresh deployment must initialize one platform administrator account with the fixed `ADMIN` platform role and root workspace access so the first owner can assign spaces; that platform administrator must not be trapped by the no-space authorization page.
- Workspace context must be first-class: selected workspace drives visible apps, tasks, automations, effective users, and inherited AI configuration.
- Core registries must support operator-safe list, search, pagination, lifecycle controls, and consistent feedback patterns.
- User and role administration must be stable enough to back login, assignment, workspace grants, task assignee selection, and external acting-user authorization from one shared source of truth.
- App and task workflows must support real backend persistence, role-aware task statuses, and workspace-valid task assignments.
- Automations must support create, edit, run-now, status transitions, and execution history within the selected workspace.
- Platform admins must be able to create, disable, and rotate platform-level open-app credentials from the settings shell and see a generated appKey in plaintext only at create or rotate time.
- External open-api requests must carry explicit app, user, timestamp, nonce, and signature headers and must pass both acting-user authorization and business capability or operation authorization.
- Public open-api routes must reject bearer-token fallback and protect against stale timestamps, replayed nonces, invalid signatures, revoked keys, and disabled open-apps.
- The product must keep locale-aware UI resolution and runtime value formatting rather than hard-coded English strings in page code.
- API responses must remain consistent enough for shared frontend hooks and predictable admin workflows.

== Feature Requirements
=== Module: Console Access And Identity
==== Feature: Console Access and Identity
- name: Console Access and Identity
- description: Authenticate operators, bootstrap protected shell state, and keep the backend as the source of truth for active session identity.
- user or role: platform admin, workspace operator, engineering lead
- entry point: `/login`, OAuth callback routes, protected backend APIs
- trigger: operator signs in, refreshes, resumes a session, or logs out
- preconditions: internal user or linked external identity exists and remains active
- main flow: login or callback issues a valid access identity, protected routes load shell state, refresh restores the session when possible, and logout clears the operator session contract
- inputs and outputs: login credentials or OAuth callback inputs in; authenticated session, refresh state, and protected API access out
- permissions or visibility: available only to active internal users with allowed providers and space access
- related page headings: `Page: /login - Operator Login`
- related business rule headings: `Rule: Console Access And Identity - Protected Session Contract`
- related exception headings: `Exception: Console Access And Identity - No Space Access`

=== Module: Platform Integrations And External Access
==== Feature: Platform Integrations And External Access - Open-App Registry And Signed OpenAPI Access
- name: Open-App Registry And Signed OpenAPI Access
- description: Let platform admins register external integration identities and govern signed open-api access to the current public agent-chat capability.
- user or role: platform admin, approved external integration maintainer, granted acting internal user
- entry point: `/settings/openapi`, `/open-api/v1/*`
- trigger: admin creates or updates an open-app, rotates a key, grants a user or capability, or an external caller sends a signed request
- preconditions: operator is authenticated as a platform admin, open-app is active, acting user is active, and the requested capability or operation is granted
- main flow: admin manages open-apps in settings; system generates one active appKey per open-app; external caller signs requests with HMAC; middleware validates timestamp, nonce, signature, acting user, and capability before the request reaches the handler
- inputs and outputs: admin form inputs and route parameters in; open-app records, one-time appKey reveal, grant records, and signed API responses out
- permissions or visibility: management UI is platform-admin-only; external requests operate only within explicit open-app user and capability grants
- related page headings: `Page: /settings/openapi - OpenAPI Management`, `Page: /open-api/v1/* - Signed Agent Chat Integration`
- related business rule headings: `Rule: Platform Integrations And External Access - Open-App Key Lifecycle`, `Rule: Platform Integrations And External Access - Signed Request Verification`, `Rule: Platform Integrations And External Access - Acting User Authorization`, `Rule: Platform Integrations And External Access - Capability Authorization`, `Rule: Platform Integrations And External Access - Agent Chat Route Mapping`
- related exception headings: `Exception: Platform Integrations And External Access - Invalid Or Replayed Signature`, `Exception: Platform Integrations And External Access - Unauthorized Acting User Or Operation`, `Exception: Platform Integrations And External Access - Rotated Or Disabled Open-App`

== Page Prototypes And Interactions
=== Page: /login - Operator Login
- entry: unauthenticated shell access and provider callback return paths
- layout: centered login card with internal credentials and available provider actions
- data display: auth provider options, field validation, and post-auth redirect context
- actions: sign in, use external provider, recover from denied space access, and sign out
- states: anonymous, loading, provider callback, authenticated, no-space blocked, and auth failure

==== Interaction: /login - Sign In
- trigger control: submit button on the login form
- preconditions: operator account exists and is active
- button or control behavior: submits credentials or provider callback state to the backend auth contract and loads the protected shell on success
- form rules: username or email and password are required for internal login
- confirmation behavior: none
- success feedback: shell loads with the authenticated operator session
- failure feedback: invalid credentials or denied access show inline and toast-level feedback without leaving the page
- navigation or refresh behavior: successful login redirects to the intended protected route or first valid landing surface

=== Page: /settings/openapi - OpenAPI Management
- entry: settings shell navigation for platform configuration
- layout: settings shell with an OpenAPI navigation item, open-app catalog table, and detail workspace for overview, user grants, and API grants
- data display: open-app name, openAppCode, status, key fingerprint or last4, granted-user count, granted-capability count, and last-rotated timestamp
- actions: create open-app, edit metadata, enable or disable, rotate key, add or remove user grants, and update capability-operation grants
- states: empty catalog, loading, populated list, one-time key reveal, grant editing, disabled open-app, and API failure feedback

==== Interaction: /settings/openapi - Create Open-App
- trigger control: primary create button in the OpenAPI catalog
- preconditions: current operator is a platform admin
- button or control behavior: opens a create form, validates unique naming and description inputs, submits to the platform open-app create API, and navigates to the saved detail view
- form rules: openAppCode is backend-generated; name is required; description is optional; new open-app starts active with no user grants and no API grants
- confirmation behavior: after successful create, show a blocking one-time secret reveal modal
- success feedback: created open-app appears in the catalog and detail view, with the generated appKey shown once
- failure feedback: inline validation and toast feedback use the backend error envelope
- navigation or refresh behavior: detail view stays selected after create and later refreshes show only fingerprinted key information

==== Interaction: /settings/openapi - Rotate Open-App Key
- trigger control: rotate key action in the open-app overview panel
- preconditions: current operator is a platform admin and the target open-app remains active
- button or control behavior: asks for confirmation, calls rotate-key, replaces the active credential, and presents the new appKey once
- form rules: no second active key is kept; old key is revoked immediately after rotation succeeds
- confirmation behavior: explicit irreversible confirmation before rotation executes
- success feedback: one-time key reveal modal shows the new appKey and the overview refreshes the fingerprint and rotated timestamp
- failure feedback: current key stays valid when rotation fails and the operator sees a recoverable error
- navigation or refresh behavior: detail page remains open and later reloads never show plaintext key material

==== Interaction: /settings/openapi - Manage User And API Grants
- trigger control: user grant add or remove actions and API grant toggles on the detail page
- preconditions: current operator is a platform admin
- button or control behavior: persists explicit userCode grants and capability-operation grants through dedicated platform APIs
- form rules: only concrete userCode rows are supported in this version; API grants are stored by business capability plus operation, not by raw path
- confirmation behavior: destructive removals require confirmation; additive changes may save inline
- success feedback: detail tables and toggles refresh in place with toast confirmation
- failure feedback: backend denial or validation errors surface without leaving the page
- navigation or refresh behavior: detail state remains on the active open-app after each mutation

=== Page: /open-api/v1/* - Signed Agent Chat Integration
- entry: external integration caller invokes agent-chat public routes
- layout: no browser page; this is an API entry surface
- data display: uniform Tron response envelope for success or error, plus SSE stream frames on streaming routes
- actions: create session, read session groups or messages, send message, consume stream, and read AGUI replay endpoints according to granted operations
- states: authorized execution, denied acting user, denied capability, invalid signature, replayed nonce, stale timestamp, rotated key, and disabled open-app

==== Interaction: /open-api/v1/* - Send Signed Request
- trigger control: external caller signs and sends an HTTP request
- preconditions: open-app is active, request carries required headers, acting user grant exists, capability grant exists, and route maps to a supported operation
- button or control behavior: middleware canonicalizes the request, verifies HMAC, enforces time and nonce rules, resolves acting user, and either rejects early or forwards to the target handler
- form rules: request must include `X-Tron-App-Code`, `X-Tron-User-Code`, `X-Tron-Timestamp`, `X-Tron-Nonce`, `X-Tron-Signature`, and `X-Tron-Signature-Version`
- confirmation behavior: none
- success feedback: route returns its normal open-api success envelope or stream result
- failure feedback: route returns a uniform auth or authorization error envelope without exposing secret material
- navigation or refresh behavior: not applicable

== Business Rules
=== Rule: Console Access And Identity - Protected Session Contract
- condition: any non-auth management route is requested
- decision or calculation: request must resolve to a valid authenticated session and may refresh access state only through the backend-owned session contract
- state transition: authenticated operator enters the protected shell; unauthenticated or unauthorized operator is redirected to the appropriate auth or no-space surface
- permission or data boundary: protected routes must not rely on stale browser state as the source of truth
- conflict or priority rule: platform-admin bootstrap access wins over no-space blocking for first deployment ownership
- example: a user without workspace space assignments is stopped on the authorization-request page instead of entering the shell

=== Rule: Platform Integrations And External Access - Open-App Key Lifecycle
- condition: platform admin creates or rotates an open-app credential
- decision or calculation: system generates one active appKey, stores sealed or encrypted secret material plus fingerprint metadata, and returns plaintext only in the create or rotate response
- state transition: previous active credential becomes revoked immediately after a successful rotation; the new credential becomes the sole active credential
- permission or data boundary: plaintext appKey is never returned by list or detail read APIs
- conflict or priority rule: if rotation fails, the current credential remains active
- example: an admin rotates an open-app key, copies the new key from the modal, and all later detail reads show only fingerprint fields

=== Rule: Platform Integrations And External Access - Signed Request Verification
- condition: any request enters `/open-api/v1/*`
- decision or calculation: compute the canonical request string from method, normalized path, normalized query, body hash, openAppCode, userCode, timestamp, and nonce, then verify `TRON-HMAC-SHA256-V1` with constant-time comparison
- state transition: valid request proceeds to authorization checks; invalid request stops before handler execution
- permission or data boundary: bearer-token fallback is not allowed for formal external open-api traffic
- conflict or priority rule: stale timestamp, replayed nonce, or invalid signature each reject the request even if the acting user would otherwise be valid
- example: a replayed signed request with the same nonce is rejected even when the signature itself matches

=== Rule: Platform Integrations And External Access - Acting User Authorization
- condition: a signed request passes signature verification
- decision or calculation: the target `X-Tron-User-Code` must map to an active internal user and must appear in the explicit open-app user grant table
- state transition: authorized acting user becomes the request auth context; unauthorized acting user yields a forbidden response
- permission or data boundary: this version supports only explicit userCode grants, not role-wide grants
- conflict or priority rule: inactive users are rejected before capability authorization
- example: a signed request for `alex.johnson` fails when the open-app has grants for `sam.li` only

=== Rule: Platform Integrations And External Access - Capability Authorization
- condition: acting user authorization succeeds
- decision or calculation: route metadata maps the request to one business capability and one operation, and the open-app must hold a matching grant row
- state transition: granted operation reaches the handler; missing grant returns a forbidden response
- permission or data boundary: grants are stored and evaluated by business capability plus operation, not by raw URL wildcard
- conflict or priority rule: route mapping is authoritative; handler code must not silently widen access later
- example: an open-app granted `agent-chat / session.read` can list session groups but cannot send a message

=== Rule: Platform Integrations And External Access - Agent Chat Route Mapping
- condition: the current public agent-chat open-api routes are registered
- decision or calculation: `GET /open-api/v1/chats/session-groups` maps to `agent-chat / session.read`; `POST /open-api/v1/chats/sessions` maps to `agent-chat / session.write`; `PATCH /open-api/v1/chats/sessions/{sessionCode}` maps to `agent-chat / session.write`; `GET /open-api/v1/chats/sessions/{sessionCode}/messages` maps to `agent-chat / message.read`; `POST /open-api/v1/chats/sessions/{sessionCode}/messages` maps to `agent-chat / message.write`; `POST /open-api/v1/chats/sessions/{sessionCode}/messages/stream` maps to `agent-chat / stream.consume`; AGUI replay reads map to `agent-chat / session.read`
- state transition: each request carries a stable operation identity into authorization
- permission or data boundary: future public capabilities must add explicit mappings before exposure
- conflict or priority rule: unmapped routes are not externally callable
- example: AGUI protocol replay remains readable only when the open-app has the same read grant that covers session inspection

== Exceptions And Boundaries
=== Exception: Console Access And Identity - No Space Access
- scenario: authenticated user has no assigned workspace spaces and is not the bootstrap platform admin
- expected product behavior: block shell navigation and route the user to the authorization-request experience
- user-visible message or fallback: explain that access must be granted by a responsible owner
- data consistency expectation: no protected route mutation occurs
- retry, recovery, or compensation behavior: user retries after access is granted
- logging/audit expectation: record the denied protected-route attempt

=== Exception: Platform Integrations And External Access - Invalid Or Replayed Signature
- scenario: request omits required headers, uses an unsupported signature version, has an invalid signature, exceeds timestamp skew, or repeats a nonce
- expected product behavior: reject before handler execution with a uniform unauthorized error envelope
- user-visible message or fallback: return a concise authentication failure code and message without leaking secret state
- data consistency expectation: no chat session, message, or grant mutation occurs
- retry, recovery, or compensation behavior: caller may retry with a fresh timestamp and nonce after correcting the signature
- logging/audit expectation: record enough auth-failure context for audit without persisting plaintext secrets

=== Exception: Platform Integrations And External Access - Unauthorized Acting User Or Operation
- scenario: signature is valid but the requested userCode is inactive, ungranted, or the capability-operation grant is missing
- expected product behavior: reject with a forbidden response after auth checks and before handler business logic executes
- user-visible message or fallback: return a denial code that distinguishes auth from authorization
- data consistency expectation: target business state stays unchanged
- retry, recovery, or compensation behavior: admin must add the missing grant or caller must switch to an already granted user or operation
- logging/audit expectation: record open-app, userCode, and denied capability-operation context

=== Exception: Platform Integrations And External Access - Rotated Or Disabled Open-App
- scenario: caller uses an old key after rotation or the platform admin disables the open-app
- expected product behavior: reject all signed requests for that credential or open-app immediately
- user-visible message or fallback: return an auth failure that requires credential refresh or admin intervention
- data consistency expectation: no partial business mutation is applied
- retry, recovery, or compensation behavior: caller must fetch the newly revealed key from its own secure storage or wait for admin re-enablement
- logging/audit expectation: record disabled or revoked credential usage attempts

== Shared Requirements
- Authenticated session model: shared because the entire console depends on one operator identity and protected routing; consumers: Features 1, 2, 3, 4, 5, 6, 7; decision: build first
- Workspace context and inheritance rules: shared because registries, delivery flows, and data surfaces resolve from the selected workspace and lineage; consumers: Features 2, 3, 4, 5, 6; decision: build first
- Stable user and role directory: shared because auth, workspace grants, task assignees, and external acting-user authorization use the same canonical records; consumers: Features 1, 2, 4, 7; decision: build first
- Search, pagination, and operator feedback patterns: shared because nearly every management surface needs consistent filtering, paging, loading, and toast feedback; consumers: Features 2, 3, 4, 5, 6, 7; decision: build first
- Locale-aware UI and runtime formatting: shared because navigation, forms, tables, empty states, and counts must switch locale coherently; consumers: Features 1, 2, 3, 4, 5, 6, 7; decision: build first
- Audit and activity ledger: shared because admin, workflow, automation, and external-access mutations eventually need a common compliance trail; consumers: Features 2, 4, 5, 6, 7; decision: defer

== Feature Inventory
1. Console Access and Identity: module Console Access And Identity; source heading `Feature: Console Access and Identity`; pages `/login`; rules protected session contract; exceptions no-space access; goal establish secure operator access and protected shell bootstrap; in scope login, OAuth callback, current-session bootstrap, logout, bearer-token protected APIs, refresh-cookie session renewal, bootstrap platform admin initialization, and no-space authorization blocking; out of scope signup, MFA, invitation acceptance, external IdP admin UI; depends on stable user and role directory
2. Administration and Workspace Governance: module Administration And Workspace Governance; source heading existing admin and workspace governance requirements; pages admin users, roles, workspace management, workspace switcher; rules workspace scope and access grant rules; exceptions archived or denied workspace mutations; goal let admins manage people, roles, and hierarchical workspaces from one control plane; in scope user directory, create-detail flows, role matrix catalog, workspace tree CRUD, workspace access grants, effective-user inspection, and workspace switcher projections; out of scope billing, per-workspace custom role systems, advanced compliance policy; depends on Feature 1
3. AI Asset Registry: module AI Asset Registry; source heading existing AI asset registry requirements; pages settings models, variables, skills, prompts, resources; rules inheritance-aware source resolution; exceptions missing inheritance sources and verify failures; goal let each workspace control effective AI configuration without duplicating everything locally; in scope platform and workspace model management, prompt registry, skill registry and bind-install flows, variables, resources, and inheritance-aware source states; out of scope marketplace sync, external package governance, and automatic runtime synthesis; depends on Feature 2
4. Workspace Delivery Workflow: module Workspace Delivery Workflow; source heading existing delivery workflow requirements; pages app registry, task kanban, task detail; rules assignment validation and role-aware status flow; exceptions invalid assignee and archived app/task mutations; goal support practical planning and delivery inside the selected workspace; in scope app registry CRUD, task kanban, task detail, parent-child tasks, assignee validation, app-to-task binding, and role-aware status flows; out of scope external import, execution engine orchestration, and strict downstream runtime enforcement beyond current registry validation; depends on Feature 2
5. Scheduled Automations: module Scheduled Automations; source heading existing automation requirements; pages automation list, detail, execution history; rules active or paused lifecycle and run-now behavior; exceptions scheduler failure and paused execution boundaries; goal enable repeatable workspace-scoped prompt execution with clear lifecycle control; in scope automation CRUD-like flows, active or paused lifecycle, run-now, execution history, in-process scheduling, and latest-result rollups; out of scope DAGs, webhook triggers, distributed scheduler ownership, and external scheduler sync; depends on Feature 2
6. Knowledge, Memory, and Observability: module Knowledge, Memory, And Observability; source heading existing knowledge and observability requirements; pages memory, graph, resources, dashboards, history; rules workspace-scoped data visibility; exceptions upstream runtime gaps and stale projections; goal provide the data and visibility plane around agent operations; in scope memory management, knowledge graph-base surfaces, resources and observability views, runtime usage dashboards, and operational context browsing; out of scope federated analytics, automated remediation, and external knowledge-market integration; depends on Feature 2 and Feature 4
7. Platform OpenAPI and External Access: module Platform Integrations And External Access; source headings `Feature: Platform Integrations And External Access - Open-App Registry And Signed OpenAPI Access`, `Page: /settings/openapi - OpenAPI Management`, `Rule: Platform Integrations And External Access - Signed Request Verification`; pages `/settings/openapi`, `/open-api/v1/*`; rules open-app key lifecycle, signed request verification, acting user authorization, capability authorization, and agent-chat route mapping; exceptions invalid or replayed signature, unauthorized acting user or operation, rotated or disabled open-app; goal let platform admins govern signed public API access without exposing internal bearer sessions; in scope platform open-app registry, one-time appKey lifecycle, explicit acting-user grants, capability-operation grants, signed agent-chat public access, nonce replay protection, and bearer removal from public open-api auth; out of scope OAuth2 client credentials, role-wide grants, dual-key overlap windows, and additional public capability families beyond agent-chat; depends on Feature 2

== Dependency Map
- Feature 2 depends on Feature 1
- Feature 3 depends on Feature 2
- Feature 4 depends on Feature 2
- Feature 4 depends on shared requirement: stable user and role directory
- Feature 5 depends on Feature 2
- Feature 6 depends on Feature 2
- Feature 6 depends on Feature 4
- Feature 6 depends on shared requirement: audit and activity ledger
- Feature 7 depends on Feature 2
- Feature 7 depends on shared requirement: stable user and role directory
- Feature 7 depends on shared requirement: audit and activity ledger

#mermaid(
  "flowchart LR
    SR1[\"Shared Requirement: Authenticated Session Model\"] --> F1[\"Feature 1: Console Access and Identity\"]
    F1 --> F2[\"Feature 2: Administration and Workspace Governance\"]
    F2 --> F3[\"Feature 3: AI Asset Registry\"]
    F2 --> F4[\"Feature 4: Workspace Delivery Workflow\"]
    F2 --> F5[\"Feature 5: Scheduled Automations\"]
    F2 --> F6[\"Feature 6: Knowledge, Memory, and Observability\"]
    F2 --> F7[\"Feature 7: Platform OpenAPI and External Access\"]
    SR2[\"Shared Requirement: Stable User and Role Directory\"] --> F4
    SR2 --> F7
    F4 --> F6
    SR3[\"Shared Requirement: Audit and Activity Ledger\"] --> F6
    SR3 --> F7"
)

== Recommended Implementation Order
1. Stable user and role directory plus authenticated session model - this is the minimum foundation for any protected console behavior, acting-user validation, and assignment-driven workflow.
2. Workspace governance and workspace switcher projections - this establishes the tenancy boundary every other feature resolves against and keeps platform-admin surfaces coherent.
3. AI asset registry - models, prompts, skills, variables, and resources become reusable building blocks for downstream workflow surfaces and establish strong settings-shell patterns.
4. Platform OpenAPI and external access - signed public agent-chat access should build on auth, admin, and shared settings patterns, and should be delivered in three bounded slices: HMAC auth foundation first, open-app management second, and public agent-chat route migration third.
5. Workspace delivery workflow - apps and tasks convert the governance foundation into operator execution planning and assignment loops.
6. Scheduled automations - scheduler-backed prompt workflows become valuable once workspace context and core operator patterns are stable.
7. Knowledge, memory, observability, and deferred audit deepening - these surfaces deliver the highest value after real workflows are already generating operational data.

== Acceptance
=== Acceptance: Console Access And Identity
- Protected routes require an authenticated session and keep shell bootstrap stable through refresh, logout, and space-authorization boundaries.

=== Acceptance: Platform Integrations And External Access - Open-App Registry And Signed OpenAPI Access
- Platform admins can manage open-app records from `/settings/openapi`, including create, update, disable, rotate key, user grants, and API grants.
- Create and rotate responses reveal plaintext appKey once, while later read APIs expose fingerprinted key information only.
- Formal `/open-api/v1/*` traffic accepts only `TRON-HMAC-SHA256-V1` requests with explicit app, user, timestamp, nonce, and signature headers.
- Public agent-chat routes enforce both acting-user grants and capability-operation grants before handler execution.
- Bearer-token fallback is removed from formal public open-api auth.
- Signed requests reject stale timestamps, replayed nonces, invalid signatures, disabled open-apps, revoked keys, unauthorized users, and unauthorized operations with a uniform Tron error envelope.

== Risks And Open Questions
- Chat-session orchestration and Main Agent behavior exist in navigation and page surfaces, but their stable backend product contract is less explicit than admin, task, model, and automation modules; the external-access feature raises the importance of making that contract explicit.
- Audit-log expectations appear in design materials, but a shared cross-feature audit contract is still weaker than the other admin foundations.
- Production localization requires `zh-CN` alongside `en-US`, while the current codebase still centers delivery around `en-US` resources.
- The fixed three-level workspace hierarchy may be too shallow for some enterprise org structures; validate before broader rollout.
- Several data domains still support mixed mock and API modes in development; confirm the launch bar for calling a module production-ready.
- Current secret handling patterns for platform variables and model API keys are insufficient for signed external credentials and must not be reused as-is.
