# Feature Spec: Variable Sensitive Data

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: variable-sensitive-data
- feature_name: Variable Sensitive Data
- status: approved_with_defaults
- owner: baiyihuan
- source_request: task 636177 asks variable management to support sensitive data with an option, encrypted storage, masked display, and second verification before page reveal.
- main_agent: Codex lead agent
- dependency_features: `variable-management`, `auth-session`, `agent-invocation-api`
- dependency_contracts: uniform API response envelope, current authenticated session context, workspace/platform variable CRUD, current agent invocation context preparation, Makefile-first validation

## Requirement

Tron variable management must let platform admins and workspace operators mark variable values as sensitive.

Sensitive values must be encrypted at rest with Rust national crypto, display as `********` in all normal UI and API display flows, and reveal only one selected value after current-user password step-up. Agent runtime preparation must stop using display data for execution values and must fail closed before sending sensitive plaintext to untrusted external service runtimes.

## Product Source Slices

- PRD module and feature slices:
  - `Feature: Variable Management - Sensitive Storage`
  - `Feature: Variable Management - Masked Display`
  - `Feature: Variable Management - Reveal After Step-Up`
  - `Feature: Variable Management - Runtime Secret Resolution`
  - `Feature: Variable Management - Compatibility And Migration`
- PRD page and interaction slices:
  - `Page: /variables - Workspace Variables`
  - `Page: /settings/variables - Platform Variables`
  - `Page: Variable Form - Sensitive Value Input`
  - `Page: Reveal Verification Dialog - Sensitive Variable Reveal`
  - `Page: Backend Runtime - Variable Secret Resolution`
  - `Interaction: /variables - Reveal Sensitive Workspace Value`
  - `Interaction: /settings/variables - Reveal Sensitive Platform Value`
  - `Interaction: Variable Form - Save Sensitive Variable`
- PRD rule and exception slices:
  - `Rule: Variable Management - SM4-GCM Storage Encryption`
  - `Rule: Variable Management - Masked By Default`
  - `Rule: Variable Management - Search Does Not Leak Secrets`
  - `Rule: Variable Management - Sensitive Edit Semantics`
  - `Rule: Variable Management - Inherited Sensitivity`
  - `Rule: Variable Management - Step-Up Reveal`
  - `Rule: Variable Management - Runtime Read Split`
  - `Rule: Variable Management - Runtime Trust Boundary`
  - `Rule: Variable Management - Authenticated Record Identity`
  - `Rule: Variable Management - Display VO Contract`
  - `Rule: Variable Management - Mutation Param Contract`
  - `Rule: Variable Management - Step-Up API Contract`
  - `Rule: Variable Management - Reveal API Contract`
  - `Exception: Variable Management - Encryption Key Missing`
  - `Exception: Variable Management - Decrypt Failed`
  - `Exception: Variable Management - Step-Up Failed`
  - `Exception: Variable Management - Step-Up Token Mismatch`
  - `Exception: Variable Management - OAuth Account Cannot Step Up`
  - `Exception: Variable Management - Untrusted Runtime Secret Delivery`
  - `Exception: Variable Management - Runtime Decrypt Failed`

## Scope Boundary

This workspace owns the v1 sensitive-variable capability for platform and workspace variables: storage metadata, SM4-GCM encryption, display masking, edit semantics, reveal step-up, runtime resolver split, frontend UI, mocks, and focused verification.

## Assumptions

- Existing non-sensitive variables remain plaintext and compatible unless a user marks them sensitive.
- The backend owns encryption and decryption; frontend code never receives ciphertext, nonce, tag, algorithm, or key version.
- The reveal flow uses password step-up for local-password users and denies OAuth-only users in v1.
- `react_runtime` is trusted because it runs in process; `service_mode_runtime` is untrusted by default.
- Key rotation in v1 supports active-version encryption and historical-version decryption, not bulk re-encryption.
- `gmcrypto-core` API use will be confirmed by a small implementation spike before the cipher wrapper is finalized.

## In Scope

- Add `isSensitive`, `valueMasked`, and `hasValue` to variable display contracts.
- Add sensitive metadata columns to workspace and platform variable storage.
- Encrypt sensitive rows with SM4-GCM using a configured active key version.
- Bind encrypted payloads to platform or workspace record identity with GCM AAD.
- Mask sensitive values in list, detail, edit, search, mock, and browser surfaces.
- Preserve or replace sensitive values safely during edit.
- Require a new value for sensitive-to-non-sensitive demotion.
- Add password step-up token and single-value reveal endpoints.
- Split display reads from runtime plaintext resolution.
- Fail untrusted service-mode runtime preparation when sensitive variables would be sent.

## Out Of Scope

- Automatic encryption of every existing plaintext variable.
- OAuth provider re-auth.
- Enterprise KMS integration.
- Full legacy variable CRUD route migration.
- Global secret-reference protocol for every runtime and tool.
- Agent or tool variable allowlists.
- User-facing audit log UI.

## Constraints

- API responses keep the repo-wide `code`, `message`, `data` envelope and current success code `SUCCESS`.
- New step-up and reveal endpoints use `POST`, kebab-case URLs, and no path parameters.
- Variable display APIs must not leak sensitive plaintext through backend search, frontend filtering, mock data, logs, or errors.
- Runtime code must never treat `********` as an execution value.
- Workers must use the selected lite git feature model and must not edit the main worktree.
- Build and test validation must use Makefile targets first when available.

## Feature Model

Bounded context:

- Sensitive handling for variable management values across storage, display, reveal, and runtime preparation.

Entities:

- `VariableSensitiveMetadata`: storage metadata containing sensitivity flag, nonce, tag, algorithm, and key version.
- `VariableDisplayRecord`: API/display record that masks sensitive values and hides crypto metadata.
- `VariableStoredValue`: backend-only storage payload that can be plaintext or encrypted ciphertext.
- `StepUpProof`: short-lived proof bound to current user and reveal target.
- `RuntimeVariableSet`: backend-only variable set that can contain plaintext for trusted execution.

Policies:

- Sensitive values are encrypted only on the backend.
- Display reads never decrypt sensitive values.
- Reveal decrypts exactly one sensitive value after token-based password step-up.
- Editing a sensitive value without replacement preserves stored encrypted payload.
- Workspace overrides for inherited sensitive keys cannot downgrade sensitivity.
- Untrusted external runtime preparation fails before sensitive plaintext leaves the backend.

## Page And Module Surface

Workspace variable page:

- Route surface: `/variables`
- Behavior: show sensitive indicator, display `********`, provide reveal and hide controls, preserve mask across refresh, and remove sensitive plaintext on close, navigation, workspace switch, or reload.

Platform variable pages:

- Route surface: `/settings/variables` and related create/edit/detail routes.
- Behavior: support sensitive toggle, masked values, reveal dialog, edit preserve or replace, and no plaintext prefill for sensitive rows.

Backend API:

- Modules: `adapters/inbound/http/variables`, `adapters/inbound/http/admin/variables`, `adapters/inbound/http/auth`, and application services under `application/variable` and `application/user`.
- Behavior: expose display-safe list/mutation responses, step-up token issue, and scoped reveal.

Backend persistence:

- Modules: SQLite migrations and `adapters/outbound/persistence/sqlite/variable_repository.rs`.
- Behavior: store sensitive metadata and encrypted payloads while keeping legacy non-sensitive rows compatible.

Backend runtime:

- Modules: `application/agent/invocation.rs`, runtime adapters, and agent context preparation.
- Behavior: request runtime variables through a dedicated resolver and enforce service-mode trust before payload delivery.

## Repository Findings

- `workspace_variables.variable_value` and `platform_variables.variable_value` currently store plaintext only.
- `VariableService::list_variables` currently serves both UI display and agent runtime preparation.
- Backend search in `application/variable/service/projection.rs` matches `record.value`.
- Frontend workspace and platform pages render and search `record.value`.
- `AgentInvocationService` puts `list_variables` results into `AgentRunContext.variables`.
- `service_mode_runtime` posts the complete `AgentRunContext` to an external endpoint.
- Existing bcrypt password hashing verifies local passwords but cannot encrypt or decrypt variable values.
- Existing Makefile exposes `be-check`, `be-test`, `fe-check`, and `fe-test`.

## Open Decisions

- None for v1 feature scope. Production secret injection remains a deployment configuration responsibility and must fail closed when absent.
