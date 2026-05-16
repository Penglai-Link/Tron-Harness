# API Contract: Workspace Management

## API Overview

Protected admin routes live under `/api/v1/admin/workspaces` and use the existing auth middleware.

Every route returns the project envelope:

```json
{
  "code": "SUCCESS",
  "message": "ok",
  "data": {}
}
```

Error responses keep `data: {}` and stable workspace-specific error codes.

## Endpoints

### Workspace Endpoints

### `GET /api/v1/admin/workspaces`

Query params:

- `search` optional
- `status` optional: `Active` or `Archived`
- `rootOnly` optional boolean

Response `data`:

- array of workspace tree nodes with nested `children`

Each node exposes:

- `id`
- `workspaceCode`
- `name`
- `description`
- `parentWorkspaceCode`
- `depth`
- `status`
- `childCount`
- `directUserCount`
- `roleGrantCount`
- `effectiveUserCount`
- `children`

### `POST /api/v1/admin/workspaces`

Request body:

- `workspaceCode`
- `name`
- `description` optional
- `parentWorkspaceCode` optional

Creates root workspace when parent is absent.

### `GET /api/v1/admin/workspaces/{workspaceCode}`

Response `data`:

- workspace detail fields
- `lineage`
- `children`
- `directUserCount`
- `roleGrantCount`
- `effectiveUserCount`
- timestamps

### `PATCH /api/v1/admin/workspaces/{workspaceCode}`

Request body:

- `name`
- `description`

### `POST /api/v1/admin/workspaces/{workspaceCode}/move`

Request body:

- `parentWorkspaceCode` nullable

### `POST /api/v1/admin/workspaces/{workspaceCode}/archive`

Response `data`:

- `workspaceCode`
- `status`
- `archivedAt`
- `affectedWorkspaceCount`

### `POST /api/v1/admin/workspaces/{workspaceCode}/restore`

Response `data`:

- `workspaceCode`
- `status`
- `archivedAt`
- `affectedWorkspaceCount`

### Access Endpoints

### `GET /api/v1/admin/workspaces/{workspaceCode}/access/users`

Paginated response records:

- `userCode`
- `displayName`
- `email`
- `isOwner`
- `grantedAt`

### `PUT /api/v1/admin/workspaces/{workspaceCode}/access/users`

Request body:

```json
{
  "users": [
    { "userCode": "USR-0001", "isOwner": true }
  ]
}
```

Full replacement of direct-user grants.

### `GET /api/v1/admin/workspaces/{workspaceCode}/access/roles`

Response records:

- `roleCode`
- `roleName`
- `matchedUserCount`
- `grantedAt`

### `PUT /api/v1/admin/workspaces/{workspaceCode}/access/roles`

Request body:

```json
{
  "roles": [
    { "roleCode": "WORKSPACE_ADMIN" }
  ]
}
```

Full replacement of role-based grants.

### `GET /api/v1/admin/workspaces/{workspaceCode}/access/effective-users`

Paginated response records:

- `userCode`
- `displayName`
- `email`
- `source`
- `matchedRoleCodes`
- `isOwner`

`source` allowed values:

- `DIRECT`
- `ROLE`
- `DIRECT_AND_ROLE`

### Statistics Endpoints

- `GET /api/v1/admin/workspaces/statistics/total-workspaces`
- `GET /api/v1/admin/workspaces/statistics/active-workspaces`
- `GET /api/v1/admin/workspaces/statistics/direct-user-grants`
- `GET /api/v1/admin/workspaces/statistics/role-grants`

Each returns:

```json
{
  "code": "SUCCESS",
  "message": "ok",
  "data": { "count": 0 }
}
```

## Auth And Permissions

- Every workspace endpoint is protected by the same authenticated admin boundary used by other admin modules.
- This feature does not introduce a new permission matrix or workspace-scoped identity source.
- Direct-user and role-grant validation reuse existing users, roles, and role-assignment data from the backend.

## Error Contract

- Invalid or blank `workspaceCode`: `ADMIN_WORKSPACE_CODE_INVALID`
- Duplicate `workspaceCode`: `ADMIN_WORKSPACE_CODE_DUPLICATE`
- Workspace not found: `ADMIN_WORKSPACE_NOT_FOUND`
- Invalid hierarchy move or depth overflow: `ADMIN_WORKSPACE_HIERARCHY_INVALID`
- Archived parent or archived write target: `ADMIN_WORKSPACE_ARCHIVED_INVALID`
- Invalid direct-access user code: `ADMIN_WORKSPACE_ACCESS_USER_INVALID`
- Invalid role-access role code: `ADMIN_WORKSPACE_ACCESS_ROLE_INVALID`
- Duplicate direct user or duplicate role in replacement payload: `ADMIN_WORKSPACE_ACCESS_DUPLICATE`
- Storage failure: `ADMIN_WORKSPACE_STORAGE_FAILED`

## Frontend Backend Boundary

- `/admin/workspace` is the canonical write surface for workspace access configuration.
- User Management may continue to display role and space summary fields, but it must not become the second canonical write path for the new workspace access rules introduced here.

## Database And Transaction Boundary

- Create, update, move, archive, and restore operations execute through the workspace repository against SQLite persistence.
- Direct-user and role-grant replacement are full-replacement writes scoped to one workspace at a time.
- Effective-user reads and statistics endpoints are derived from persisted workspace, direct-user, role, and role-assignment data.

## Mock Or Adapter Expectations

- Frontend mock data under `tron-frontend/src/domain/admin/mock/` must mirror the same route shapes and record fields as the backend API.
- Browser proof may intercept `/api/v1/admin/workspaces/**` while preserving the real request paths, payloads, and envelope structure.
- The page must stay compatible with the existing `VITE_TRON_ADMIN_DATA_SOURCE` switch between mock and API-backed reads.
