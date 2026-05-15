# Database Design: Role Matrix Management

## Database Decision

- status: approved
- approved_by: clouds3n
- approved_at: 2026-05-15
- rationale: this feature now owns repository-backed role-catalog persistence, so the admin role matrix needs dedicated SQLite tables and migrations

## Existing Schema Impact

Existing user-management persistence remains untouched. This feature adds role-catalog persistence alongside it.

## Proposed Schema

- `roles`
  - `id`
  - `role_code`
  - `name`
  - `description`
  - `kind`
  - `managed_by`
  - `created_at`
  - `updated_at`
- `role_permissions`
  - `id`
  - `role_id`
  - `role_code`
  - `permission_domain`
  - `permission_code`
  - `created_at`

## Relationships And Cardinality

- One `roles.id` has many `role_permissions.role_id`.
- One role code may be referenced by zero or more users from the existing user-management dataset, but this feature does not add foreign-key coupling into the user tables.

## Migration Plan

- Apply `20260515_0004_create_role_management_roles.sql`.
- Apply `20260515_0005_create_role_management_permissions.sql`.
- Seed the role catalog with required system roles when the `roles` table is empty.

## Query And Performance Plan

- `GET /api/v1/admin/roles` loads roles ordered by system-first then name and joins permissions in repository code.
- `PUT /api/v1/admin/roles` replaces the full catalog inside one transaction.
- Frontend performance remains bounded by the in-memory working catalog, permission-domain list, and a single user list read used only for assignment counts.

## Data Governance

- No new backend-stored secrets are introduced.
- Stored role metadata contains role codes, names, descriptions, ownership hints, and permission flags only.
- The role catalog must not store user passwords, reset tokens, or profile-private fields.

## Test Data And Fixtures

- Reuse existing admin mock roles and users on the frontend side.
- Backend tests may create temporary SQLite databases and replace the role catalog with a custom role to prove persistence.

## Validation Commands

- `rtk make fe-check`
- `rtk make be-test`
- `rtk npm --prefix tron-frontend exec playwright -- test role-matrix-smoke --reporter=line`
