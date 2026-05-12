# Acceptance: User Management Model

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

Acceptance has been revised for a standalone User Management MVP. Role Management, Space Management, Permission Management, invitation email delivery, API permission enforcement, and audit-log pages are outside this feature.

Evolution alignment:

- workflow: Workflow B Feature Evolution
- change_classification: acceptance criteria and test scope structure change
- affected_files: `spec/acceptance.md`, `spec/test-scope.md`
- retained_decisions: AC-UM-01 through AC-UM-08, fixed MVP roles, `Active`/`Archived`, bcrypt, backend-generated temporary password, no API permission enforcement
- superseded_shape: separate RED/GREEN acceptance tables are replaced by TDD cycle references in `spec/test-scope.md`
- pause_resume_decision: no worker dispatch exists yet, so no active worktree or task evidence is superseded

## Acceptance Criteria Matrix

| id | criterion | evidence type | expected evidence | priority |
| --- | --- | --- | --- | --- |
| AC-UM-01 | User directory and user detail can read users by stable `userCode`, list/filter/paginate users, and exclude archived users by default. | automated | TDD-UM-01 plus TDD-UM-08: RED proves stable-code reads or archived filtering are missing; GREEN passes list/detail API and browser smoke evidence. | must |
| AC-UM-02 | User lifecycle supports 64-bit Snowflake IDs, JSON-safe ID serialization, rich profile fields, create, profile update, `Active`, `Archived`, logical deletion, and restore to active. | automated | TDD-UM-02: RED proves lifecycle/model behavior is missing; GREEN passes domain, repository, API, and DTO evidence. | must |
| AC-UM-03 | User-role assignment is many-to-many and uses only fixed role codes `ADMIN`, `WORKSPACE_ADMIN`, `DEVELOPER`, and `VIEWER`. | automated | TDD-UM-03: RED proves role-code assignment behavior is missing; GREEN passes fixed role-code assignment and invalid-code rejection evidence. | must |
| AC-UM-04 | User-space assignment is many-to-many and references external spaces by stable `spaceCode`. | automated | TDD-UM-04: RED proves space-code assignment behavior is missing; GREEN passes external space-code assignment evidence. | must |
| AC-UM-05 | User password management uses bcrypt hashes and supports MVP password reset by generating a temporary password without email or permission checks. | automated | TDD-UM-05 plus TDD-UM-08: RED proves password reset/security behavior is missing; GREEN passes bcrypt, one-time temporary-password, and reset UI evidence. | must |
| AC-UM-06 | User, role, space, and permission integration contracts use stable code fields to avoid ID-change inconsistency. | review + automated | TDD-UM-06: RED proves stable code or JSON-safe ID contract is missing; GREEN passes DTO/schema/repository contract evidence. | must |
| AC-UM-07 | MVP APIs do not use role or permission data for API authorization. | review + automated | TDD-UM-07: RED proves any accidental role/permission gate is detectable; GREEN passes boundary evidence that role assignments are managed data only. | must |
| AC-UM-08 | Existing frontend user-management flow is adjusted to the revised user-only boundary. | automated | TDD-UM-08: RED proves the frontend model or browser flow is still misaligned; GREEN passes `make fe-check` and browser smoke evidence. | should |

## Business Acceptance

- [ ] AC-UM-01 User management reads and displays users through stable user codes.
- [ ] AC-UM-02 Admins can create, update, archive, and restore users with first name, last name, display name, username, email, and optional contact/profile fields.
- [ ] AC-UM-03 Admins can assign fixed roles to users through many-to-many role-code assignments.
- [ ] AC-UM-04 Admins can assign spaces to users through many-to-many space-code assignments.
- [ ] AC-UM-05 Users can be reset through backend-generated temporary passwords without email delivery or permission checks in MVP.

## UI Acceptance

- [ ] AC-UM-01 `/admin/users` shows user records with `Active` and `Archived` status support and excludes archived users by default.
- [ ] AC-UM-01 `/admin/users/detail?user=<userCode>` loads detail by stable `userCode`.
- [ ] AC-UM-02 User detail can edit profile fields and restore archived users to active.
- [ ] AC-UM-03 User detail can replace fixed role-code assignments without embedding role permission matrix behavior.
- [ ] AC-UM-04 User detail can replace space-code assignments without embedding space hierarchy or space-detail management behavior.
- [ ] AC-UM-05 Password reset uses a confirmation dialog and a one-time generated temporary-password result dialog.
- [ ] AC-UM-08 Role Management and Space Management remain separate pages/modules.

## Backend Acceptance

- [ ] AC-UM-01 API endpoints match `spec/api-contract.md` for list and detail reads.
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

## Integration Acceptance

- [ ] AC-UM-03 User Management references Role Management by `roleCode`.
- [ ] AC-UM-04 User Management references Space Management by `spaceCode`.
- [ ] AC-UM-06 Permission `permissionCode` is documented as an external contract, not implemented here.
- [ ] AC-UM-08 Frontend mocks and adapters use `id`, `userCode`, `username`, `firstName`, `lastName`, `displayName`, `roleCodes`, and `spaceCodes`.
- [ ] AC-UM-08 Current invalid user statuses `Invited` and `Suspended` are removed or mapped out of MVP user mocks.

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
| Browser smoke for `/admin/users`, detail navigation, archive/restore, assignment editing, and password reset dialog. | AC-UM-01, AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-05, AC-UM-08 | TDD-UM-08 | RED smoke failure proving frontend mismatch; GREEN passing browser evidence. |
| Final review mapping each AC ID to changed paths, commands, worker worktrees, and residual risks. | AC-UM-01, AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-05, AC-UM-06, AC-UM-07, AC-UM-08 | TDD-UM-01, TDD-UM-02, TDD-UM-03, TDD-UM-04, TDD-UM-05, TDD-UM-06, TDD-UM-07, TDD-UM-08 | GREEN regression evidence and final review evidence. |
