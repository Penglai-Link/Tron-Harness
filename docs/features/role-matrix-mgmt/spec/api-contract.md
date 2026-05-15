# API Contract: Role Matrix Management

## API Overview

This feature introduces repository-owned backend role-matrix APIs. The page reads permission domains and role catalogs from the admin API, derives assignment context from the existing user API, and persists the saved catalog through a whole-snapshot `PUT`.

## Endpoints

### Read Endpoints

- `GET /api/v1/admin/permission-domains`
  - purpose: load the ordered permission-domain headers for the matrix
  - response: uniform envelope whose `data` is the ordered domain list
- `GET /api/v1/admin/roles`
  - purpose: load the baseline role catalog and role permissions
  - response: uniform envelope whose `data` is the saved role catalog
- `GET /api/v1/admin/users`
  - purpose: derive current assignment counts and related user names for a selected role
  - response: existing paged uniform envelope or mock fallback page

### Write Endpoint

- `PUT /api/v1/admin/roles`
  - purpose: replace the saved role catalog with the current role-matrix snapshot
  - request body:
    - `roles[]`
    - per role: `id`, `code`, `name`, `description`, `kind`, `managedBy`, `permissions`, `updatedAt`
  - response: uniform envelope whose `data` is the normalized persisted catalog
  - persistence rule: system roles must remain present; custom roles may be added, edited, or removed through the supplied snapshot

### Frontend Working-Copy Contract

- `RoleMatrixPage.tsx` keeps a page-local working catalog for unsaved edits, dirty tracking, clone/reset/delete flows, and discard behavior.
- In API mode, the `Save` action calls `PUT /api/v1/admin/roles` and reload recovery comes from a fresh `GET /api/v1/admin/roles`.
- In mock mode, the page still behaves without backend writes, but repository-owned persistence lives only behind API mode.

## Auth And Permissions

- No new API auth rule is introduced in this feature.
- Page interactions rely on existing frontend route access only.
- This feature does not add API-side RBAC enforcement.

## Error Contract

- Existing read adapters keep the project-wide envelope shape:
  - `code`
  - `message`
  - `data`
- The new write endpoint uses the same envelope shape on success and validation failure.
- When API mode reads fail, the frontend keeps the existing fallback-to-mock behavior.
- Backend validation rejects:
  - missing required system role codes
  - duplicate role codes
  - duplicate role names
  - invalid permission domains or permission codes
  - invalid role-code characters
- Frontend validation errors use shared toast feedback before submit, for example:
  - empty role name
  - duplicate role code
  - duplicate role name
  - save with invalid draft metadata

## Frontend Backend Boundary

- Backend owns the saved role catalog contract, normalization, validation, seeded system roles, and SQLite persistence.
- Frontend owns draft editing, dirty tracking, custom-role creation, clone, reset, delete, save orchestration, discard, and visible feedback.
- Stable fixed role codes consumed by `user-management-model` remain unchanged by this feature.

## Database And Transaction Boundary

- SQLite persistence is repository-owned through `roles` and `role_permissions`.
- `PUT /api/v1/admin/roles` replaces the catalog inside one repository transaction.
- Discard remains frontend-local because it restores the current page from the last saved baseline.

## Mock Or Adapter Expectations

- `useRoleList` and `usePermissionDomainList` remain the source of baseline read data.
- `useUserList` may be used in read-only mode to derive assignment counts and related names.
- `replaceAdminRoles` is the frontend write adapter for the saved catalog.
- If API payloads omit role metadata beyond the current `RoleRecord` shape, the page must normalize missing fields locally and still behave correctly.
