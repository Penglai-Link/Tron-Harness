# Feature Spec: User Management Model

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

This Phase 1 draft was revised from the user's confirmation items. The feature is now scoped to the User Management module only. Role Management and Space Management are separate admin modules and are referenced by stable codes, not designed here. The frontend API boundary may be redesigned to match that module split. Confirmed MVP decisions include backend-generated temporary passwords, bcrypt password hashing, `Archived -> Active` restore support, and the `/api/v1/admin/users` API namespace.

Workflow B update on 2026-05-13 reopens the requirement/API/test-plan gate for the user-management consistency refactor. The change adds dedicated backend statistic APIs, aligns list/search query semantics with statistics, expands deterministic seed data, moves create behavior to `/admin/users/new`, and tightens directory table layout.

EV-07 update on 2026-05-14 resumes feedback/loading evaluation after `master` delivered `shared-ui-feedback`. User Management must consume the completed shared `useToast`/`ToastProvider` contract from `docs/features/shared-ui-feedback` and must no longer rely on page-local inline or screen-reader-only notices for operator feedback.

## Feature Identity

- feature_slug: user-management-model
- status: approved_with_defaults
- owner: clouds3n
- source_request: Finish user management model design based on the pages in the frontend-tron.
- main_agent: Codex lead agent

## Requirement Description

Design the backend-ready model and API boundary for the Tron User Management module.

The user management page owns:

- User directory listing, filtering, pagination, export, and navigation to user detail.
- Shared top-right Toast/Snackbar feedback for every user-management interaction that produces operator feedback.
- Loading state for long-running Export and Save-style actions while their download or mutation is in progress.
- Dashboard statistic cards backed by dedicated count APIs instead of the current page of directory records.
- User creation and profile maintenance.
- User logical deletion through `Archived` status.
- User password management and password reset.
- User-to-role assignments through fixed role codes owned by the future Role Management module.
- User-to-space assignments through workspace/space codes owned by the future Space Management module.

The Role Management page, Space Management page, permission matrix, and permission-domain maintenance are not part of this feature.

## Goals And Non-Goals

Goals:

- Use `id` as an internal 64-bit integer Snowflake ID for users and user-management-owned join/reset records.
- Define a persisted user model with stable `user_code` so external references do not break when internal IDs change.
- Store rich basic user profile attributes, including first name, last name, display name, username, email, phone, avatar, locale, and timezone.
- Support only two user statuses in this MVP: `Active` and `Archived`.
- Treat `Archived` as logical deletion. Archived users remain queryable for administrative history but are hidden from default active lists.
- Manage user password hashes and simplified MVP password reset.
- Support many-to-many user-role assignments by `role_code`.
- Support many-to-many user-space assignments by `space_code`.
- Return dedicated statistic counts for total users, active users, role assignment rows, and space assignment rows through one endpoint per card.
- Keep list/search totals and statistics based on the same repository query semantics: default list excludes archived users, `status=Archived` returns real archived users only, and `includeArchived=true` returns all statuses.
- Keep deterministic seed data large enough to verify active/archived pagination, role-assignment counts, and space-assignment counts.
- Support the fixed MVP role set: `Admin`, `Workspace Admin`, `Developer`, and `Viewer`.
- Keep permission and role definitions outside this feature while requiring Role and Permission modules to expose stable code fields.
- Redesign frontend admin user interfaces or DTOs if the current frontend shape mixes user, role, and space responsibilities.

Non-goals:

- Designing Role Management tables, role permission matrices, permission domains, or permission rules.
- Designing Space Management tables, hierarchy, inheritance, or workspace governance.
- Using role or permission data for API authorization in this MVP.
- Invitation email delivery.
- Public self-service profile management.
- Full SSO, MFA, external identity-provider integration, and session lifecycle design.

Assumptions:

- Tron is an internal enterprise console.
- SQLite is the default storage option based on `tron-backend/docs/tech-stack.md`.
- The backend will be Rust and domain-driven, but no Rust source, migrations, entities, or repositories exist yet.
- Role and Space modules will later provide canonical records keyed by `role_code` and `space_code`.
- User Management may validate fixed role codes locally for the MVP, but it must not own role definitions beyond those allowed codes.
- Password hashes use bcrypt in the MVP.

## User Roles And Permissions

MVP role assignment boundary:

- User Management stores assignments to fixed role codes.
- Fixed role codes are:
  - `ADMIN`
  - `WORKSPACE_ADMIN`
  - `DEVELOPER`
  - `VIEWER`
- Role display names are `Admin`, `Workspace Admin`, `Developer`, and `Viewer`.
- Role definitions and permissions belong to Role Management.
- Permission records belong to a future permission model and must have stable `permission_code`; this feature does not create permission tables.
- User Management does not use roles or permissions to authorize API access in this MVP.

Business rules:

- A user has an internal 64-bit integer Snowflake `id`.
- The internal Snowflake `id` is serialized to frontend JSON as a decimal string to avoid JavaScript integer precision loss.
- A user has a stable, unique `user_code`.
- A user has structured profile names: `first_name`, `last_name`, and `display_name`.
- A user has a unique username.
- A user email is unique.
- A user may have zero or more role assignments unless product later requires at least one role.
- A user may have zero or more space assignments unless product later requires at least one space.
- User-role is many-to-many through `user_code` and `role_code`.
- User-space is many-to-many through `user_code` and `space_code`.
- `Archived` users are logically deleted and excluded from default active lists.
- `Archived` users can be restored to `Active`.
- Passwords must be stored as hashes only.
- Password reset must not send email in this MVP.
- Password reset is intentionally simple in this MVP: any caller with access to the endpoint or UI action can trigger reset; no API permission, email verification, or secondary verification is required.

## Page Descriptions

User directory:

- Route: `/admin/users`
- Data source: redesigned `GET /api/v1/admin/users`
- Statistic data sources:
  - `GET /api/v1/admin/users/statistics/total-users`
  - `GET /api/v1/admin/users/statistics/active-users`
  - `GET /api/v1/admin/users/statistics/role-assignments`
  - `GET /api/v1/admin/users/statistics/space-assignments`
- Primary data: user id, user code, username, first name, last name, display name, email, phone, status, role codes, space codes, password state, last active time, created/updated timestamps.
- Interactions: search, status filter, pagination, export, create user, open user detail, archive user.
- Default filter: show `Active` users unless status is explicitly requested.
- Directory table layout:
  - `User` column shows only the full name/display name linked to detail.
  - `Contact` column shows email and phone number.
  - Assignment card label is `Space Assignments`, not `Space Links`.
- Add User button navigates to `/admin/users/new`; the directory page no longer owns the create form.
- Export shows a loading state while the CSV artifact is being prepared and emits success or error feedback through the shared toast API.
- Directory action feedback uses the shared toast API, not an inline page notice or `sr-only` only message.
- Empty state: no matching users after filters.

Create user:

- Route: `/admin/users/new`
- Data source: `POST /api/v1/admin/users`
- Primary data: user code, username, first name, last name, display name, email, optional phone, role codes, and space codes.
- Interactions: enter create form fields, submit create request, show visible success/error feedback, and navigate back to `/admin/users` after a successful create.
- Create submit shows a loading state while the request is in progress and sends success/error feedback through the shared toast API.

User detail:

- Route: `/admin/users/detail?userCode=<userCode>`
- Data source: redesigned `GET /api/v1/admin/users/{userCode}`
- Primary data: user profile fields, status, password state, role assignments, space assignments.
- Interactions: update profile, replace role assignments, replace space assignments, archive/reactivate user, reset password.
- Save Profile, role replacement, space replacement, archive/reactivate, reset confirmation, and copy feedback all send visible operator feedback through the shared toast API.
- Save Profile, role replacement, space replacement, archive/reactivate, and reset confirmation show an action-local loading state while the backend mutation is in progress.
- Password reset interaction:
  - The user detail page shows a reset-password action.
  - Clicking reset opens a confirmation dialog explaining that the backend will generate a temporary password and no email verification will be sent.
  - Confirming calls `POST /api/v1/admin/users/{userCode}/password/reset`.
  - The backend generates a temporary password, stores only its bcrypt hash, sets `password_reset_required = true`, records a reset event, and returns the temporary password once.
  - The UI shows the generated temporary password in a result dialog with a copy action. After the dialog is closed, the UI must not be able to reveal it again.
  - The operator manually shares the temporary password outside Tron.

Out-of-scope admin pages:

- Role Management page: owns role list, role definitions, permission matrix, and fixed/custom role future expansion.
- Space Management page: owns spaces/workspaces, hierarchy, inheritance, and space metadata.
- Permission Management: owns permission records and `permission_code`.

## Interactions

- IA-UM-01: `/admin/users` search sends `search` to `GET /api/v1/admin/users` and renders the response records.
- IA-UM-02: `/admin/users` status filter sends `status` or explicit archive inclusion intent to `GET /api/v1/admin/users`.
- IA-UM-03: `/admin/users` pagination sends `current` and `size` to `GET /api/v1/admin/users` and renders the paged response envelope.
- IA-UM-04: `/admin/users` Export creates a CSV/download artifact for the current directory result set.
- IA-UM-05: `/admin/users` Add User navigates to `/admin/users/new`.
- IA-UM-06: `/admin/users` opens `/admin/users/detail?userCode=<userCode>` using a stable user code.
- IA-UM-07: user archive from directory or detail calls the archive endpoint and refreshes visible state.
- IA-UM-08: user detail profile update calls `PATCH /api/v1/admin/users/{userCode}`.
- IA-UM-09: user detail role-code replacement calls `PUT /api/v1/admin/users/{userCode}/roles`.
- IA-UM-10: user detail space-code replacement calls `PUT /api/v1/admin/users/{userCode}/spaces`.
- IA-UM-11: user detail restore/reactivate calls the restore endpoint and refreshes visible state.
- IA-UM-12: user detail reset-password confirmation calls `POST /api/v1/admin/users/{userCode}/password/reset`.
- IA-UM-13: reset result Copy writes the one-time temporary password and renders visible operator feedback.
- IA-UM-14: `/admin/users` statistic cards call their four dedicated statistic APIs and render `data.count` values.
- IA-UM-15: `/admin/users` directory renders display name only in `User`, email/phone in `Contact`, and `Space Assignments` copy.
- IA-UM-16: `/admin/users/new` submits `POST /api/v1/admin/users`, handles the uniform response body, and returns visible success feedback.
- IA-UM-17: user-management interaction feedback uses `useToast` from `shared-ui-feedback` and appears as a top-right auto-dismiss toast.
- IA-UM-18: `/admin/users` Export shows a loading state until the CSV download is prepared and then emits shared toast success/error feedback.
- IA-UM-19: `/admin/users/new` Create User shows a loading state while submitting and emits shared toast success/error feedback.
- IA-UM-20: user detail Save Profile, role/space changes, archive/restore, reset confirmation, and Copy show action feedback through shared toast; mutation buttons expose a loading state while pending.

## Domain Model

Bounded context:

- User Management is the only bounded context in this feature.
- Role Management, Space Management, and Permission Management are external bounded contexts.
- This feature may reference external stable codes, but it must not create Role, Space, Workspace, Permission, or role-permission domain models.

Aggregate roots:

- `User`: aggregate root for platform identity, profile, lifecycle status, password state, role-code assignments, and space-code assignments.
- `UserPasswordResetEvent`: append-only record for a completed backend-generated temporary password reset. It belongs to the User Management bounded context but is not used to recover plaintext credentials.

Entities inside the `User` aggregate:

- `UserProfile`: structured profile data with username, first name, last name, display name, email, optional phone, optional avatar, locale, and timezone.
- `UserRoleAssignment`: assignment edge from `userCode` to external `roleCode`; it does not contain role definitions or permissions.
- `UserSpaceAssignment`: assignment edge from `userCode` to external `spaceCode`; it does not contain space metadata or hierarchy.
- `UserPasswordState`: password hash metadata, hash algorithm, password update time, and reset-required flag.

Value objects:

- `SnowflakeId`: internal 64-bit integer ID for User Management owned records; generated by the common Snowflake ID capability and serialized to frontend JSON as a decimal string.
- `UserCode`: stable external user identifier used by APIs and cross-module references.
- `Username`: unique login/account name exposed in User Management.
- `PersonalName`: first name, last name, and display name.
- `EmailAddress`: normalized unique email address.
- `PhoneNumber`: optional country/region code plus local number.
- `UserStatus`: `Active` or `Archived`.
- `RoleCode`: external Role Management reference; MVP accepted values are `ADMIN`, `WORKSPACE_ADMIN`, `DEVELOPER`, and `VIEWER`.
- `SpaceCode`: external Space Management reference.
- `PasswordHash`: bcrypt hash and algorithm metadata; plaintext passwords are never a value object stored by the domain.
- `ResetCode`: stable reset event code for password reset records.

Domain services and policies:

- `CommonSnowflakeIdProvider`: dependency on `docs/features/common-snowflake-id` for generating 64-bit IDs for users, assignments, and reset events. User Management does not own the generator implementation.
- `UserUniquenessPolicy`: enforces unique `userCode`, `username`, and email through repository/database checks.
- `UserProfilePolicy`: validates required profile fields and normalizes username/email where applicable.
- `UserStatisticsPolicy`: counts total users, active users, role assignment rows, and space assignment rows from persisted repository state.
- `FixedRoleCodePolicy`: validates MVP role codes without owning role definitions.
- `SpaceCodePolicy`: validates only the shape and non-empty value of `spaceCode`; canonical space existence belongs to the future Space module.
- `PasswordHashingService`: hashes initial and reset passwords with bcrypt.
- `TemporaryPasswordService`: generates a temporary password for reset and returns it only in the reset response.
- `UserLifecyclePolicy`: archives users logically and allows restoring archived users to active.

Domain events and lifecycle signals:

- `UserCreated`
- `UserProfileUpdated`
- `UserArchived`
- `UserRestored`
- `UserRoleAssignmentsReplaced`
- `UserSpaceAssignmentsReplaced`
- `UserPasswordReset`

These events are domain signals for tests, handlers, or later audit integration. This feature does not design an Audit Log module or event bus persistence.

External references:

- `RoleCode` references the future Role Management module.
- `SpaceCode` references the future Space Management module.
- `PermissionCode` belongs to the future Permission module and is not stored by User Management in this MVP.

Relationships and cardinality:

- One `User` has zero or more `UserRoleAssignment` records.
- One external `RoleCode` can be assigned to many users.
- One `User` has zero or more `UserSpaceAssignment` records.
- One external `SpaceCode` can be assigned to many users.
- One `User` has zero or more `UserPasswordResetEvent` records.

Invariants:

- `User.id`, assignment IDs, and reset event IDs are internal 64-bit Snowflake IDs.
- `UserCode`, `Username`, and `EmailAddress` are unique.
- `UserStatus` is only `Active` or `Archived`.
- `Archived` means logical deletion; archived users remain stored and can be restored to `Active`.
- Archive and restore do not delete role-code or space-code assignments.
- Password hashes use bcrypt in the MVP and are never returned by API DTOs.
- Generated temporary passwords are returned once and are never stored as plaintext.
- Role and permission data are never used as API authorization gates in this MVP.

State transitions:

- User creation initializes a user as `Active` unless a later product decision allows direct archived creation.
- User lifecycle supports `Active -> Archived`.
- User lifecycle supports `Archived -> Active`.
- Password reset is a command on a user that replaces `UserPasswordState`, marks password reset required, and appends a completed `UserPasswordResetEvent`.

## Module And Boundary Map

Frontend modules currently involved:

- `tron-frontend/src/domain/admin/pages/UsersPage.tsx`
- `tron-frontend/src/domain/admin/pages/CreateUserPage.tsx`
- `tron-frontend/src/domain/admin/pages/UserDetailsPage.tsx`
- `tron-frontend/src/shared/ui/Toast.tsx`
- `tron-frontend/src/shared/index.ts`
- `tron-frontend/src/domain/admin/model/AdminModel.ts`
- `tron-frontend/src/domain/admin/api/adminApi.ts`
- `tron-frontend/src/domain/admin/hooks/useUserList.ts`
- `tron-frontend/src/domain/admin/mock/data.ts`
- `tron-frontend/src/router/routes.tsx`
- `tron-frontend/src/router/pageRegistry.ts`

Frontend modules that should move out of User Management scope:

- `tron-frontend/src/domain/admin/pages/RoleMatrixPage.tsx`
- Permission-domain hooks and mocks tied to role matrix behavior.
- Future Space Management page components.

Backend modules to be created in later phases:

- Rust user domain models, repositories, HTTP handlers, validation services, password hashing/reset service, and SQLite migrations.
- Integration adapters for external Role and Space modules through `role_code` and `space_code`.
- Repository statistics methods and HTTP routes for count-only dashboard cards.

Shared boundary:

- Frontend User Management DTOs should use `userCode`, `roleCodes`, and `spaceCodes`.
- User APIs must not expose password hashes.
- Snowflake ID generation is owned by the common capability in `docs/features/common-snowflake-id`; User Management consumes it through a provider interface.
- Role and Space display names can be resolved by separate modules or supplied as denormalized read labels later.

## Repository Findings

- `Makefile` exposes frontend workflow targets: `fe-install`, `fe-dev`, `fe-check`, `fe-build`, `fe-preview`, and `fe-clean`.
- `tron-frontend/package.json` uses Vite, React 19, TypeScript, React Router, Playwright, i18next, lucide-react, and Zustand.
- Current frontend admin routes include `/admin/users`, `/admin/users/detail`, `/admin/roles`, and `/admin/audit-logs`.
- Current frontend mock `UserRecord` uses `id`, `name`, `email`, `roles`, `workspaces`, `status`, and `lastActive`.
- The current frontend mixes user, role, permission, and audit concerns under one admin domain. The revised model splits User Management, Role Management, and Space Management into independent modules.
- `docs/tech-stack.md` describes a Rust + React monorepo and backend-served production runtime.
- `tron-backend/docs/tech-stack.md` says the backend is Rust with SQLite by default.
- `tron-backend` now has initialized Rust backend scaffolding and shared Snowflake capability, but no user-management domain module, migrations, or user API handlers yet.
- `master` contains the completed `shared-ui-feedback` feature with `ToastProvider`, `useToast`, top-right toast rendering, default auto-dismiss, and manual dismissal. User-management feedback/loading work must consume that canonical implementation.

## Feature Dependencies

| dependency feature name | feature slug | evidence from feature-spec.md | required contract or artifact | impact on current feature | decision/status |
| --- | --- | --- | --- | --- | --- |
| Common Snowflake ID Generation | common-snowflake-id | `docs/features/common-snowflake-id/spec/feature-spec.md` confirms shared `SnowflakeIdProvider`, YAML config policy, and decimal-string ID serialization contract. | `tron-backend/src/shared/id/*` provider + serialization policy + test adapter boundary. | User, assignment, and password-reset records must consume this provider for all internal 64-bit IDs and keep JSON-safe serialization. | dependency_confirmed |
| Init Rust Backend | init-rust-backend | `docs/features/init-rust-backend/spec/feature-spec.md` confirms backend runtime skeleton, `ApiResponse<T>`, and Makefile backend command rails. | `tron-backend` app/runtime scaffolding, root `Makefile` `be-*` targets, shared response envelope. | User management handlers, repository wiring, and regression commands depend on the initialized backend runtime baseline. | dependency_confirmed |
| Shared UI Feedback | shared-ui-feedback | `docs/features/shared-ui-feedback/spec/feature-spec.md` confirms a shared `ToastProvider`, `useToast` API, top-right placement, default auto-dismiss, and dismiss control. | `tron-frontend/src/shared/ui/Toast.tsx`, root provider mount, and shared export from `tron-frontend/src/shared/index.ts`. | User-management feedback and loading work must use the shared toast primitive rather than page-local notices. | dependency_confirmed_from_master |

## Open Decisions

- None.
