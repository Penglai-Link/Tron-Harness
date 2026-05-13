# Test Scope: User Management Model

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

The TDD scope is revised to match the user-only MVP and the updated testing documentation template. Role Management, Space Management, Permission Management, API authorization, invitation email delivery, and audit-log pages are excluded. Password management uses bcrypt and a simplified backend-generated temporary password flow; it does not introduce RBAC or email verification.

Evolution alignment:

- workflow: Workflow B Feature Evolution
- change_classification: acceptance criteria, frontend interaction test scope, and evidence correction
- affected_files: `spec/acceptance.md`, `spec/test-scope.md`, `spec/test-plan.md`, `tasks/test-task.md`, `tasks/frontend-task.md`, `orchestration/worktree-plan.md`, `orchestration/integration-log.md`, `reports/test-report.md`, `reports/frontend-result.md`, `reports/review-report.md`
- retained_decisions: selected custom MVP scope, AC-UM-01 through AC-UM-08, no API permission enforcement, no invitation email delivery, backend-generated temporary password reset
- superseded_shape: the broad `TDD-UM-08` browser smoke is retained for model/route alignment but superseded for search/pagination backend requests, Export download, Add User submit, Reset API call, and visible Copy feedback
- pause_resume_decision: prior worker branches are integrated; new dispatch must use new Lite Spec worktrees and preserve existing evidence as retained but incomplete for evolved interactions

## Scope Decision

- status: approved_with_defaults
- selected_profile: custom
- approved_by: clouds3n
- approved_at: 2026-05-12

Valid profiles: `minimal`, `standard`, `strict`, `custom`.

## Recommended Scope

Use a `custom` MVP scope because this feature has database impact, API contract impact, frontend model impact, password-sensitive data, and cross-module stable-code references.

Recommended coverage:

- Strong unit and integration coverage for user persistence, 64-bit Snowflake IDs, JSON-safe ID serialization, rich profile attributes, status lifecycle including restore, code stability, many-to-many assignments, and password reset.
- API contract coverage for `/api/v1/admin/users/*`, including parameter passing method, uniform response body format, error codes, and no API permission gate.
- Frontend adapter/component coverage for the user-only model boundary and renamed `username` field.
- Browser runtime smoke coverage for the critical User Management flows after implementation exists.
- Security coverage focused on bcrypt hash-only persistence, one-time temporary password response, and no email verification.

Minimum recommended TDD boundary:

- TDD-UM-01: user list/detail read model, query filtering, pagination, stable `userCode`, and archived default filtering.
- TDD-UM-02: user lifecycle, Snowflake IDs, profile validation, create, update, archive, and restore.
- TDD-UM-03: fixed role-code assignment replacement.
- TDD-UM-04: external space-code assignment replacement.
- TDD-UM-05: bcrypt password hashing and backend-generated temporary password reset.
- TDD-UM-06: stable code fields and JSON-safe ID contracts.
- TDD-UM-07: no role/permission authorization gate in MVP user APIs.
- TDD-UM-08: frontend user-only DTOs, mocks, routes, and browser smoke for directory/detail/reset.
- TDD-UM-09: frontend directory search/status/pagination sends parameterized list API requests and renders paged response data.
- TDD-UM-10: frontend Add User opens a form, submits the create API, handles the uniform response body, and updates visible directory state.
- TDD-UM-11: frontend Export creates a real CSV/download artifact for the current directory result set.
- TDD-UM-12: frontend reset confirmation calls the backend reset API, displays the backend-generated temporary password once, and Copy produces visible feedback.
- TDD-UM-13: frontend detail profile, assignment, archive, and restore interactions call backend mutation endpoints and refresh visible state from responses.

## Included Test Layers

| layer | included | rationale | command or evidence |
| --- | --- | --- | --- |
| unit | yes | Core user lifecycle, restore behavior, Snowflake ID handling, JSON-safe serialization, profile attributes, stable codes, assignments, and password behavior. | Future Rust unit commands under `tron-backend`. |
| integration | yes | API/repository/transaction behavior is required for user writes, assignment replacement, and reset flows. | Future Rust integration/API commands under `tron-backend`. |
| api-contract | yes | User Management API is being redesigned and must follow `/api/v1/admin/users` plus uniform response format. | Future API contract tests for `/api/v1/admin/users/*`. |
| database-migration | yes | Database impact is `yes` for user-owned tables, assignment tables, indexes, and reset events. | Future SQLite migration/repository tests. |
| frontend-component | yes | Frontend user DTOs, mocks, and adapters must change from the current mixed admin model. | `make fe-check` plus component/adapter tests if added. |
| browser-runtime | yes | User directory/detail/reset/create/export/search/pagination are user-facing admin workflows that need runtime evidence and network/download assertions. | Playwright or browser smoke test against a real app server. |
| performance | no | Initial scale is low thousands of users; indexes and pagination are enough for MVP. | Revisit when user volume or complex search requirements grow. |
| security | yes | Password hashes, generated temporary credentials, email, and username are sensitive data. | Hash-only, generated temporary password, one-time response, and no-email-verification tests. |
| manual-review | yes | Feature-oriented workflow requires traceability and final acceptance review. | `reports/review-report.md` in later phase. |

## Excluded Test Layers

- Role Management tests: excluded because roles are external and fixed by code in this MVP.
- Space Management tests: excluded because spaces are external and referenced by `spaceCode`.
- Permission Management tests: excluded because permissions are external and only require future `permissionCode` contract.
- API authorization/RBAC enforcement tests: excluded because MVP does not use API permissions.
- Invitation email tests: excluded because invitation email sending is not in scope.
- Email verification and reset-token verification tests: excluded because MVP reset directly generates a temporary password.
- Audit-log page tests: excluded because the User Management module does not own the audit-log page.
- Load testing: excluded until concrete user volume and concurrency targets are defined.
- Cross-database tests: excluded because the current backend tech stack declares SQLite as the default storage option.

## Environment Requirements

- Frontend dependencies installed under `tron-frontend`.
- Existing Makefile targets remain the entry point for frontend checks.
- Backend Rust workspace, Cargo manifest, SQLite migration tooling, and bcrypt-compatible password hashing library need to exist before backend TDD tasks can run.
- API-mode frontend tests require `VITE_TRON_ADMIN_DATA_SOURCE=api` or `VITE_TRON_DATA_SOURCE=api` plus `VITE_TRON_API_BASE_URL`.
- Browser tests should run against a real Vite dev server or integrated backend runtime, depending on implementation phase.
- Test data must include 64-bit Snowflake IDs, first/last/display names, usernames, active users, archived users, restored users, fixed role-code assignments, space-code assignments, bcrypt password hashes, normal password state, and reset-required password state.
- Password reset test fixtures must never store plaintext temporary passwords after the one-time response assertion.

## Test Boundary Matrix

| acceptance ids | required layers | risk or boundary | included | evidence expectation | command candidate | notes |
| --- | --- | --- | --- | --- | --- | --- |
| AC-UM-01 | integration/api-contract/browser-runtime | Stable-code reads, default archived filter, parameterized search/status/pagination requests, export download, and user journey continuity. | yes | RED proves missing list/detail/filter behavior or missing frontend network/download assertion; GREEN proves API + browser path. | `rtk cargo test --manifest-path tron-backend/Cargo.toml user_management_list_detail`; `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke` | Browser run must assert network query parameters and download output, not only page presence. |
| AC-UM-02 | unit/integration/database/browser-runtime | Lifecycle correctness: create/update/archive/restore, JSON-safe Snowflake serialization, and Add User submit/visible state. | yes | RED proves lifecycle gaps or Add User UI submit gap; GREEN proves persistence/API/browser recovery journey. | `rtk cargo test --manifest-path tron-backend/Cargo.toml user_management_lifecycle`; `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke` | Includes logical deletion, restore path, and frontend create flow. |
| AC-UM-03 | unit/integration/api-contract | Fixed role-code assignment and invalid-code rejection. | yes | RED proves assignment validation is missing; GREEN proves replace semantics and constraints. | `rtk cargo test --manifest-path tron-backend/Cargo.toml user_role_assignments` | Role definitions remain external. |
| AC-UM-04 | unit/integration/api-contract | External space-code reference replacement and invalid-space rejection. | yes | RED proves external reference boundary is missing; GREEN proves stable `spaceCode` assignment behavior. | `rtk cargo test --manifest-path tron-backend/Cargo.toml user_space_assignments` | Space hierarchy remains out of scope. |
| AC-UM-05 | unit/integration/security/browser-runtime | bcrypt hash-only persistence, one-time temporary password response, reset API call from UI, and visible Copy feedback. | yes | RED proves reset policy/API/UI feedback is missing; GREEN proves secure reset flow and one-time UI display. | `rtk cargo test --manifest-path tron-backend/Cargo.toml password_reset`; `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke` | No email verification and no API permission check in MVP. |
| AC-UM-06 | api-contract/database | Stable public code fields and decimal-string ID contract. | yes | RED proves contract drift; GREEN proves DTO/schema stability. | `rtk cargo test --manifest-path tron-backend/Cargo.toml stable_code_contracts` | Includes future `permissionCode` contract note boundary. |
| AC-UM-07 | integration/review | API authorization boundary must stay decoupled from role/permission data. | yes | RED or review check proves accidental coupling is detectable; GREEN confirms no permission gate in MVP. | `rtk cargo test --manifest-path tron-backend/Cargo.toml user_api_authorization_boundary` | Security-hardening phase may later supersede this rule. |
| AC-UM-08 | frontend-component/browser-runtime/review | Frontend user-only model alignment and route continuity. | yes | RED proves old mixed model remains; GREEN proves adapted model + runtime journey. | `rtk make fe-check`; `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke` | Role/space pages stay independent modules. |

## TDD Cycle Matrix

Define the intended TDD cycles before task breakdown. Phase 1 does not need final test file paths, but it must define what RED test should be written first, what failure proves the behavior is missing, the command candidate, and what GREEN evidence is required after implementation.

| cycle id | acceptance ids | test layer | RED test intent | RED expected failure | command candidate | GREEN required evidence | status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TDD-UM-01 | AC-UM-01 | integration/api-contract | Add list/detail tests for stable `userCode`, query filtering, pagination, and default exclusion of archived users. | Failure proves the backend does not yet satisfy the redesigned list/detail contract, not just that a route is absent. | `cargo test --manifest-path tron-backend/Cargo.toml user_management_list_detail` | Same command passes with API/repository evidence for list, detail, filtering, pagination, and archived default behavior. | blocked_backend_missing |
| TDD-UM-02 | AC-UM-02 | unit/integration/database | Add lifecycle tests for Snowflake IDs, JSON-safe ID serialization, profile validation, create, update, archive, restore, and logical deletion. | Failure proves user lifecycle and profile rules are not implemented or not persisted correctly. | `cargo test --manifest-path tron-backend/Cargo.toml user_management_lifecycle` | Same command passes with domain, repository, migration, and API evidence. | blocked_backend_missing |
| TDD-UM-03 | AC-UM-03 | unit/integration/api-contract | Add tests for replacing user fixed role-code assignments and rejecting invalid role codes. | Failure proves many-to-many role-code assignment behavior or validation is missing. | `cargo test --manifest-path tron-backend/Cargo.toml user_role_assignments` | Same command passes with persistence/API evidence for fixed role-code assignments. | blocked_backend_missing |
| TDD-UM-04 | AC-UM-04 | unit/integration/api-contract | Add tests for replacing user space-code assignments and rejecting blank or malformed space codes. | Failure proves many-to-many external space-code assignment behavior or validation is missing. | `cargo test --manifest-path tron-backend/Cargo.toml user_space_assignments` | Same command passes with persistence/API evidence that User Management only references external `spaceCode`. | blocked_backend_missing |
| TDD-UM-05 | AC-UM-05 | unit/integration/security | Add tests for bcrypt hash-only persistence, backend-generated temporary password reset, reset-required state, one-time response, and no email verification. | Failure proves password reset/security behavior is missing or violates MVP constraints. | `cargo test --manifest-path tron-backend/Cargo.toml password_reset` | Same command passes with evidence that plaintext temporary passwords are not persisted and hash algorithm is bcrypt. | blocked_backend_missing |
| TDD-UM-06 | AC-UM-06 | api-contract/database | Add DTO/schema tests for `userCode`, `roleCode`, `spaceCode`, future `permissionCode` contract notes, and decimal-string Snowflake IDs. | Failure proves stable code fields or JSON-safe ID serialization are missing from public contracts. | `cargo test --manifest-path tron-backend/Cargo.toml stable_code_contracts` | Same command passes with DTO/schema/repository evidence that codes are stable public fields. | blocked_backend_missing |
| TDD-UM-07 | AC-UM-07 | integration/review | Add a boundary test or review check proving user APIs do not require role or permission authorization in the MVP. | Failure proves endpoint behavior is coupled to role/permission authorization. | `cargo test --manifest-path tron-backend/Cargo.toml user_api_authorization_boundary` | Same command or approved review check passes, confirming role assignments are managed data only. | blocked_backend_missing |
| TDD-UM-08 | AC-UM-01, AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-05, AC-UM-08 | frontend-component/browser-runtime | Add frontend adapter/component and browser smoke tests for user-only DTOs, directory/detail navigation, archive/restore, assignment editing, and reset dialog. | Failure proves the current frontend model or runtime journey is still misaligned with the redesigned user-only contract. | `make fe-check` and `npm --prefix tron-frontend exec playwright -- test user-management-smoke` | Commands pass with browser evidence for directory, detail, assignments, archive/restore, and one-time reset result. | blocked_until_tests_created |
| TDD-UM-09 | AC-UM-01, AC-UM-08 | browser-runtime/api-adapter | Add browser tests that click pagination, type search, change status, and assert parameterized `GET /api/v1/admin/users` requests and paged response rendering. | Failure proves the current frontend only filters/slices local state or fails to pass query params. | `npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "directory query parameters"` | Same focused smoke passes and full slow/browser VERIFY records query-parameter evidence. | blocked_until_tests_created |
| TDD-UM-10 | AC-UM-02, AC-UM-08 | browser-runtime/api-adapter | Add browser tests that open Add User, submit required fields, assert `POST /api/v1/admin/users`, and verify the created user becomes visible. | Failure proves the current Add User button has no real form or create API call. | `npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "add user"` | Same focused smoke passes with payload and visible-state evidence. | blocked_until_tests_created |
| TDD-UM-11 | AC-UM-01, AC-UM-08 | browser-runtime/frontend-component | Add browser tests that click Export, wait for a download, and inspect CSV headers/content. | Failure proves Export only emits a notice and does not produce an artifact. | `npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "export users"` | Same focused smoke passes with filename and CSV content evidence. | blocked_until_tests_created |
| TDD-UM-12 | AC-UM-05, AC-UM-08 | browser-runtime/api-adapter | Add browser tests that confirm reset, assert the reset API request, show the backend temporary password, Copy it, see visible feedback, close the result, and verify no re-reveal. | Failure proves reset is client-generated or Copy feedback is not visible to operators. | `npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "reset password"` | Same focused smoke passes with network, result-dialog, copy-feedback, and no-reveal evidence. | blocked_until_tests_created |
| TDD-UM-13 | AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-08 | browser-runtime/api-adapter | Add browser tests that edit profile fields, toggle role/space assignments, archive, restore, and assert backend mutation requests plus response-driven UI updates. | Failure proves detail interactions only mutate local state or lack profile edit capability. | `npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "detail mutations"` | Same focused smoke passes with PATCH/PUT/archive/restore request and visible-state evidence. | blocked_until_tests_created |

## Regression Command Candidates

| command | purpose | required for final | status |
| --- | --- | --- | --- |
| `rtk make fe-check` | Frontend typecheck and production build. | yes | available |
| `rtk make be-test` | Backend regression gate after user-management backend implementation. | yes | pending_implementation |
| `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke` | Browser runtime verification for user directory/detail/reset journeys. | yes after smoke spec exists | blocked_until_tests_created |
| `rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/user-management-model --phase spec --spec-stage acceptance` | Validate Phase 1D acceptance and test-scope docs. | yes for current phase | available |
| `rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/user-management-model --phase final` | Final feature evidence validation. | yes after implementation | blocked_until_later_phases |

## Evidence Required For Completion

| acceptance ids | TDD cycle ids | RED evidence type | GREEN evidence type |
| --- | --- | --- | --- |
| AC-UM-01 | TDD-UM-01, TDD-UM-08 | Failing list/detail and browser tests showing missing stable-code reads or archived filtering. | Passing API tests and browser evidence for directory/detail navigation. |
| AC-UM-02 | TDD-UM-02, TDD-UM-08 | Failing lifecycle tests for Snowflake IDs, profile validation, archive, restore, or JSON-safe serialization. | Passing lifecycle tests plus database/API/browser evidence. |
| AC-UM-03 | TDD-UM-03, TDD-UM-08 | Failing role-assignment replacement or invalid-code rejection tests. | Passing assignment tests for fixed role codes plus frontend evidence. |
| AC-UM-04 | TDD-UM-04, TDD-UM-08 | Failing space-assignment replacement or invalid/blank-code rejection tests. | Passing assignment tests for external `spaceCode` references plus frontend evidence. |
| AC-UM-05 | TDD-UM-05, TDD-UM-08 | Failing password reset tests for bcrypt, temporary-password generation, one-time response, or no-email behavior. | Passing password/security evidence with no plaintext persistence plus reset UI evidence. |
| AC-UM-06 | TDD-UM-06 | Failing stable-code or JSON-safe ID contract tests. | Passing DTO/schema/repository contract evidence. |
| AC-UM-07 | TDD-UM-07 | Failing boundary test or review check if API authorization is coupled to role/permission data. | Passing boundary evidence confirming no API permission enforcement in MVP. |
| AC-UM-08 | TDD-UM-08 | Failing frontend adapter/component/browser smoke evidence for current user model mismatch. | Passing `make fe-check` and browser smoke evidence. |
| AC-UM-01 | TDD-UM-09, TDD-UM-11 | Failing browser tests for missing parameterized list requests or CSV download. | Passing browser evidence for query parameters, pagination, and export artifact. |
| AC-UM-02 | TDD-UM-10 | Failing browser test for missing Add User form/API submit. | Passing browser evidence for create request and visible directory update. |
| AC-UM-05 | TDD-UM-12 | Failing browser test for client-generated reset or invisible Copy feedback. | Passing browser evidence for reset API call, visible result/copy feedback, and no re-reveal after close. |
| AC-UM-02, AC-UM-03, AC-UM-04 | TDD-UM-13 | Failing browser test for local-only detail profile, assignment, archive, or restore mutations. | Passing browser evidence for backend mutation requests and response-driven detail state. |

## Open Decisions

- None.
