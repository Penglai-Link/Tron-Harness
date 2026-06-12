= PRD: Variable Sensitive Data

#import "@preview/mmdr:0.2.2": mermaid

== Summary
- Product: Tron variable management.
- Problem: workspace and platform variables can contain API keys, tokens, and credentials, but current storage, APIs, runtime context, and UI treat every value as plaintext.
- Target users: platform admins who manage global variables, workspace operators who manage effective workspace variables, and backend runtimes that need controlled access to real values during execution.
- Outcome: users can mark variables as sensitive; sensitive values are encrypted at rest, masked as `********` by default, revealed only after token-based second verification, and resolved for runtime through a dedicated backend path with fail-closed external delivery.

== Background
- Why now: task 636177 asks variable management to support sensitive data, including encrypted storage, encrypted display behavior, and second verification before page reveal.
- Current pain: variables are persisted in plaintext, list APIs return values directly, frontend pages render and search values directly, and agent invocation uses the same plaintext list path as UI display.
- Business context: this feature must protect secrets without breaking existing non-sensitive variable management or workspace inheritance.

== Goals
- Add a sensitive option to platform and workspace variables.
- Encrypt sensitive values at rest with Rust national crypto.
- Mask sensitive values as `********` in normal lists, details, edit forms, and search results.
- Require server-side second verification before revealing one sensitive value.
- Split display reads from runtime plaintext reads.
- Preserve existing non-sensitive variables and legacy data compatibility.

== Scope
- In scope: sensitive flag, SM4-GCM encrypted storage, masked display, reveal after password step-up token, edit preserve or replace behavior, platform and workspace variable coverage, inheritance sensitivity rules, runtime secret resolution, frontend mocks, and focused verification.
- Out of scope: automatic encryption of every existing plaintext variable, OAuth provider re-auth, full variable API route-style migration, external key-management integration, agent/tool variable allowlists, and a broad secret-reference redesign for all runtimes.

== Feature Requirements
=== Module: Variable Management
==== Feature: Variable Management - Sensitive Storage
- name: Sensitive Storage
- description: Persist sensitive platform and workspace variable values as encrypted payloads instead of plaintext.
- user or role: backend service, platform admin, workspace operator.
- entry point: variable create and update flows.
- trigger: user creates a variable with `isSensitive=true`, marks an existing variable sensitive, or replaces a sensitive value.
- preconditions: encryption config contains an active SM4 key and key version.
- main flow: validate request -> generate nonce -> encrypt value with SM4-GCM -> store ciphertext, nonce, tag, algorithm, key version, and sensitive flag -> return masked display data.
- inputs and outputs: inputs are variable key, value, description, scope, and `isSensitive`; output is a variable record that never exposes sensitive plaintext through normal display APIs.
- permissions or visibility: same variable management permissions as current create/update flows, with stronger secret handling.
- related page headings: `Page: /variables - Workspace Variables`, `Page: /settings/variables - Platform Variables`, `Page: Variable Form - Sensitive Value Input`
- related business rule headings: `Rule: Variable Management - SM4-GCM Storage Encryption`, `Rule: Variable Management - Sensitive Edit Semantics`, `Rule: Variable Management - Inherited Sensitivity`
- related exception headings: `Exception: Variable Management - Encryption Key Missing`, `Exception: Variable Management - Decrypt Failed`, `Exception: Variable Management - Legacy Plaintext Row`

==== Feature: Variable Management - Masked Display
- name: Masked Display
- description: Show sensitive values as `********` by default across workspace and platform variable pages.
- user or role: platform admin and workspace operator.
- entry point: `/variables`, `/settings/variables`, platform variable detail page, platform variable edit page, and workspace variable detail panel.
- trigger: user opens a variable list, detail view, or edit view.
- preconditions: variable data can be loaded for the selected platform or workspace scope.
- main flow: backend returns masked sensitive records -> frontend renders mask and sensitive indicator -> search excludes sensitive plaintext -> non-sensitive rows keep current visible-value behavior.
- inputs and outputs: input is current variable scope and search text; output is display-safe records with `value`, `isSensitive`, `valueMasked`, and `hasValue`.
- permissions or visibility: users who can view variable management can see metadata and masks, not sensitive plaintext.
- related page headings: `Page: /variables - Workspace Variables`, `Page: /settings/variables - Platform Variables`
- related business rule headings: `Rule: Variable Management - Masked By Default`, `Rule: Variable Management - Search Does Not Leak Secrets`
- related exception headings: `Exception: Variable Management - Unauthorized Variable View`

==== Feature: Variable Management - Reveal After Step-Up
- name: Reveal After Step-Up
- description: Let an authorized user reveal one sensitive value only after a second password verification.
- user or role: platform admin or workspace operator with access to the variable.
- entry point: reveal action on one sensitive variable row or detail panel.
- trigger: user chooses to reveal a masked sensitive value.
- preconditions: user has an active session, the variable is sensitive, and the account supports password verification.
- main flow: open verification dialog -> submit password -> backend returns short-lived step-up token -> reveal endpoint verifies token and scope -> backend decrypts one value -> frontend displays the plaintext temporarily -> user hides it or navigates away.
- inputs and outputs: inputs are current password, variable scope, variable key, and later `stepUpToken`; output is one plaintext value for the requested variable only.
- permissions or visibility: reveal is scoped to the current user and variable permission; the step-up proof is short-lived.
- related page headings: `Page: Reveal Verification Dialog - Sensitive Variable Reveal`
- related business rule headings: `Rule: Variable Management - Step-Up Reveal`, `Rule: Variable Management - Plaintext Lifetime`
- related exception headings: `Exception: Variable Management - Step-Up Failed`, `Exception: Variable Management - Step-Up Expired`, `Exception: Variable Management - OAuth Account Cannot Step Up`

==== Feature: Variable Management - Runtime Secret Resolution
- name: Runtime Secret Resolution
- description: Provide a backend-only path that resolves real variable values for runtime execution without reusing display APIs.
- user or role: backend runtime and agent invocation service.
- entry point: agent invocation context preparation.
- trigger: an agent run is prepared for a workspace.
- preconditions: workspace is valid and runtime trust policy is known.
- main flow: invocation requests runtime variables -> backend resolves inherited variables -> backend decrypts sensitive values only for trusted runtime delivery -> in-process runtime receives plaintext -> untrusted service-mode runtime fails preparation before any sensitive payload is sent.
- inputs and outputs: input is workspace code and target runtime policy; output is a runtime variable set that can contain plaintext only inside an approved backend runtime boundary.
- permissions or visibility: no human-facing page; display APIs cannot call this path.
- related page headings: `Page: Backend Runtime - Variable Secret Resolution`
- related business rule headings: `Rule: Variable Management - Runtime Read Split`, `Rule: Variable Management - Runtime Trust Boundary`
- related exception headings: `Exception: Variable Management - Untrusted Runtime Secret Delivery`, `Exception: Variable Management - Runtime Decrypt Failed`

==== Feature: Variable Management - Compatibility And Migration
- name: Compatibility And Migration
- description: Add sensitive metadata without breaking existing non-sensitive variables or legacy SQLite rows.
- user or role: backend service, platform admin, workspace operator.
- entry point: database migration, variable list, create, update, and delete flows.
- trigger: backend starts after migration or users manage pre-existing variables.
- preconditions: existing database can be migrated with default values.
- main flow: add nullable/defaulted sensitive columns -> mark existing rows as non-sensitive -> continue serving existing values as before -> encrypt only rows that users mark sensitive.
- inputs and outputs: input is existing variable tables; output is a schema that supports sensitive and non-sensitive values side by side.
- permissions or visibility: no new user-facing page; behavior is visible through existing management pages.
- related page headings: `Page: /variables - Workspace Variables`, `Page: /settings/variables - Platform Variables`
- related business rule headings: `Rule: Variable Management - Legacy Compatibility`
- related exception headings: `Exception: Variable Management - Migration Failure`

== Page Prototypes And Interactions
=== Page: /variables - Workspace Variables
- entry: workspace console route `/variables`.
- layout: existing split-pane variable management page.
- data display: effective variables with key, description, source, inherited labels, update time, sensitive indicator, and value column.
- actions: create local variable, edit local variable, delete local variable, reveal sensitive value, hide revealed value.
- states: loading, empty, inherited row, local row, masked sensitive row, revealed row, reveal verification pending, reveal failure, and decrypt failure.

==== Interaction: /variables - Reveal Sensitive Workspace Value
- trigger control: reveal action on a masked sensitive row or detail panel.
- preconditions: selected variable is sensitive and visible to the current user.
- button or control behavior: open the reveal verification dialog and submit a scoped reveal request after password verification.
- form rules: password is required; variable key and workspace code come from current state and are not user-editable.
- confirmation behavior: no destructive confirmation.
- success feedback: selected value is shown temporarily and the row keeps its sensitive indicator.
- failure feedback: show a scoped error and keep `********` visible.
- navigation or refresh behavior: close, workspace switch, route change, or reload hides the value again.

=== Page: /settings/variables - Platform Variables
- entry: settings route `/settings/variables`.
- layout: existing settings-shell platform variable table plus create, detail, and edit pages.
- data display: platform variables with key, description, update time, sensitive indicator, and masked or visible value.
- actions: create platform variable, edit platform variable, delete platform variable, reveal sensitive value, hide revealed value.
- states: loading, empty, masked sensitive row, revealed row, edit preserving stored secret, edit replacing stored secret, reveal verification pending, and reveal failure.

==== Interaction: /settings/variables - Reveal Sensitive Platform Value
- trigger control: reveal action on a platform sensitive variable row or detail page.
- preconditions: current user can access platform variable management and the row is sensitive.
- button or control behavior: open the reveal verification dialog and submit a platform-scoped reveal request after password verification.
- form rules: password is required; variable key comes from selected row and is not user-editable.
- confirmation behavior: no destructive confirmation.
- success feedback: selected value is shown temporarily.
- failure feedback: show a scoped error and keep `********` visible.
- navigation or refresh behavior: close, route change, or reload hides the value again.

=== Page: Variable Form - Sensitive Value Input
- entry: workspace variable create/edit UI and platform variable create/edit UI.
- layout: existing form layout with a sensitive toggle near the value field.
- data display: key, value input, description, sensitive toggle, and current stored-value state for edits.
- actions: save new variable, save edit, preserve stored sensitive value, replace stored sensitive value.
- states: new non-sensitive, new sensitive, edit non-sensitive, edit sensitive with preserved value, edit sensitive with replacement, and validation error.

==== Interaction: Variable Form - Save Sensitive Variable
- trigger control: `Save`.
- preconditions: key and description rules pass; value is present for create or replacement.
- button or control behavior: submit value and `isSensitive=true`.
- form rules: new sensitive variables require a value; editing a sensitive value may preserve the stored secret or replace it with a new value.
- confirmation behavior: no extra confirmation for create or replace.
- success feedback: form closes or navigates using existing page behavior and the saved value displays as `********`.
- failure feedback: show validation or encryption failure without keeping plaintext in shared page state.
- navigation or refresh behavior: return to the appropriate list or detail page and reload masked data.

=== Page: Reveal Verification Dialog - Sensitive Variable Reveal
- entry: opened from a sensitive variable reveal action.
- layout: modal dialog with password field, reveal action, cancel action, and scoped target metadata.
- data display: variable key, scope label, masked value, and verification state.
- actions: submit password, cancel, hide revealed value.
- states: idle, verifying, verified, failed verification, expired verification, unsupported account, and rate-limited.

=== Page: Backend Runtime - Variable Secret Resolution
- entry: internal agent invocation preparation.
- layout: no user-facing page.
- data display: not applicable.
- actions: resolve display-safe variables for UI or plaintext variables for trusted runtime delivery.
- states: trusted in-process runtime, explicitly trusted external runtime, untrusted external runtime preparation failure, decrypt failure, and configuration failure.

== Business Rules
=== Rule: Variable Management - SM4-GCM Storage Encryption
- condition: a variable is saved with `isSensitive=true`.
- decision or calculation: encrypt its value with SM4-GCM using `gmcrypto-core` with `sm4-aead`; store ciphertext, nonce, tag, algorithm, and key version.
- state transition: plaintext input becomes encrypted storage payload; normal display output becomes `********`.
- permission or data boundary: encryption and decryption occur only on the backend.
- conflict or priority rule: missing encryption config blocks sensitive writes and reveal.
- example: `OPENAI_API_KEY` is saved as sensitive and the database row does not contain the original API-key plaintext.

=== Rule: Variable Management - Masked By Default
- condition: a sensitive variable is returned through a list, detail, edit, or mock display path.
- decision or calculation: return `value = ********`, `isSensitive = true`, `valueMasked = true`, and `hasValue = true`.
- state transition: none.
- permission or data boundary: human display reads never receive sensitive plaintext.
- conflict or priority rule: reveal must use a separate scoped operation.
- example: the workspace variable table shows `********` for a sensitive inherited platform API key.

=== Rule: Variable Management - Search Does Not Leak Secrets
- condition: user searches variable management.
- decision or calculation: search can match key, description, source labels, workspace names, and non-sensitive values; it must not match sensitive plaintext.
- state transition: none.
- permission or data boundary: frontend and backend search logic cannot depend on decrypted sensitive values.
- conflict or priority rule: a secret substring typed into search must not expose the matching sensitive row solely because of its hidden value.
- example: searching `sk-` does not reveal that an API key row contains that prefix.

=== Rule: Variable Management - Sensitive Edit Semantics
- condition: user edits a sensitive variable or changes sensitivity.
- decision or calculation: preserve stored encrypted value when no replacement is supplied; encrypt a replacement value; require a new value when changing sensitive to non-sensitive.
- state transition: preserve encrypted payload, replace encrypted payload, or reject unsafe demotion.
- permission or data boundary: edit forms do not prefill sensitive plaintext.
- conflict or priority rule: the backend must not decrypt a stored secret and write it back as plaintext automatically.
- example: editing only the description of a sensitive variable keeps the old encrypted value.

=== Rule: Variable Management - Inherited Sensitivity
- condition: a workspace effective row inherits or shadows a sensitive variable key.
- decision or calculation: inherited sensitive rows remain sensitive; a workspace override of a sensitive inherited key must also be sensitive.
- state transition: inherited secret remains masked; local override cannot downgrade the key to plaintext.
- permission or data boundary: child workspaces cannot weaken parent/platform secret handling for the same key.
- conflict or priority rule: local value precedence does not override the sensitivity requirement for inherited secret keys.
- example: platform `OPENAI_API_KEY` is sensitive, so a workspace override for `OPENAI_API_KEY` must also be stored as sensitive.

=== Rule: Variable Management - Step-Up Reveal
- condition: user requests reveal for one sensitive variable.
- decision or calculation: backend verifies the current user's password without changing the login session, returns a short-lived step-up token, and reveal decrypts one value only when that token matches the current user and scope.
- state transition: masked display can temporarily become revealed in the current component only.
- permission or data boundary: step-up proof is short-lived and bound to the user.
- conflict or priority rule: normal session authentication alone is not enough to reveal plaintext.
- example: a user opens a masked value, enters the account password, and sees only that selected value.

=== Rule: Variable Management - Plaintext Lifetime
- condition: a value is revealed in the browser.
- decision or calculation: frontend stores plaintext only in local reveal state and removes it on hide, close, navigation, workspace switch, or refresh.
- state transition: revealed -> masked.
- permission or data boundary: plaintext is not placed in global stores, URL params, logs, or reusable mock fixtures.
- conflict or priority rule: background refresh must restore masked data unless the user reveals again.
- example: after closing the detail panel, reopening the variable shows `********`.

=== Rule: Variable Management - Runtime Read Split
- condition: backend prepares variables for UI or runtime.
- decision or calculation: display reads return masked values; runtime reads use a dedicated resolver that can decrypt.
- state transition: no shared display response is reused for runtime plaintext.
- permission or data boundary: only backend runtime preparation can request plaintext resolution.
- conflict or priority rule: changing list display to mask values must not break non-secret runtime resolution.
- example: `/variables` shows `********`, while trusted backend runtime preparation can resolve the real key.

=== Rule: Variable Management - Runtime Trust Boundary
- condition: a target runtime may receive variables.
- decision or calculation: in-process backend runtime can receive sensitive plaintext; external service-mode runtime must be explicitly trusted before sensitive plaintext is included.
- state transition: trusted runtime receives allowed plaintext; untrusted service-mode runtime preparation fails when sensitive values would be required.
- permission or data boundary: secrets must not be sent to arbitrary external runtime endpoints by default.
- conflict or priority rule: encrypted storage is not a protection after the backend transmits plaintext outside its trust boundary, and the backend must not send `********` as a runtime value.
- example: `service_mode_runtime` needs an explicit trusted secret-delivery config before receiving sensitive values in `AgentRunContext`.

=== Rule: Variable Management - Authenticated Record Identity
- condition: sensitive value encryption or decryption runs.
- decision or calculation: use GCM additional authenticated data derived from immutable record identity, such as `tron-variable:v1:platform:OPENAI_API_KEY` or `tron-variable:v1:workspace:TRON_CORE:OPENAI_API_KEY`.
- state transition: none.
- permission or data boundary: ciphertext copied to a different variable key or scope fails authentication.
- conflict or priority rule: renaming a sensitive key or moving it across scopes must decrypt with old AAD and re-encrypt with new AAD; direct identity-column updates are not valid.
- example: ciphertext from workspace `TRON_CORE/OPENAI_API_KEY` cannot decrypt as `TRON_CORE/GITHUB_TOKEN`.

=== Rule: Variable Management - Legacy Compatibility
- condition: migration runs against existing variable rows.
- decision or calculation: existing rows default to `isSensitive=false` and keep their current plaintext values until a user marks them sensitive.
- state transition: schema expands without encrypting legacy values automatically.
- permission or data boundary: legacy values remain visible as non-sensitive unless changed.
- conflict or priority rule: migration must not corrupt existing platform/workspace variable behavior.
- example: a pre-existing `REGION=us-east-1` variable still displays `us-east-1` after migration.

== Exceptions And Boundaries
=== Exception: Variable Management - Encryption Key Missing
- scenario: user tries to create, update, reveal, or runtime-resolve a sensitive variable but no active encryption key is configured.
- expected product behavior: reject the operation with a stable error and keep existing data unchanged.
- user-visible message or fallback: show that sensitive-variable encryption is not configured.
- data consistency expectation: no plaintext secret is stored as a sensitive row.
- retry, recovery, or compensation behavior: configure an active key and retry.
- logging/audit expectation: log the configuration failure without plaintext values.

=== Exception: Variable Management - Decrypt Failed
- scenario: stored ciphertext, nonce, tag, key version, or authenticated context does not verify.
- expected product behavior: fail reveal or runtime preparation for that value and keep the masked display state.
- user-visible message or fallback: show that the sensitive value could not be decrypted.
- data consistency expectation: do not return corrupted or partial plaintext.
- retry, recovery, or compensation behavior: operator can replace the value with a new secret if permitted.
- logging/audit expectation: log record scope, key, algorithm, and key version, but not plaintext.

=== Exception: Variable Management - Legacy Plaintext Row
- scenario: an existing row has no sensitive metadata after migration.
- expected product behavior: treat it as non-sensitive by default.
- user-visible message or fallback: value displays as before until user marks it sensitive.
- data consistency expectation: no automatic encryption or destructive migration occurs.
- retry, recovery, or compensation behavior: user can edit and enable sensitivity to encrypt it.
- logging/audit expectation: no special log unless migration failed.

=== Exception: Variable Management - Step-Up Failed
- scenario: password verification fails for reveal.
- expected product behavior: do not decrypt the value and keep `********` visible.
- user-visible message or fallback: show that verification failed.
- data consistency expectation: no reveal token or plaintext is returned.
- retry, recovery, or compensation behavior: user can retry subject to rate limits.
- logging/audit expectation: record failed reveal verification without password or plaintext.

=== Exception: Variable Management - Step-Up Expired
- scenario: user submits reveal with an expired or mismatched step-up proof.
- expected product behavior: reject reveal and require verification again.
- user-visible message or fallback: show that verification expired.
- data consistency expectation: no plaintext is returned.
- retry, recovery, or compensation behavior: reopen verification and retry.
- logging/audit expectation: record expired proof use without plaintext.

=== Exception: Variable Management - Step-Up Token Mismatch
- scenario: user submits a token for a different user, variable scope, or variable key.
- expected product behavior: reject reveal and keep `********` visible.
- user-visible message or fallback: show that verification is invalid for this reveal.
- data consistency expectation: no plaintext is returned.
- retry, recovery, or compensation behavior: repeat password verification for the selected variable.
- logging/audit expectation: record mismatched proof use without plaintext.

=== Exception: Variable Management - OAuth Account Cannot Step Up
- scenario: current user has no local password that the backend can verify.
- expected product behavior: deny reveal in v1.
- user-visible message or fallback: show that password verification is unavailable for this account.
- data consistency expectation: no plaintext is returned.
- retry, recovery, or compensation behavior: future OAuth re-auth can be designed separately.
- logging/audit expectation: record unsupported step-up method without plaintext.

=== Exception: Variable Management - Unauthorized Variable View
- scenario: user lacks access to the workspace or platform variable scope.
- expected product behavior: reject list, edit, or reveal using the standard response envelope.
- user-visible message or fallback: show an access or unavailable-scope error.
- data consistency expectation: no variable metadata or plaintext is leaked.
- retry, recovery, or compensation behavior: user must switch to an authorized scope.
- logging/audit expectation: log denied access with user and scope identifiers.

=== Exception: Variable Management - Untrusted Runtime Secret Delivery
- scenario: agent invocation targets an external runtime that is not trusted for secret delivery.
- expected product behavior: fail invocation preparation before posting the runtime context when sensitive variables would be included.
- user-visible message or fallback: show a clear runtime configuration error.
- data consistency expectation: sensitive values remain encrypted at rest and absent from external payloads.
- retry, recovery, or compensation behavior: configure the runtime as trusted or use a backend-controlled runtime path.
- logging/audit expectation: log the trust-policy decision without plaintext.

=== Exception: Variable Management - Runtime Decrypt Failed
- scenario: trusted runtime preparation cannot decrypt one sensitive variable.
- expected product behavior: fail the affected invocation preparation instead of sending masked or corrupt values as if they were real secrets.
- user-visible message or fallback: show a stable run-preparation error.
- data consistency expectation: no partial secret payload is sent.
- retry, recovery, or compensation behavior: replace the broken variable or restore the encryption key version.
- logging/audit expectation: log scope and variable key without plaintext.

=== Exception: Variable Management - Migration Failure
- scenario: sensitive metadata migration fails during backend startup or test setup.
- expected product behavior: startup or migration fails clearly instead of running with a half-updated schema.
- user-visible message or fallback: backend health or startup logs report migration failure.
- data consistency expectation: existing variable rows are not partially rewritten.
- retry, recovery, or compensation behavior: fix migration state and rerun.
- logging/audit expectation: include migration name and database error.

== Shared Requirements
- Uniform API envelope: shared because variable list, mutation, step-up, reveal, and runtime errors must return or map through the repo-wide `code`, `message`, and `data` shape; consumers: Sensitive Storage, Masked Display, Reveal After Step-Up; decision: use the current success code `SUCCESS` and keep the standard response envelope.
- Rust national crypto wrapper: shared because platform variables, workspace variables, reveal, and runtime resolution need the same encryption contract; consumers: Sensitive Storage, Reveal After Step-Up, Runtime Secret Resolution; decision: build first behind a small backend cipher abstraction using `gmcrypto-core` `sm4-aead`; pin the exact crate version during implementation spike.
- Step-up verification: shared because both platform and workspace reveal use the same second-verification proof; consumers: Reveal After Step-Up; decision: build once in auth/application code and call from variable reveal endpoints.
- Display/runtime read split: shared because the same variable catalog feeds UI and agent invocation today; consumers: Masked Display, Runtime Secret Resolution; decision: build before masking UI responses.
- Frontend sensitive-value component behavior: shared because workspace and platform pages need the same mask, reveal, hide, and edit-preserve semantics; consumers: Masked Display, Reveal After Step-Up; decision: keep local to variable management unless another domain needs it.

== Interface Contract
=== Rule: Variable Management - Display VO Contract
- condition: a variable record is returned for normal platform or workspace display.
- decision or calculation: expose `value`, `isSensitive`, `valueMasked`, and `hasValue`; never expose ciphertext, nonce, tag, algorithm, or key version in display APIs.
- state transition: none.
- permission or data boundary: display consumers cannot distinguish stored encrypted payload details.
- conflict or priority rule: `value` is `********` for sensitive rows and current plaintext for non-sensitive rows.
- example: `OPENAI_API_KEY` display data has `valueMasked=true` and `value=********`.

=== Rule: Variable Management - Mutation Param Contract
- condition: create or update handles platform or workspace variables.
- decision or calculation: create accepts `isSensitive`; update accepts `isSensitive` and `valueMode` with values `PRESERVE` or `REPLACE`.
- state transition: `PRESERVE` keeps stored payload; `REPLACE` writes a new plaintext input through the correct sensitive or non-sensitive storage path.
- permission or data boundary: update does not decrypt a secret just to preserve or demote it.
- conflict or priority rule: create, `REPLACE`, and sensitive-to-non-sensitive demotion require `value`.
- example: editing only a sensitive variable description sends `valueMode=PRESERVE` and no `value`.

=== Rule: Variable Management - Step-Up API Contract
- condition: user starts reveal from workspace or platform variable management.
- decision or calculation: call `POST /api/v1/auth/verify-step-up-password` with `{ password }`; response data contains `{ stepUpToken, expiresAt }`; do not create or refresh login session state.
- state transition: current user gains a short-lived proof for reveal only.
- permission or data boundary: user identity comes from auth context, not from request body.
- conflict or priority rule: unsupported auth methods return a stable unsupported error.
- example: password verification succeeds and returns a token valid for a short reveal window.

=== Rule: Variable Management - Reveal API Contract
- condition: user submits reveal after step-up.
- decision or calculation: workspace reveal uses `POST /api/v1/variables/reveal-workspace-variable` with `{ workspaceCode, variableKey, stepUpToken }`; platform reveal uses `POST /api/v1/admin/platform/variables/reveal-platform-variable` with `{ variableKey, stepUpToken }`.
- state transition: selected browser component can move from masked to revealed.
- permission or data boundary: reveal response data contains only `{ value }` for the requested variable.
- conflict or priority rule: reveal endpoints are POST, kebab-case, and no path parameters.
- example: revealing workspace `TRON_CORE/OPENAI_API_KEY` returns one plaintext value after token verification.

== Feature Inventory
1. Sensitive Storage: module Variable Management; source headings `Feature: Variable Management - Sensitive Storage`, `Rule: Variable Management - SM4-GCM Storage Encryption`, `Rule: Variable Management - Sensitive Edit Semantics`, `Exception: Variable Management - Encryption Key Missing`, `Exception: Variable Management - Decrypt Failed`; goal encrypt sensitive rows at rest; in scope platform/workspace storage and migration metadata; out of scope enterprise KMS; depends on Rust national crypto wrapper.
2. Masked Display: module Variable Management; source headings `Feature: Variable Management - Masked Display`, `Page: /variables - Workspace Variables`, `Page: /settings/variables - Platform Variables`, `Rule: Variable Management - Masked By Default`, `Rule: Variable Management - Search Does Not Leak Secrets`; goal prevent normal UI/API display leakage; in scope tables, detail, edit, search, mocks; out of scope global audit UI; depends on Sensitive Storage and display/runtime read split.
3. Reveal After Step-Up: module Variable Management; source headings `Feature: Variable Management - Reveal After Step-Up`, `Page: Reveal Verification Dialog - Sensitive Variable Reveal`, `Rule: Variable Management - Step-Up Reveal`, `Exception: Variable Management - Step-Up Failed`; goal reveal one value after password verification; in scope platform/workspace reveal; out of scope OAuth re-auth; depends on Sensitive Storage, Masked Display, and Step-up verification.
4. Runtime Secret Resolution: module Variable Management; source headings `Feature: Variable Management - Runtime Secret Resolution`, `Page: Backend Runtime - Variable Secret Resolution`, `Rule: Variable Management - Runtime Read Split`, `Rule: Variable Management - Runtime Trust Boundary`, `Exception: Variable Management - Untrusted Runtime Secret Delivery`; goal keep agent execution separated from UI display safety; in scope backend resolver and trust boundary; out of scope full secret-reference runtime protocol; depends on Sensitive Storage and display/runtime read split.
5. Compatibility And Migration: module Variable Management; source headings `Feature: Variable Management - Compatibility And Migration`, `Rule: Variable Management - Legacy Compatibility`, `Exception: Variable Management - Migration Failure`; goal upgrade schema without breaking existing non-sensitive rows; in scope default non-sensitive migration; out of scope automatic bulk encryption; depends on Sensitive Storage.

== Dependency Map
#mermaid(
  "flowchart LR
    A[\"Shared: Rust national crypto wrapper\"] --> B[\"Feature: Sensitive Storage\"]
    B --> C[\"Feature: Compatibility And Migration\"]
    B --> D[\"Feature: Masked Display\"]
    D --> E[\"Feature: Reveal After Step-Up\"]
    B --> F[\"Feature: Runtime Secret Resolution\"]
    G[\"Shared: Step-up verification\"] --> E
    H[\"Shared: Display/runtime read split\"] --> D
    H --> F"
)

== Recommended Implementation Order
1. Crypto wrapper and config - required before encrypted writes or decrypt tests can be meaningful.
2. Database migration and repository model - creates the storage contract while keeping legacy rows non-sensitive.
3. Backend display/runtime/reveal service split - prevents UI masking from breaking agent runtime or leaking display plaintext.
4. Step-up and reveal API endpoints - unlocks user-visible reveal after the backend storage contract is safe.
5. Frontend platform/workspace UI updates - add toggles, masks, reveal dialog, edit-preserve behavior, and search updates.
6. Runtime trust policy verification - fail untrusted external runtime preparation when sensitive plaintext would be delivered.
7. Regression and browser proof - prove non-sensitive compatibility, encrypted storage, no UI/API plaintext leakage, and reveal flow.

== Acceptance
=== Acceptance: Variable Management - Sensitive Storage
- Sensitive platform and workspace variables are stored without plaintext values in SQLite and can still be decrypted by backend-controlled flows.
- Existing variables migrate as non-sensitive and keep current behavior.

=== Acceptance: Variable Management - Masked Display
- Workspace and platform variable lists, detail views, edit forms, mocks, and search results show sensitive values as `********` and never expose hidden plaintext through normal display APIs or frontend state.

=== Acceptance: Variable Management - Reveal After Step-Up
- A user can reveal one sensitive value only after successful second verification, and failed, expired, unsupported, or unauthorized verification returns no plaintext.

=== Acceptance: Variable Management - Sensitive Editing
- Editing a sensitive variable can preserve the stored secret or replace it with a newly encrypted value, and sensitive-to-non-sensitive demotion requires a new value.

=== Acceptance: Variable Management - Runtime Secret Resolution
- Agent runtime preparation uses a dedicated resolver for plaintext values, and untrusted external service-mode runtime preparation fails before sensitive plaintext is posted.

=== Acceptance: Variable Management - No Secret Leakage
- Logs, errors, event payloads, tests, browser screenshots, and API responses used for normal display do not include sensitive plaintext.

== Risks And Open Questions
- Production key delivery still needs deployment input. The implementation should support configured key versions and fail closed when no active key exists.
- `service_mode_runtime` is fail-closed in v1 unless explicitly configured as trusted for secret delivery.
- OAuth-only reveal is out of v1. Users without local passwords cannot reveal until OAuth re-auth is designed.
- Existing variable CRUD routes are legacy REST/path-parameter endpoints. New reveal and step-up endpoints should follow the frontend/backend consensus, while full CRUD route migration remains out of scope.
