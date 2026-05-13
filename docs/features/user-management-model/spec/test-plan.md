# Test Plan: User Management Model

## Test Plan Decision

- status: approved_with_defaults
- approved_by: clouds3n
- approved_at: 2026-05-13
- evolution_reason: user supplied Workflow B consistency refactor plan covering backend statistics, list/search semantics, seed data, frontend cards, directory layout, and create sub-page routing.

Valid statuses: `waiting_human_decision`, `approved`, `approved_with_defaults`.

Use `approved` or `approved_with_defaults` only after the user reviews this exact test plan and explicitly approves moving to task breakdown.

## Test Strategy

This plan continues the approved custom scope from `spec/test-scope.md` and maps each acceptance criterion to executable TDD cycles. The backend already has runtime/bootstrap, shared Snowflake capability, and initial user-management APIs, so evolution RED cycles start from frontend interaction gaps that escaped the first browser smoke.

Execution order is strict: RED tests first in worker test scope, then GREEN implementation in backend/frontend scopes, then verify/review. Password-security and boundary cycles are isolated to prevent mixed failure causes. Evolution cycles `TDD-UM-09..12` supersede the broad frontend part of `TDD-UM-08` for the affected interactions only; consistency refactor cycles `TDD-UM-14..18` supersede current-page-derived dashboard counts, inline create ownership, and loose directory layout. Earlier backend/domain evidence remains retained.

## TDD Cycle Table

Short index only. Group cycles by scenario category. The only identifier in these tables is the TDD cycle ID. Use `TDD-<MODULE>-NN` format, where `<MODULE>` matches the primary acceptance ID module segment.

### Positive Scenarios

| cycle id | primary acceptance id | related acceptance ids | test slice type | slice focus | test layer | expected result | status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TDD-UM-01 | AC-UM-01 | AC-UM-08 | core-business | User list/detail reads by stable `userCode`, pagination/filtering, and default archived exclusion. | integration/api-contract/browser-runtime | List/detail APIs and frontend journey succeed with default active-only behavior. | draft |
| TDD-UM-02 | AC-UM-02 | AC-UM-06 | core-business | Create/update/archive/restore lifecycle with 64-bit Snowflake IDs and decimal-string DTO serialization. | unit/integration/database | Lifecycle transitions and persistence invariants succeed. | draft |
| TDD-UM-03 | AC-UM-03 | AC-UM-06 | core-business | Replace fixed role-code assignments with many-to-many persistence. | unit/integration/api-contract | Valid fixed role-code replacement succeeds and remains stable in DTO reads. | draft |
| TDD-UM-09 | AC-UM-01 | AC-UM-08 | frontend-integration | User directory search, status filter, and pagination pass `search`, `status`, `current`, and `size` query parameters to `GET /api/v1/admin/users` instead of only filtering/slicing local state. | browser-runtime/api-adapter | Runtime assertions observe new list requests with expected query parameters after search/filter/page interactions. | draft |
| TDD-UM-10 | AC-UM-02 | AC-UM-03, AC-UM-04, AC-UM-05, AC-UM-08 | frontend-integration | Add User opens a usable form, submits `POST /api/v1/admin/users`, handles the uniform response body, and refreshes or inserts the created user in the directory. | browser-runtime/api-adapter | Runtime assertions observe create payload and visible created-user feedback/state. | draft |
| TDD-UM-11 | AC-UM-01 | AC-UM-08 | frontend-integration | Export creates a real CSV/download artifact for the current directory result set with stable user-management headers. | browser-runtime/frontend-component | Runtime assertions observe a download and validate CSV filename/content headers. | draft |
| TDD-UM-13 | AC-UM-02 | AC-UM-03, AC-UM-04, AC-UM-08 | frontend-integration | Detail profile, role-code, space-code, archive, and restore interactions call the backend mutation endpoints and refresh visible state. | browser-runtime/api-adapter | Runtime assertions observe PATCH/PUT/archive/restore requests and response-driven UI updates. | draft |
| TDD-UM-14 | AC-UM-09 | AC-UM-11 | api-contract | Four statistic endpoints return uniform envelopes with `data.count` for total users, active users, role assignments, and space assignments. | integration/api-contract | Focused backend assertions observe exact counts from deterministic persisted data. | draft |
| TDD-UM-15 | AC-UM-11 | AC-UM-01, AC-UM-09 | core-business | List/search repository semantics match statistic semantics for default active-only, archived-only, include-archived, pagination totals, and expanded seeds. | integration/database | Focused backend assertions observe filtered database totals before pagination. | draft |
| TDD-UM-16 | AC-UM-09 | AC-UM-08 | frontend-integration | Statistic cards call the four statistic APIs and render `data.count`, not aggregates from `users.records`. | browser-runtime/api-adapter | Runtime assertions observe four statistic GETs and independent card values. | draft |
| TDD-UM-17 | AC-UM-10 | AC-UM-08 | frontend-integration | Directory table renders display name only in `User`, email/phone in `Contact`, and `Space Assignments` card copy. | browser-runtime/frontend-component | Runtime assertions observe the revised visible table/card layout. | draft |
| TDD-UM-18 | AC-UM-10 | AC-UM-02, AC-UM-08 | frontend-integration | Add User navigates to `/admin/users/new`; CreateUserPage owns form submit to `POST /api/v1/admin/users`. | browser-runtime/api-adapter | Runtime assertions observe route transition, create request payload, and visible success feedback. | draft |

### Exception Scenarios

| cycle id | primary acceptance id | related acceptance ids | test slice type | slice focus | test layer | expected result | status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TDD-UM-04 | AC-UM-04 | AC-UM-06 | error | Reject blank/malformed space codes and preserve existing assignments on rejection. | unit/integration/api-contract | Invalid space-code replacement fails with stable error contract. | draft |
| TDD-UM-05 | AC-UM-05 | AC-UM-07 | error/security | Reset-password flow enforces bcrypt hash-only persistence and one-time temporary-password response. | unit/integration/security/browser-runtime | Reset succeeds securely; plaintext is not persisted and cannot be re-read. | draft |
| TDD-UM-07 | AC-UM-07 | AC-UM-03, AC-UM-04 | permission-boundary | User APIs stay decoupled from role/permission authorization checks in MVP. | integration/review | Endpoints do not require role/permission gate and do not return authorization errors from managed role data. | draft |
| TDD-UM-12 | AC-UM-05 | AC-UM-08 | frontend-integration | Reset Password confirmation calls `POST /api/v1/admin/users/{userCode}/password/reset`, renders the backend-generated temporary password once, and Copy produces visible operator feedback. | browser-runtime/api-adapter | Runtime assertions observe reset API call, visible result dialog, visible copy feedback, and no re-reveal after close. | draft |

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
- Evolution note: retained for broad model/route alignment. It no longer counts as sufficient evidence for search/pagination backend requests, Export download, Add User submit, Reset API call, or visible Copy feedback; those are split into `TDD-UM-09..12`.
- Status: retained_evidence_incomplete_for_evolved_interactions

### TDD-UM-09 Details

- Assertions: directory load, status filter, search input, and pagination each use the user list API with query parameters matching `spec/api-contract.md`; pagination is driven by the paged response envelope.
- Test data: intercepted API responses with distinct page/search/status payloads so client-only filtering or local slicing cannot satisfy the test.
- RED proof: `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "directory query parameters"` fails because current interactions do not issue new parameterized list requests.
- GREEN/VERIFY evidence: same command passes, and slow/browser full smoke verifies the runtime journey once after integration.
- Status: draft

### TDD-UM-10 Details

- Assertions: Add User opens a form, validates required fields, submits `POST /api/v1/admin/users`, maps the uniform response body, and makes the created user visible in the directory.
- Test data: create payload for `USER_MAYA_SINGH`, fixed role-code and space-code arrays, and mock API success/error envelopes.
- RED proof: `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "add user"` fails because the current button only emits an invisible live-region notice.
- GREEN/VERIFY evidence: same command passes with request payload and visible created-user assertions.
- Status: draft

### TDD-UM-11 Details

- Assertions: Export creates a real CSV download for the current directory result set and includes stable headers for user code, username, display name, email, status, roles, and spaces.
- Test data: current directory rows from an intercepted list response.
- RED proof: `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "export users"` fails because no download is produced.
- GREEN/VERIFY evidence: same command passes with download filename and CSV content assertions.
- Status: draft

### TDD-UM-12 Details

- Assertions: reset confirmation calls the backend reset endpoint, displays the returned temporary password in a result dialog, Copy produces visible feedback, and closing the dialog removes the temporary password from the UI.
- Test data: reset response containing `temporaryPassword` and `passwordResetRequired`.
- RED proof: `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "reset password"` fails because current code generates the password client-side and Copy feedback is screen-reader-only.
- GREEN/VERIFY evidence: same command passes with reset request, result dialog, Copy feedback, and close/no-reveal assertions.
- Status: draft

### TDD-UM-13 Details

- Assertions: detail profile edit submits `PATCH /api/v1/admin/users/{userCode}`, role-code toggles submit `PUT /roles`, space-code toggles submit `PUT /spaces`, archive submits `/archive`, restore submits `/restore`, and each visible state change is driven by the response body.
- Test data: intercepted detail/mutation responses for `USER_ALEX_JOHNSON` and an archived/active transition pair.
- RED proof: `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "detail mutations"` fails because current detail interactions mutate local React state without backend calls.
- GREEN/VERIFY evidence: same command passes with mutation request assertions and response-driven visible state.
- Status: draft

### TDD-UM-14 Details

- Assertions: `GET /api/v1/admin/users/statistics/total-users`, `active-users`, `role-assignments`, and `space-assignments` return `SUCCESS` envelopes with `data.count`.
- Test data: deterministic SQLite fixture seeded with active and archived users, multi-role users, and multi-space users.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml user_management_statistics` fails because statistic routes or repository count methods are missing.
- GREEN/VERIFY evidence: same command passes with exact total, active, role-assignment, and space-assignment row counts.
- Status: draft

### TDD-UM-15 Details

- Assertions: default list excludes archived users; `status=Archived` returns archived users only; `includeArchived=true` returns all statuses; pagination `total` equals filtered database count before pagination; seed size supports more than one page.
- Test data: expanded seeded users across `Active` and `Archived`, distinct role-code and space-code assignment counts, and page size smaller than the active set.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml user_management_statistics user_management_list_detail` fails on count/list mismatch or insufficient deterministic seeds.
- GREEN/VERIFY evidence: same command passes with repository-backed filtered totals and pagination evidence.
- Status: draft

### TDD-UM-16 Details

- Assertions: `/admin/users` statistic cards call all four statistic endpoints and render `data.count` values independent of the current paged list records.
- Test data: intercepted statistic responses with counts that intentionally differ from the current page length.
- RED proof: `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "statistics cards"` fails because cards currently derive from `users.records`/`users.total`.
- GREEN/VERIFY evidence: same command passes and full slow/browser smoke records network/card evidence.
- Status: draft

### TDD-UM-17 Details

- Assertions: `User` column shows only the fullname/display name, `Contact` column shows email and phone number, and the assignment card label reads `Space Assignments`.
- Test data: user row with username, email, and phone so the test can prove username/email moved out of `User`.
- RED proof: `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "directory columns"` fails because the current directory still renders username/email under `User` and `Space Links`.
- GREEN/VERIFY evidence: same command passes with visible column and label assertions.
- Status: draft

### TDD-UM-18 Details

- Assertions: Add User button navigates to `/admin/users/new`; `admin.usersNew` route and `CreateUserPage` render; create submit calls `POST /api/v1/admin/users`, handles the uniform response body, shows visible feedback, and returns to `/admin/users`.
- Test data: create payload for `USER_MAYA_SINGH`, fixed role-code and space-code arrays, and mock API success/error envelopes.
- RED proof: `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "create user page"` fails because the current directory owns an inline create form and no `/admin/users/new` route exists.
- GREEN/VERIFY evidence: same command passes with route, request payload, response handling, and visible feedback assertions.
- Status: draft

## Unit Test Plan

Unit coverage focuses on lifecycle invariants, role/space validation, password reset policy helpers, and DTO serialization helpers for stable code fields and Snowflake ID string boundaries.

## Integration Test Plan

Integration coverage focuses on repository writes/reads, assignment replacement transactions, list/detail query filters, restore behavior, and API-level reset/authorization-boundary behavior.

## Database Test Plan

Database coverage includes migration application, schema constraints/indexes for users and assignment tables, uniqueness checks, and reset-event persistence invariants.

## Frontend Test Plan

Frontend coverage includes DTO/adapters for user-only boundary, page-model updates for directory/detail flows, statistic-card hooks, status filtering behavior for `Active` and `Archived`, parameterized list requests, Add User create sub-page flow, CSV export, backend reset integration, and visible operator feedback.

## Browser Acceptance Plan

Run smoke flow on a real app runtime: open `/admin/users`, verify statistic API calls and card counts, filter/search, paginate, export current results, navigate to `/admin/users/new`, create a user, open detail by `userCode`, trigger password reset through the backend API, copy the temporary password with visible feedback, close the result dialog, and verify the temporary password cannot be revealed again.

## Security And Privacy Test Plan

Security coverage verifies bcrypt hash-only storage, one-time temporary-password response handling, no role/permission authorization enforcement in MVP user APIs, and no plaintext password persistence in fixtures/logs.

## Test Data And Fixtures

Shared fixtures include active/archived users, fixed role-code sets, space-code sets, duplicate-identifier cases, reset-required state, large 64-bit IDs serialized as strings, more than one page of active users, and assignment row counts that differ from user counts.

## Regression Scope

Regression re-runs include backend focused user-management suites, frontend typecheck/build, browser smoke journey, and feature-doc validator gates for spec/tasks/worktrees/reports/final as the workflow advances.

## Command Matrix

| command | purpose | when to run | expected evidence |
| --- | --- | --- | --- |
| `rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/user-management-model --phase spec --complete-spec` | Validate complete spec and test plan before task execution. | After any spec/test-plan edit. | Validation passes. |
| `rtk cargo test --manifest-path tron-backend/Cargo.toml user_management` | Run backend user-management suites (lifecycle/assignment/reset/contracts). | RED, GREEN, verify, and final regression phases. | RED failures prove missing behavior; GREEN/final pass. |
| `rtk cargo test --manifest-path tron-backend/Cargo.toml user_management_statistics` | Run backend statistic/list consistency suites. | RED, GREEN, verify, and final regression phases. | RED failures prove missing statistic/list consistency; GREEN/final pass. |
| `rtk make be-test` | Backend Makefile-first regression gate. | After GREEN merges and before final review. | Backend tests pass through Makefile command. |
| `rtk make fe-check` | Frontend typecheck/build regression. | After frontend adapter/page changes and before final review. | Typecheck/build pass. |
| `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke` | Browser runtime smoke for user management journeys. | After frontend/backend integration is runnable. | Smoke journey passes with query-parameter, export, create, reset API, and visible feedback evidence. |

## Command Index

- command: `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke`; purpose: browser runtime verification for evolved user-management interactions; cost: slow/browser; when: verify; rerun: review-if-stale or when VERIFY evidence is missing, failed, stale against changed frontend/API paths, or explicitly requested.
- command: `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "consistency refactor"`; purpose: focused browser RED/GREEN checks for consistency refactor interactions; cost: slow/browser; when: red/green/verify; rerun: review-if-stale or when VERIFY evidence is missing, failed, stale against changed frontend/API paths, or explicitly requested.
