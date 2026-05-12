# Test Plan: User Management Model

## Test Plan Decision

- status: approved_with_defaults
- approved_by: clouds3n
- approved_at: 2026-05-12

Valid statuses: `waiting_human_decision`, `approved`, `approved_with_defaults`.

Use `approved` or `approved_with_defaults` only after the user reviews this exact test plan and explicitly approves moving to task breakdown.

## Test Strategy

This plan continues the approved custom scope from `spec/test-scope.md` and maps each acceptance criterion to executable TDD cycles. The backend already has runtime/bootstrap and shared Snowflake capability, so RED cycles can start from user-management behavior gaps rather than crate bootstrap issues.

Execution order is strict: RED tests first in worker test scope, then GREEN implementation in backend/frontend scopes, then verify/review. Password-security and boundary cycles are isolated to prevent mixed failure causes.

## TDD Cycle Table

Short index only. Group cycles by scenario category. The only identifier in these tables is the TDD cycle ID. Use `TDD-<MODULE>-NN` format, where `<MODULE>` matches the primary acceptance ID module segment.

### Positive Scenarios

| cycle id | primary acceptance id | related acceptance ids | test slice type | slice focus | test layer | expected result | status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TDD-UM-01 | AC-UM-01 | AC-UM-08 | core-business | User list/detail reads by stable `userCode`, pagination/filtering, and default archived exclusion. | integration/api-contract/browser-runtime | List/detail APIs and frontend journey succeed with default active-only behavior. | draft |
| TDD-UM-02 | AC-UM-02 | AC-UM-06 | core-business | Create/update/archive/restore lifecycle with 64-bit Snowflake IDs and decimal-string DTO serialization. | unit/integration/database | Lifecycle transitions and persistence invariants succeed. | draft |
| TDD-UM-03 | AC-UM-03 | AC-UM-06 | core-business | Replace fixed role-code assignments with many-to-many persistence. | unit/integration/api-contract | Valid fixed role-code replacement succeeds and remains stable in DTO reads. | draft |

### Exception Scenarios

| cycle id | primary acceptance id | related acceptance ids | test slice type | slice focus | test layer | expected result | status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TDD-UM-04 | AC-UM-04 | AC-UM-06 | error | Reject blank/malformed space codes and preserve existing assignments on rejection. | unit/integration/api-contract | Invalid space-code replacement fails with stable error contract. | draft |
| TDD-UM-05 | AC-UM-05 | AC-UM-07 | error/security | Reset-password flow enforces bcrypt hash-only persistence and one-time temporary-password response. | unit/integration/security/browser-runtime | Reset succeeds securely; plaintext is not persisted and cannot be re-read. | draft |
| TDD-UM-07 | AC-UM-07 | AC-UM-03, AC-UM-04 | permission-boundary | User APIs stay decoupled from role/permission authorization checks in MVP. | integration/review | Endpoints do not require role/permission gate and do not return authorization errors from managed role data. | draft |

### Boundary Scenarios

| cycle id | primary acceptance id | related acceptance ids | test slice type | slice focus | test layer | expected result | status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TDD-UM-06 | AC-UM-06 | AC-UM-01, AC-UM-02 | boundary | Stable code fields and decimal-string 64-bit ID boundaries (including values beyond JavaScript safe integer). | api-contract/database | DTO/schema boundaries preserve exact ID values and stable code contracts. | draft |
| TDD-UM-08 | AC-UM-08 | AC-UM-01, AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-05 | boundary/regression | Frontend user-only model migration, route continuity, and smoke journey for directory/detail/archive-restore/reset. | frontend-component/browser-runtime | Frontend build/test/smoke passes without role/space boundary regression. | draft |

## TDD Cycle Details

### TDD-UM-01 Details

- Assertions: list endpoint defaults to active users; detail endpoint resolves by `userCode`; archived users are excluded unless explicitly requested.
- Test data: active user set, archived user set, multi-filter query fixtures, and pagination fixtures.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml user_management_list_detail` fails because list/detail handlers or repository query behavior are missing.
- GREEN/VERIFY evidence: same command passes and `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke` confirms directory/detail journey.
- Status: draft

### TDD-UM-02 Details

- Assertions: create/update/archive/restore transitions enforce profile requirements and logical-delete invariants; API serializes 64-bit IDs as strings.
- Test data: valid creation payload, duplicate username/email payloads, archived and restored records.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml user_management_lifecycle` fails because lifecycle domain/repository rules are missing.
- GREEN/VERIFY evidence: same command passes with migration/repository/API lifecycle evidence.
- Status: draft

### TDD-UM-03 Details

- Assertions: role assignments replace current set atomically; only `ADMIN`, `WORKSPACE_ADMIN`, `DEVELOPER`, `VIEWER` are accepted.
- Test data: zero-role user, multi-role replacement payload, invalid role payload (`OWNER`).
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml user_role_assignments` fails because assignment replacement/validation is missing.
- GREEN/VERIFY evidence: same command passes with persistence + API contract evidence.
- Status: draft

### TDD-UM-04 Details

- Assertions: invalid `spaceCode` values are rejected and do not partially persist; valid replacements preserve many-to-many semantics.
- Test data: blank `spaceCode`, malformed `spaceCode`, valid multi-space replacement payload.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml user_space_assignments` fails because space-code validation/replace behavior is missing.
- GREEN/VERIFY evidence: same command passes with replace semantics and stable error mapping evidence.
- Status: draft

### TDD-UM-05 Details

- Assertions: reset flow generates temporary password server-side, stores only bcrypt hash, marks reset-required flag, and returns temporary password once.
- Test data: user with existing hash, reset request fixture, one-time response replay fixture.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml password_reset` fails because reset policy/security behavior is missing.
- GREEN/VERIFY evidence: same command passes and `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke` verifies one-time reset dialog behavior.
- Status: draft

### TDD-UM-06 Details

- Assertions: public DTO contracts expose stable `userCode`, `roleCode`, `spaceCode`; internal Snowflake IDs serialize as decimal strings with exact value preservation.
- Test data: large 64-bit IDs (`9007199254740993`, `734103002743185408`) and stable code fixtures.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml stable_code_contracts` fails because schema/DTO boundaries are incomplete.
- GREEN/VERIFY evidence: same command passes with contract-level tests and schema checks.
- Status: draft

### TDD-UM-07 Details

- Assertions: user endpoints do not enforce role/permission authorization in MVP; role assignments remain managed data only.
- Test data: endpoint calls without permission context and with varied role assignment states.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml user_api_authorization_boundary` fails when endpoint behavior is coupled to role/permission gates.
- GREEN/VERIFY evidence: same command passes and review confirms no permission gate logic is introduced in user handlers.
- Status: draft

### TDD-UM-08 Details

- Assertions: frontend admin user model uses `userCode`, `username`, `roleCodes`, `spaceCodes`; user journeys remain functional after API boundary alignment.
- Test data: updated admin mock fixtures with `Active`/`Archived` only and reset-dialog states.
- RED proof: `rtk make fe-check` and `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke` fail while old mixed model persists.
- GREEN/VERIFY evidence: both commands pass with route and interaction evidence.
- Status: draft

## Unit Test Plan

Unit coverage focuses on lifecycle invariants, role/space validation, password reset policy helpers, and DTO serialization helpers for stable code fields and Snowflake ID string boundaries.

## Integration Test Plan

Integration coverage focuses on repository writes/reads, assignment replacement transactions, list/detail query filters, restore behavior, and API-level reset/authorization-boundary behavior.

## Database Test Plan

Database coverage includes migration application, schema constraints/indexes for users and assignment tables, uniqueness checks, and reset-event persistence invariants.

## Frontend Test Plan

Frontend coverage includes DTO/adapters for user-only boundary, page-model updates for directory/detail flows, and status filtering behavior for `Active` and `Archived`.

## Browser Acceptance Plan

Run smoke flow on a real app runtime: open `/admin/users`, filter/search, open detail by `userCode`, replace roles/spaces, archive/restore, trigger password reset, and verify one-time temporary-password dialog behavior.

## Security And Privacy Test Plan

Security coverage verifies bcrypt hash-only storage, one-time temporary-password response handling, no role/permission authorization enforcement in MVP user APIs, and no plaintext password persistence in fixtures/logs.

## Test Data And Fixtures

Shared fixtures include active/archived users, fixed role-code sets, space-code sets, duplicate-identifier cases, reset-required state, and large 64-bit IDs serialized as strings.

## Regression Scope

Regression re-runs include backend focused user-management suites, frontend typecheck/build, browser smoke journey, and feature-doc validator gates for spec/tasks/worktrees/reports/final as the workflow advances.

## Command Matrix

| command | purpose | when to run | expected evidence |
| --- | --- | --- | --- |
| `rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/user-management-model --phase spec --complete-spec` | Validate complete spec and test plan before task execution. | After any spec/test-plan edit. | Validation passes. |
| `rtk cargo test --manifest-path tron-backend/Cargo.toml user_management` | Run backend user-management suites (lifecycle/assignment/reset/contracts). | RED, GREEN, verify, and final regression phases. | RED failures prove missing behavior; GREEN/final pass. |
| `rtk make be-test` | Backend Makefile-first regression gate. | After GREEN merges and before final review. | Backend tests pass through Makefile command. |
| `rtk make fe-check` | Frontend typecheck/build regression. | After frontend adapter/page changes and before final review. | Typecheck/build pass. |
| `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke` | Browser runtime smoke for user management journeys. | After frontend/backend integration is runnable. | Smoke journey passes with archive/restore/reset evidence. |
