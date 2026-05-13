# Acceptance: User Management Model

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

Acceptance has been revised for a standalone User Management MVP. Role Management, Space Management, Permission Management, invitation email delivery, API permission enforcement, and audit-log pages are outside this feature.

Evolution alignment:

- workflow: Workflow B Feature Evolution
- change_classification: acceptance criteria, frontend interaction test scope, and evidence correction
- affected_files: `spec/acceptance.md`, `spec/test-scope.md`, `spec/test-plan.md`, `tasks/test-task.md`, `tasks/frontend-task.md`, `orchestration/worktree-plan.md`, `orchestration/integration-log.md`, `reports/test-report.md`, `reports/frontend-result.md`, `reports/review-report.md`
- retained_decisions: AC-UM-01 through AC-UM-08, fixed MVP roles, `Active`/`Archived`, bcrypt, backend-generated temporary password, no API permission enforcement
- superseded_shape: `TDD-UM-08` remains retained for frontend model/route alignment but does not satisfy evolved interaction evidence for search/pagination API requests, Export download, Add User submit, Reset API call, or visible Copy feedback
- consistency_refactor_shape: `TDD-UM-14..18` supersede current-page-derived statistics, inline directory create form ownership, and loose directory table copy for the affected behaviors only.
- pause_resume_decision: prior worktrees are integrated; new evidence must be produced by new RED/GREEN/VERIFY worktrees before final status can return to complete

## Acceptance Criteria Matrix

| id | criterion | evidence type | expected evidence | priority |
| --- | --- | --- | --- | --- |
| AC-UM-01 | User directory and user detail can read users by stable `userCode`, list/filter/paginate users through backend query parameters, export a real current-result CSV, and exclude archived users by default. | automated | TDD-UM-01, TDD-UM-09, TDD-UM-11, plus retained TDD-UM-08: RED proves stable-code reads, archived filtering, parameterized directory requests, or export download behavior is missing; GREEN passes list/detail API and browser network/download evidence. | must |
| AC-UM-02 | User lifecycle supports 64-bit Snowflake IDs, JSON-safe ID serialization, rich profile fields, create, profile update, `Active`, `Archived`, logical deletion, restore to active, and a frontend Add User flow that calls the create API. | automated | TDD-UM-02 plus TDD-UM-10: RED proves lifecycle/model or frontend create-submit behavior is missing; GREEN passes domain, repository, API, DTO, and browser create evidence. | must |
| AC-UM-03 | User-role assignment is many-to-many and uses only fixed role codes `ADMIN`, `WORKSPACE_ADMIN`, `DEVELOPER`, and `VIEWER`. | automated | TDD-UM-03: RED proves role-code assignment behavior is missing; GREEN passes fixed role-code assignment and invalid-code rejection evidence. | must |
| AC-UM-04 | User-space assignment is many-to-many and references external spaces by stable `spaceCode`. | automated | TDD-UM-04: RED proves space-code assignment behavior is missing; GREEN passes external space-code assignment evidence. | must |
| AC-UM-05 | User password management uses bcrypt hashes and supports MVP password reset by calling the backend reset API, generating a temporary password without email or permission checks, showing it once, and giving visible Copy feedback. | automated | TDD-UM-05 plus TDD-UM-12 and retained TDD-UM-08: RED proves password reset/security/API/UI feedback behavior is missing; GREEN passes bcrypt, one-time temporary-password, reset API, visible Copy feedback, and no-reveal evidence. | must |
| AC-UM-06 | User, role, space, and permission integration contracts use stable code fields to avoid ID-change inconsistency. | review + automated | TDD-UM-06: RED proves stable code or JSON-safe ID contract is missing; GREEN passes DTO/schema/repository contract evidence. | must |
| AC-UM-07 | MVP APIs do not use role or permission data for API authorization. | review + automated | TDD-UM-07: RED proves any accidental role/permission gate is detectable; GREEN passes boundary evidence that role assignments are managed data only. | must |
| AC-UM-08 | Existing frontend user-management flow is adjusted to the revised user-only boundary. | automated | TDD-UM-08: RED proves the frontend model or browser flow is still misaligned; GREEN passes `make fe-check` and browser smoke evidence. | should |
| AC-UM-09 | User dashboard statistic cards use dedicated backend statistic APIs, one endpoint per card, and each response returns `data.count` from persisted database state. | automated | TDD-UM-14 and TDD-UM-16: RED proves missing statistic endpoints or frontend current-page-derived cards; GREEN passes backend API count tests and browser network/card assertions. | must |
| AC-UM-10 | User Directory renders display name only in `User`, renders email and phone in new `Contact`, renames `Space Links` to `Space Assignments`, and moves creation to `/admin/users/new`. | automated | TDD-UM-17 and TDD-UM-18: RED proves column/copy drift or missing create sub-page; GREEN passes browser route, column, and create-submit evidence. | must |
| AC-UM-11 | List/search and seed data are consistent with statistic semantics: default list excludes archived users, `status=Archived` returns archived users, `includeArchived=true` returns all statuses, and pagination `total` is the filtered database count. | automated | TDD-UM-15: RED proves query totals or deterministic fixtures are inconsistent; GREEN passes backend list/search/seed tests. | must |

## Business Acceptance

- [ ] AC-UM-01 User management reads and displays users through stable user codes.
- [ ] AC-UM-01 User directory search, status filter, and pagination issue backend list requests with query parameters instead of only filtering local state.
- [ ] AC-UM-01 User directory Export produces a real CSV/download artifact for the current result set.
- [ ] AC-UM-02 Admins can create, update, archive, and restore users with first name, last name, display name, username, email, and optional contact/profile fields.
- [ ] AC-UM-02 Add User opens a usable form, submits the create API, and makes the created user visible.
- [ ] AC-UM-09 User statistic cards are independent backend counts and do not derive from the current page of `users.records`.
- [ ] AC-UM-10 Add User navigates to `/admin/users/new`, where create form behavior lives.
- [ ] AC-UM-11 Search, status, include-archived, and pagination totals are truthful database counts.
- [ ] AC-UM-03 Admins can assign fixed roles to users through many-to-many role-code assignments.
- [ ] AC-UM-04 Admins can assign spaces to users through many-to-many space-code assignments.
- [ ] AC-UM-05 Users can be reset through backend-generated temporary passwords without email delivery or permission checks in MVP.

## UI Acceptance

- [ ] AC-UM-01 `/admin/users` shows user records with `Active` and `Archived` status support and excludes archived users by default.
- [ ] AC-UM-01 `/admin/users` sends `search`, `status`, `current`, and `size` to `GET /api/v1/admin/users` when the operator searches, filters, or paginates.
- [ ] AC-UM-01 `/admin/users` Export produces a CSV/download with stable user-management headers.
- [ ] AC-UM-09 `/admin/users` requests the four statistic endpoints and renders `data.count`.
- [ ] AC-UM-10 `/admin/users` shows only fullname/display name in `User`, email and phone in `Contact`, and `Space Assignments` card copy.
- [ ] AC-UM-10 `/admin/users/new` renders create fields, submits `POST /api/v1/admin/users`, shows visible submit feedback, and returns to the directory.
- [ ] AC-UM-01 `/admin/users/detail?userCode=<userCode>` loads detail by stable `userCode`.
- [ ] AC-UM-02 User detail can edit profile fields and restore archived users to active.
- [ ] AC-UM-03 User detail can replace fixed role-code assignments without embedding role permission matrix behavior.
- [ ] AC-UM-04 User detail can replace space-code assignments without embedding space hierarchy or space-detail management behavior.
- [ ] AC-UM-05 Password reset uses a confirmation dialog, calls the backend reset API, shows a one-time generated temporary-password result dialog, and Copy produces visible feedback.
- [ ] AC-UM-08 Role Management and Space Management remain separate pages/modules.

## Interaction Acceptance Checklist

- IA-UM-01 | source: `/admin/users` search | acceptance: AC-UM-01, AC-UM-08 | red: TDD-UM-09 | status: ready
- IA-UM-02 | source: `/admin/users` status filter | acceptance: AC-UM-01, AC-UM-08 | red: TDD-UM-09 | status: ready
- IA-UM-03 | source: `/admin/users` pagination | acceptance: AC-UM-01, AC-UM-08 | red: TDD-UM-09 | status: ready
- IA-UM-04 | source: `/admin/users` Export | acceptance: AC-UM-01, AC-UM-08 | red: TDD-UM-11 | status: ready
- IA-UM-05 | source: `/admin/users` Add User | acceptance: AC-UM-02, AC-UM-08 | red: TDD-UM-10 | status: ready
- IA-UM-06 | source: `/admin/users` open user detail | acceptance: AC-UM-01, AC-UM-08 | red: TDD-UM-08 | status: retained
- IA-UM-07 | source: directory or detail archive user | acceptance: AC-UM-02, AC-UM-08 | red: TDD-UM-13 | status: ready
- IA-UM-08 | source: user detail update profile | acceptance: AC-UM-02, AC-UM-08 | red: TDD-UM-13 | status: ready
- IA-UM-09 | source: user detail replace role assignments | acceptance: AC-UM-03, AC-UM-08 | red: TDD-UM-13 | status: ready
- IA-UM-10 | source: user detail replace space assignments | acceptance: AC-UM-04, AC-UM-08 | red: TDD-UM-13 | status: ready
- IA-UM-11 | source: user detail restore/reactivate | acceptance: AC-UM-02, AC-UM-08 | red: TDD-UM-13 | status: ready
- IA-UM-12 | source: user detail reset-password confirmation | acceptance: AC-UM-05, AC-UM-08 | red: TDD-UM-12 | status: ready
- IA-UM-13 | source: reset result Copy feedback | acceptance: AC-UM-05, AC-UM-08 | red: TDD-UM-12 | status: ready
- IA-UM-14 | source: `/admin/users` statistic cards | acceptance: AC-UM-09 | red: TDD-UM-16 | status: ready
- IA-UM-15 | source: `/admin/users` directory columns and card copy | acceptance: AC-UM-10 | red: TDD-UM-17 | status: ready
- IA-UM-16 | source: `/admin/users/new` create submit | acceptance: AC-UM-10, AC-UM-02 | red: TDD-UM-18 | status: ready

## Backend Acceptance

- [ ] AC-UM-01 API endpoints match `spec/api-contract.md` for list and detail reads.
- [ ] AC-UM-09 Statistic endpoints match `spec/api-contract.md` and return `data.count`.
- [ ] AC-UM-11 List/search repository totals match filtered database counts before pagination.
- [ ] AC-UM-11 `status=Archived` returns only archived records, while `includeArchived=true` returns all statuses.
- [ ] AC-UM-02 Validation rejects duplicate user codes, duplicate usernames, duplicate emails, missing required profile fields, invalid statuses, and weak passwords.
- [ ] AC-UM-02 Archive is logical deletion and archived users can be restored to `Active`.
- [ ] AC-UM-03 User-role assignments are many-to-many and invalid fixed role codes are rejected.
- [ ] AC-UM-04 User-space assignments are many-to-many and invalid or blank space codes are rejected.
- [ ] AC-UM-05 Passwords are never persisted in plaintext and password hashes use bcrypt in MVP.
- [ ] AC-UM-05 Reset generates a temporary password server-side, stores only its bcrypt hash, marks reset required, and returns the generated value once.
- [ ] AC-UM-07 MVP endpoints do not use managed role/permission data as API authorization gates.

## Database Acceptance

- [ ] AC-UM-02 Database impact is explicitly approved before implementation.
- [ ] AC-UM-02 Schema creates only user-management-owned tables.
- [ ] AC-UM-02 Schema does not create role, space, permission, invitation-email, or audit-log tables.
- [ ] AC-UM-02 All user-management-owned `id` fields are 64-bit integer Snowflake IDs.
- [ ] AC-UM-02 API DTOs serialize 64-bit Snowflake IDs as decimal strings for frontend safety.
- [ ] AC-UM-06 `users.user_code` is unique and stable.
- [ ] AC-UM-02 `users` includes typed profile columns for username, first name, last name, display name, email, phone, avatar, locale, and timezone.
- [ ] AC-UM-03 Assignment tables use `role_code`.
- [ ] AC-UM-04 Assignment tables use `space_code`.
- [ ] AC-UM-05 Password reset event table records reset events without storing plaintext temporary passwords.
- [ ] AC-UM-01 Indexes cover directory, detail, assignment filtering, archive filtering, and password reset lookup.
- [ ] AC-UM-11 Seed data includes enough active and archived users, role assignments, and space assignments for count/pagination consistency tests.

## Integration Acceptance

- [ ] AC-UM-03 User Management references Role Management by `roleCode`.
- [ ] AC-UM-04 User Management references Space Management by `spaceCode`.
- [ ] AC-UM-06 Permission `permissionCode` is documented as an external contract, not implemented here.
- [ ] AC-UM-08 Frontend mocks and adapters use `id`, `userCode`, `username`, `firstName`, `lastName`, `displayName`, `roleCodes`, and `spaceCodes`.
- [ ] AC-UM-08 Current invalid user statuses `Invited` and `Suspended` are removed or mapped out of MVP user mocks.
- [ ] AC-UM-09 Frontend statistic hooks unwrap the backend uniform envelope and read `data.count`.

## Security And Privacy Acceptance

- [ ] AC-UM-07 API permission enforcement is explicitly out of MVP scope.
- [ ] AC-UM-05 Password hashes are not exposed through API responses.
- [ ] AC-UM-05 Generated temporary password values are returned only once.
- [ ] AC-UM-05 MVP password reset has no email verification and no API permission check.
- [ ] AC-UM-05 Email and username identifiers are not written into unstructured logs beyond approved fields.
- [ ] AC-UM-05 Password reset tests cover backend-generated temporary password behavior and one-time display.

## Regression Acceptance

- [ ] AC-UM-08 `make fe-check` passes after frontend model/API changes.
- [ ] AC-UM-08 User routes remain registered in `src/router/routes.tsx` and `src/router/pageRegistry.ts`.
- [ ] AC-UM-08 Role and Space page routes remain independent from User Management.
- [ ] AC-UM-01 Feature docs validation passes for the approved phase.

## Evidence Requirements

| evidence item | acceptance ids | TDD cycle ids | required evidence |
| --- | --- | --- | --- |
| User list/detail API and archived-filter tests. | AC-UM-01 | TDD-UM-01 | RED failure proving missing list/detail behavior; GREEN passing API evidence. |
| User lifecycle domain/repository/API tests. | AC-UM-02 | TDD-UM-02 | RED failure proving missing lifecycle behavior; GREEN passing lifecycle and DTO evidence. |
| Assignment tests for fixed roles and external spaces. | AC-UM-03, AC-UM-04 | TDD-UM-03, TDD-UM-04 | RED failures proving assignment behavior is missing; GREEN passing assignment evidence. |
| Password security tests for bcrypt hash-only persistence and one-time generated temporary password response. | AC-UM-05 | TDD-UM-05 | RED failure proving password reset/security behavior is missing; GREEN passing password/security evidence. |
| Stable code and JSON-safe ID contract tests. | AC-UM-06 | TDD-UM-06 | RED failure proving code contracts are missing; GREEN passing DTO/schema/repository evidence. |
| Authorization-boundary review/test proving no role/permission gate. | AC-UM-07 | TDD-UM-07 | RED or review check proving accidental authorization coupling is detectable; GREEN boundary evidence. |
| Retained browser smoke for `/admin/users`, detail navigation, archive/restore, assignment editing, and password reset dialog. | AC-UM-01, AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-05, AC-UM-08 | TDD-UM-08 | Retained evidence for model/route alignment only; not sufficient for evolved interaction evidence. |
| Browser runtime network/download evidence for directory search, status filter, pagination, and Export. | AC-UM-01, AC-UM-08 | TDD-UM-09, TDD-UM-11 | RED failure proving missing parameterized requests or download; GREEN passing network and CSV artifact evidence. |
| Browser runtime create-flow evidence for Add User. | AC-UM-02, AC-UM-08 | TDD-UM-10 | RED failure proving missing form/API submit; GREEN passing create request and visible-state evidence. |
| Browser runtime reset/copy evidence for backend reset integration. | AC-UM-05, AC-UM-08 | TDD-UM-12 | RED failure proving client-generated reset or invisible Copy feedback; GREEN passing reset API, result, copy-feedback, and no-reveal evidence. |
| Browser runtime detail-mutation evidence for profile, role, space, archive, and restore actions. | AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-08 | TDD-UM-13 | RED failure proving local-only mutation behavior; GREEN passing backend mutation request and response-driven UI evidence. |
| Backend statistic API and list/search consistency evidence. | AC-UM-09, AC-UM-11 | TDD-UM-14, TDD-UM-15 | RED failure proving missing statistic routes or inconsistent filtered totals; GREEN passing count and list/search tests. |
| Frontend statistic-card network evidence. | AC-UM-09 | TDD-UM-16 | RED failure proving cards derive from `users.records`; GREEN browser evidence for four statistic API requests and `data.count` rendering. |
| Directory layout and create sub-page browser evidence. | AC-UM-10 | TDD-UM-17, TDD-UM-18 | RED failure proving User/Contact columns or `/admin/users/new` are missing; GREEN browser evidence for route, create submit, and visible feedback. |
| Final review mapping each AC ID to changed paths, commands, worker worktrees, and residual risks. | AC-UM-01, AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-05, AC-UM-06, AC-UM-07, AC-UM-08 | TDD-UM-01, TDD-UM-02, TDD-UM-03, TDD-UM-04, TDD-UM-05, TDD-UM-06, TDD-UM-07, TDD-UM-08 | GREEN regression evidence and final review evidence. |
