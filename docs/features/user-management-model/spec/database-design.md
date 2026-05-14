# Database Design: User Management Model

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

The database design is revised to own only User Management persistence. Role, Space, and Permission modules are external and referenced by stable codes. This feature creates no role table, no workspace table, no permission table, and no audit-log table.

## Database Decision

- status: approved
- impact: yes
- approved_by: clouds3n
- approved_at: 2026-05-12

SQLite remains the recommended MVP database based on current backend tech-stack docs. All user-management-owned `id` fields use 64-bit integer Snowflake IDs stored as SQLite `INTEGER`; API JSON serializes those IDs as decimal strings to avoid JavaScript precision loss.

## Existing Schema Impact

No user-management schema, migrations, entities, repositories, or caches were found yet. Current backend code only contains platform bootstrap and shared Snowflake capability.

Expected downstream consumers:

- User directory and user detail pages.
- Future Role Management module through `role_code` references.
- Future Space Management module through `space_code` references.
- Future Permission module through `permission_code` references, outside this feature.

Out of scope:

- Role definition tables.
- Space/workspace definition tables.
- Permission definition tables.
- Audit log tables.
- Invitation/email tables.

## Proposed Schema

### Table: users

| field | SQLite type | required | constraints | description |
| --- | --- | --- | --- | --- |
| id | INTEGER | yes | primary key; 64-bit Snowflake ID | Internal immutable user identifier. |
| user_code | TEXT | yes | unique; stable; uppercase snake-case recommended | Stable external user code used by APIs and cross-module references. |
| username | TEXT | yes | unique; case-insensitive unique index recommended | Login/account name shown in admin tools. |
| first_name | TEXT | yes | length 1..100 | User given name. |
| last_name | TEXT | yes | length 1..100 | User family name. |
| display_name | TEXT | yes | length 1..200 | Display label; defaults to first name + last name when not supplied. |
| email | TEXT | yes | unique; normalized lowercase | Primary email address. |
| phone_country_code | TEXT | no | length 1..8 | Optional phone country/region code, for example `+86`. |
| phone_number | TEXT | no | length 1..32 | Optional phone number. |
| avatar_url | TEXT | no | URL length limit decided in implementation | Optional avatar URL. |
| locale | TEXT | no | BCP 47 locale, for example `en-US` or `zh-CN` | Preferred UI locale. |
| timezone | TEXT | no | IANA timezone, for example `Asia/Shanghai` | Preferred timezone for user-facing dates. |
| status | TEXT | yes | check in `active`, `archived` | User lifecycle status; `archived` means logical delete. |
| password_hash | TEXT | yes | never returned by API | Password hash only; no plaintext persistence. |
| password_hash_algorithm | TEXT | yes | `bcrypt` for MVP | Password hashing algorithm identifier and optional parameter version. |
| password_updated_at | INTEGER | no | Unix epoch milliseconds | Last password hash update time. |
| password_reset_required | INTEGER | yes | 0 or 1; default 1 | Boolean reset-required flag stored as SQLite integer. |
| last_active_at | INTEGER | no | Unix epoch milliseconds | Last observed activity time. |
| archived_at | INTEGER | no | Unix epoch milliseconds; required when status is `archived` | Logical deletion timestamp. |
| created_at | INTEGER | yes | Unix epoch milliseconds | Creation time. |
| updated_at | INTEGER | yes | Unix epoch milliseconds | Last update time. |
| created_by | INTEGER | no | 64-bit Snowflake user ID when available | Operator that created the user. |
| updated_by | INTEGER | no | 64-bit Snowflake user ID when available | Operator that last updated the user. |
| version | INTEGER | yes | default 0 | Optimistic-lock version. |

Indexes:

- `uk_users_user_code` unique on `user_code`.
- `uk_users_username` unique on normalized `username`.
- `uk_users_email` unique on normalized `email`.
- `idx_users_status_display_name` on `status`, `display_name`.
- `idx_users_last_active_at` on `last_active_at`.

### Table: user_role_assignments

| field | SQLite type | required | constraints | description |
| --- | --- | --- | --- | --- |
| id | INTEGER | yes | primary key; 64-bit Snowflake ID | Internal assignment ID. |
| user_id | INTEGER | yes | references `users.id` | Internal user reference for joins. |
| user_code | TEXT | yes | references current `users.user_code` value | Stable user code snapshot/reference. |
| role_code | TEXT | yes | check in `ADMIN`, `WORKSPACE_ADMIN`, `DEVELOPER`, `VIEWER` | External Role module code. |
| assigned_at | INTEGER | yes | Unix epoch milliseconds | Assignment time. |
| assigned_by | INTEGER | no | 64-bit Snowflake user ID when available | Operator that assigned the role. |

Indexes:

- `uk_user_role_assignments_user_role` unique on `user_id`, `role_code`.
- `idx_user_role_assignments_user_code` on `user_code`.
- `idx_user_role_assignments_role_code` on `role_code`.

### Table: user_space_assignments

| field | SQLite type | required | constraints | description |
| --- | --- | --- | --- | --- |
| id | INTEGER | yes | primary key; 64-bit Snowflake ID | Internal assignment ID. |
| user_id | INTEGER | yes | references `users.id` | Internal user reference for joins. |
| user_code | TEXT | yes | references current `users.user_code` value | Stable user code snapshot/reference. |
| space_code | TEXT | yes | non-empty; external Space module code | External Space module code. |
| assigned_at | INTEGER | yes | Unix epoch milliseconds | Assignment time. |
| assigned_by | INTEGER | no | 64-bit Snowflake user ID when available | Operator that assigned the space. |

Indexes:

- `uk_user_space_assignments_user_space` unique on `user_id`, `space_code`.
- `idx_user_space_assignments_user_code` on `user_code`.
- `idx_user_space_assignments_space_code` on `space_code`.

### Table: user_password_reset_events

| field | SQLite type | required | constraints | description |
| --- | --- | --- | --- | --- |
| id | INTEGER | yes | primary key; 64-bit Snowflake ID | Internal reset event ID. |
| reset_code | TEXT | yes | unique; stable reset event code | Public-safe reset event identifier. |
| user_id | INTEGER | yes | references `users.id` | User being reset. |
| user_code | TEXT | yes | references current `users.user_code` value | Stable user code snapshot/reference. |
| password_hash_algorithm | TEXT | yes | `bcrypt` for MVP | Algorithm used for the generated temporary password hash. |
| password_reset_required_after | INTEGER | yes | 0 or 1; expected 1 | Records whether reset forced next-login password change. |
| temporary_password_returned | INTEGER | yes | 0 or 1; expected 1 | Records that the generated temporary password was returned once in the reset response. |
| created_at | INTEGER | yes | Unix epoch milliseconds | Creation time. |
| created_by | INTEGER | no | 64-bit Snowflake user ID when available | Caller/operator that triggered reset; nullable because MVP allows any caller. |

Indexes:

- `uk_user_password_reset_events_reset_code` unique on `reset_code`.
- `idx_user_password_reset_events_user_code` on `user_code`.
- `idx_user_password_reset_events_created_at` on `created_at`.

External code contracts:

| external module | required stable code | notes |
| --- | --- | --- |
| Role Management | role_code | MVP accepted values: `ADMIN`, `WORKSPACE_ADMIN`, `DEVELOPER`, `VIEWER`. |
| Space Management | space_code | User Management stores references only. |
| Permission Management | permission_code | Required by future permission records, not stored in this module. |

## Relationships And Cardinality

- `users` to `user_role_assignments`: one-to-many.
- `role_code` to users: many-to-many through `user_role_assignments`.
- `users` to `user_space_assignments`: one-to-many.
- `space_code` to users: many-to-many through `user_space_assignments`.
- `users` to `user_password_reset_events`: one-to-many.

Invariants:

- Every user-management-owned `id` is a 64-bit integer Snowflake ID.
- `user_code` is stable and unique.
- `username` is stable enough for login and unique.
- `first_name`, `last_name`, and `display_name` are required for the MVP.
- `email` is unique.
- User status is only `active` or `archived`.
- Archived users are logical deletes and remain stored.
- Archived users can be restored to active.
- Password hash is never exposed through API DTOs.
- Password hashes use bcrypt in the MVP.
- Password reset generates a temporary password server-side, stores only its bcrypt hash in `users.password_hash`, marks `password_reset_required`, and returns the temporary password once.
- User role assignments must use fixed MVP role codes.
- User space assignments must use `space_code`; this module does not create or mutate spaces.

## Migration Plan

Recommended migration names:

- `20260511_0001_create_user_management_users`
- `20260511_0002_create_user_management_assignments`
- `20260511_0003_create_user_password_reset_events`

Order:

1. Create `users`.
2. Create `user_role_assignments`.
3. Create `user_space_assignments`.
4. Create `user_password_reset_events`.
5. Seed development users with fixed role-code and space-code assignments only if demo fixtures are required.

Rollback:

- Drop password reset events first.
- Drop assignment tables before users.
- Do not delete production archived-user data without an approved backup/export plan.

Compatibility:

- Current frontend mocks use `id`, `roles`, and `workspaces`; adapters should migrate page-facing records to `userCode`, `username`, `roleCodes`, and `spaceCodes`.
- Existing `Invited` and `Suspended` frontend statuses must be removed or mapped before API-mode integration.

## Query And Performance Plan

Primary query paths:

- User directory: filter by `status`, `role_code`, `space_code`, search by display name, username, email, or user code.
- User detail: fetch by `user_code` with role-code and space-code assignments.
- Password management: fetch current password state and recent reset events by `user_code`.

Index plan:

- `users(user_code)` unique.
- `users(email)` unique.
- `users(status, display_name)`.
- `users(last_active_at)`.
- `user_role_assignments(user_code)`.
- `user_role_assignments(role_code)`.
- `user_space_assignments(user_code)`.
- `user_space_assignments(space_code)`.
- `user_password_reset_events(user_code)`.
- `user_password_reset_events(created_at)`.

Expected cardinality:

- Users: hundreds to low thousands for the initial internal console.
- Role assignments: users multiplied by a small fixed role set.
- Space assignments: users multiplied by assigned workspaces/spaces.
- Password reset events: bounded by retention cleanup.

## Data Governance

- Email and username identifiers are personal data.
- Password hashes and generated temporary credentials are sensitive data.
- Plaintext passwords must never be stored.
- A generated temporary password is returned once in the reset response and must not be recoverable afterward.
- Archived users remain stored for consistency and may be excluded from default lists.
- User Management does not use role or permission codes as API authorization gates in the MVP.
- MVP password reset intentionally has no email verification, no reset token verification, and no API permission gate.
- Retention for password reset event records requires a cleanup policy before production hardening.

## Test Data And Fixtures

Seed fixtures should be revised from current frontend mock data:

- Users: sample active users plus at least one archived user.
- Statuses: `Active`, `Archived`.
- Fixed role codes: `ADMIN`, `WORKSPACE_ADMIN`, `DEVELOPER`, `VIEWER`.
- Space codes: stable sample values such as `ENTERPRISE`, `PLATFORM`, `DATA`, and `SUPPORT`.
- Password state: one normal user, one `password_reset_required` user, one bcrypt-hashed generated temporary password, and one archived user.

Factories should support:

- User with no role assignments.
- User with multiple role assignments.
- User with multiple space assignments.
- Archived user with retained assignments.
- Password reset event with `temporary_password_returned = 1`.
- Archived user restored to active.

## Validation Commands

| command | purpose | expected evidence |
| --- | --- | --- |
| `rtk make be-test` | Backend regression for user model, repository, assignment, archive, and password reset behavior once implemented. | Passing backend tests. |
| `rtk make fe-check` | Frontend typecheck/build gate after user-model API adapter changes. | Typecheck and build success. |
| `rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/user-management-model --phase spec --spec-stage database` | Validate Phase 1B database design gate. | Validation passes. |

## Open Decisions

- None.
