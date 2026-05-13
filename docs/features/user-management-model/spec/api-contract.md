# API Contract: User Management Model

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

The API contract is revised around a standalone User Management module. Role, Space, and Permission management APIs are out of scope. User APIs may reference `roleCode` and `spaceCode`, but they do not own those external records.

## API Overview

API namespace: `/api/v1/admin/users`

The API namespace is confirmed for the MVP and should remain `/api/v1/admin/users`.

The backend owns user data invariants, assignment persistence, password hashing, password reset lifecycle, and logical deletion. The frontend owns user management presentation and may redesign current admin mocks/hooks to match this boundary.

MVP permission boundary:

- APIs do not enforce role or permission authorization.
- Managed role codes are assignment data only.
- Permission code design belongs to the future Permission module.

## Endpoints

| method | path | purpose | auth | acceptance ids |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/admin/users` | List users with status, role-code, space-code, search, sort, and pagination filters. | MVP: no permission enforcement | AC-UM-01 |
| GET | `/api/v1/admin/users/statistics/total-users` | Count all persisted users across all statuses. | MVP: no permission enforcement | AC-UM-09 |
| GET | `/api/v1/admin/users/statistics/active-users` | Count persisted users with `Active` status only. | MVP: no permission enforcement | AC-UM-09 |
| GET | `/api/v1/admin/users/statistics/role-assignments` | Count persisted user-role assignment rows. | MVP: no permission enforcement | AC-UM-09 |
| GET | `/api/v1/admin/users/statistics/space-assignments` | Count persisted user-space assignment rows. | MVP: no permission enforcement | AC-UM-09 |
| POST | `/api/v1/admin/users` | Create user with password state, fixed role-code assignments, and space-code assignments. | MVP: no permission enforcement | AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-05 |
| GET | `/api/v1/admin/users/{userCode}` | Read user detail by stable user code. | MVP: no permission enforcement | AC-UM-01 |
| PATCH | `/api/v1/admin/users/{userCode}` | Update user profile fields and status. | MVP: no permission enforcement | AC-UM-02 |
| POST | `/api/v1/admin/users/{userCode}/archive` | Logically delete user by setting status to `Archived`. | MVP: no permission enforcement | AC-UM-02 |
| POST | `/api/v1/admin/users/{userCode}/restore` | Restore archived user to `Active`. | MVP: no permission enforcement | AC-UM-02 |
| PUT | `/api/v1/admin/users/{userCode}/roles` | Replace role-code assignments. | MVP: no permission enforcement | AC-UM-03 |
| PUT | `/api/v1/admin/users/{userCode}/spaces` | Replace space-code assignments. | MVP: no permission enforcement | AC-UM-04 |
| POST | `/api/v1/admin/users/{userCode}/password/reset` | Reset user password without email delivery. | MVP: no permission enforcement | AC-UM-05 |

## Request And Response DTOs

Follow `AGENTS.md` response body format for every endpoint:

```json
{
  "code": "SUCCESS",
  "message": "ok",
  "data": {}
}
```

Each endpoint declares how parameters are transmitted. `parameterLocations` uses `path`, `query`, and `body`; `requestBody: false` means the client must not send a JSON body.

Pagination responses use:

```json
{
  "code": "SUCCESS",
  "message": "ok",
  "data": {
    "current": 1,
    "size": 20,
    "total": 100,
    "records": []
  }
}
```

### GET /api/v1/admin/users

Parameter transmission:

```json
{
  "method": "GET",
  "path": "/api/v1/admin/users",
  "parameterLocations": ["query"],
  "requestBody": false
}
```

Example URL:

```text
/api/v1/admin/users?search=alex&status=Active&roleCode=DEVELOPER&spaceCode=PLATFORM&includeArchived=false&current=1&size=20&sort=displayName&direction=asc
```

Query parameters:

```json
{
  "search": "alex",
  "status": "Active",
  "roleCode": "DEVELOPER",
  "spaceCode": "PLATFORM",
  "includeArchived": false,
  "current": 1,
  "size": 20,
  "sort": "displayName",
  "direction": "asc"
}
```

Query semantics:

- Default list excludes `Archived` users.
- `status=Archived` returns only real archived users.
- `includeArchived=true` returns all statuses unless a `status` filter is also provided.
- `data.total` is the filtered database count before pagination, not the current page length.
- Statistics endpoints use the same persisted repository state as list/search, so counts do not derive from frontend current-page records.

Response body:

```json
{
  "code": "SUCCESS",
  "message": "ok",
  "data": {
    "current": 1,
    "size": 20,
    "total": 1,
    "records": [
      {
        "id": "734102998871252992",
        "userCode": "USER_ALEX_JOHNSON",
        "username": "alex.johnson",
        "firstName": "Alex",
        "lastName": "Johnson",
        "displayName": "Alex Johnson",
        "email": "alex@tron.ai",
        "phone": {
          "countryCode": "+1",
          "number": "5550100"
        },
        "avatarUrl": "https://cdn.tron.local/avatar/alex.png",
        "locale": "en-US",
        "timezone": "America/Los_Angeles",
        "status": "Active",
        "roleCodes": ["DEVELOPER"],
        "spaceCodes": ["PLATFORM"],
        "passwordResetRequired": false,
        "lastActiveAt": "2026-05-11T08:30:00Z",
        "createdAt": "2026-05-11T08:00:00Z",
        "updatedAt": "2026-05-11T08:00:00Z"
      }
    ]
  }
}
```

### GET /api/v1/admin/users/statistics/{statisticName}

Statistic endpoints are intentionally split one endpoint per dashboard card. Each endpoint returns the uniform response envelope, and `data.count` is the canonical frontend value.

Supported paths:

```text
/api/v1/admin/users/statistics/total-users
/api/v1/admin/users/statistics/active-users
/api/v1/admin/users/statistics/role-assignments
/api/v1/admin/users/statistics/space-assignments
```

Response body:

```json
{
  "code": "SUCCESS",
  "message": "ok",
  "data": {
    "count": 7
  }
}
```

### POST /api/v1/admin/users

Parameter transmission:

```json
{
  "method": "POST",
  "path": "/api/v1/admin/users",
  "parameterLocations": ["body"],
  "requestBody": true
}
```

Example URL:

```text
/api/v1/admin/users
```

Body parameters:

```json
{
  "userCode": "USER_MAYA_SINGH",
  "username": "maya.singh",
  "firstName": "Maya",
  "lastName": "Singh",
  "displayName": "Maya Singh",
  "email": "maya@tron.ai",
  "phone": {
    "countryCode": "+1",
    "number": "5550101"
  },
  "avatarUrl": null,
  "locale": "en-US",
  "timezone": "America/Los_Angeles",
  "initialPassword": "temporary-input-optional",
  "passwordResetRequired": true,
  "roleCodes": ["DEVELOPER"],
  "spaceCodes": ["PLATFORM", "DATA"]
}
```

Response body:

```json
{
  "code": "SUCCESS",
  "message": "ok",
  "data": {
    "id": "734103002743185408",
    "userCode": "USER_MAYA_SINGH",
    "username": "maya.singh",
    "firstName": "Maya",
    "lastName": "Singh",
    "displayName": "Maya Singh",
    "email": "maya@tron.ai",
    "phone": {
      "countryCode": "+1",
      "number": "5550101"
    },
    "avatarUrl": null,
    "locale": "en-US",
    "timezone": "America/Los_Angeles",
    "status": "Active",
    "roleAssignments": [
      {
        "roleCode": "DEVELOPER"
      }
    ],
    "spaceAssignments": [
      {
        "spaceCode": "PLATFORM"
      },
      {
        "spaceCode": "DATA"
      }
    ],
    "password": {
      "resetRequired": true,
      "updatedAt": "2026-05-11T08:40:00Z",
      "lastResetAt": null
    },
    "lastActiveAt": null,
    "createdAt": "2026-05-11T08:40:00Z",
    "updatedAt": "2026-05-11T08:40:00Z",
    "archivedAt": null
  }
}
```

### GET /api/v1/admin/users/{userCode}

Parameter transmission:

```json
{
  "method": "GET",
  "path": "/api/v1/admin/users/{userCode}",
  "parameterLocations": ["path"],
  "requestBody": false
}
```

Example URL:

```text
/api/v1/admin/users/USER_MAYA_SINGH
```

Path parameters:

```json
{
  "userCode": "USER_MAYA_SINGH"
}
```

Response body:

```json
{
  "code": "SUCCESS",
  "message": "ok",
  "data": {
    "id": "734103002743185408",
    "userCode": "USER_MAYA_SINGH",
    "username": "maya.singh",
    "firstName": "Maya",
    "lastName": "Singh",
    "displayName": "Maya Singh",
    "email": "maya@tron.ai",
    "phone": {
      "countryCode": "+1",
      "number": "5550101"
    },
    "avatarUrl": null,
    "locale": "en-US",
    "timezone": "America/Los_Angeles",
    "status": "Active",
    "roleAssignments": [
      {
        "roleCode": "DEVELOPER"
      }
    ],
    "spaceAssignments": [
      {
        "spaceCode": "PLATFORM"
      },
      {
        "spaceCode": "DATA"
      }
    ],
    "password": {
      "resetRequired": true,
      "updatedAt": "2026-05-11T08:40:00Z",
      "lastResetAt": null
    },
    "lastActiveAt": null,
    "createdAt": "2026-05-11T08:40:00Z",
    "updatedAt": "2026-05-11T08:40:00Z",
    "archivedAt": null
  }
}
```

### PATCH /api/v1/admin/users/{userCode}

Parameter transmission:

```json
{
  "method": "PATCH",
  "path": "/api/v1/admin/users/{userCode}",
  "parameterLocations": ["path", "body"],
  "requestBody": true
}
```

Example URL:

```text
/api/v1/admin/users/USER_MAYA_SINGH
```

Path parameters:

```json
{
  "userCode": "USER_MAYA_SINGH"
}
```

Body parameters:

```json
{
  "username": "maya.singh",
  "firstName": "Maya",
  "lastName": "Singh",
  "displayName": "Maya Singh",
  "email": "maya@tron.ai",
  "phone": {
    "countryCode": "+1",
    "number": "5550199"
  },
  "avatarUrl": null,
  "locale": "en-US",
  "timezone": "America/Los_Angeles",
  "status": "Active"
}
```

Response body:

```json
{
  "code": "SUCCESS",
  "message": "ok",
  "data": {
    "userCode": "USER_MAYA_SINGH",
    "updatedAt": "2026-05-11T08:45:00Z"
  }
}
```

### POST /api/v1/admin/users/{userCode}/archive

Parameter transmission:

```json
{
  "method": "POST",
  "path": "/api/v1/admin/users/{userCode}/archive",
  "parameterLocations": ["path"],
  "requestBody": false
}
```

Example URL:

```text
/api/v1/admin/users/USER_MAYA_SINGH/archive
```

Path parameters:

```json
{
  "userCode": "USER_MAYA_SINGH"
}
```

Response body:

```json
{
  "code": "SUCCESS",
  "message": "ok",
  "data": {
    "userCode": "USER_MAYA_SINGH",
    "status": "Archived",
    "archivedAt": "2026-05-11T08:50:00Z"
  }
}
```

### POST /api/v1/admin/users/{userCode}/restore

Parameter transmission:

```json
{
  "method": "POST",
  "path": "/api/v1/admin/users/{userCode}/restore",
  "parameterLocations": ["path"],
  "requestBody": false
}
```

Example URL:

```text
/api/v1/admin/users/USER_MAYA_SINGH/restore
```

Path parameters:

```json
{
  "userCode": "USER_MAYA_SINGH"
}
```

Response body:

```json
{
  "code": "SUCCESS",
  "message": "ok",
  "data": {
    "userCode": "USER_MAYA_SINGH",
    "status": "Active",
    "archivedAt": null,
    "updatedAt": "2026-05-11T08:55:00Z"
  }
}
```

### PUT /api/v1/admin/users/{userCode}/roles

Parameter transmission:

```json
{
  "method": "PUT",
  "path": "/api/v1/admin/users/{userCode}/roles",
  "parameterLocations": ["path", "body"],
  "requestBody": true
}
```

Example URL:

```text
/api/v1/admin/users/USER_MAYA_SINGH/roles
```

Path parameters:

```json
{
  "userCode": "USER_MAYA_SINGH"
}
```

Body parameters:

```json
{
  "roleCodes": ["WORKSPACE_ADMIN", "DEVELOPER"]
}
```

Response body:

```json
{
  "code": "SUCCESS",
  "message": "ok",
  "data": {
    "userCode": "USER_MAYA_SINGH",
    "roleAssignments": [
      {
        "roleCode": "WORKSPACE_ADMIN"
      },
      {
        "roleCode": "DEVELOPER"
      }
    ],
    "updatedAt": "2026-05-11T09:00:00Z"
  }
}
```

### PUT /api/v1/admin/users/{userCode}/spaces

Parameter transmission:

```json
{
  "method": "PUT",
  "path": "/api/v1/admin/users/{userCode}/spaces",
  "parameterLocations": ["path", "body"],
  "requestBody": true
}
```

Example URL:

```text
/api/v1/admin/users/USER_MAYA_SINGH/spaces
```

Path parameters:

```json
{
  "userCode": "USER_MAYA_SINGH"
}
```

Body parameters:

```json
{
  "spaceCodes": ["PLATFORM", "DATA"]
}
```

Response body:

```json
{
  "code": "SUCCESS",
  "message": "ok",
  "data": {
    "userCode": "USER_MAYA_SINGH",
    "spaceAssignments": [
      {
        "spaceCode": "PLATFORM"
      },
      {
        "spaceCode": "DATA"
      }
    ],
    "updatedAt": "2026-05-11T09:05:00Z"
  }
}
```

### POST /api/v1/admin/users/{userCode}/password/reset

Parameter transmission:

```json
{
  "method": "POST",
  "path": "/api/v1/admin/users/{userCode}/password/reset",
  "parameterLocations": ["path"],
  "requestBody": false
}
```

Example URL:

```text
/api/v1/admin/users/USER_MAYA_SINGH/password/reset
```

Path parameters:

```json
{
  "userCode": "USER_MAYA_SINGH"
}
```

Response body:

```json
{
  "code": "SUCCESS",
  "message": "ok",
  "data": {
    "userCode": "USER_MAYA_SINGH",
    "temporaryPassword": "TronTmp734103002743185409!",
    "passwordResetRequired": true
  }
}
```

Backward compatibility note:

- Current frontend `UserRecord` should be redesigned from `id/name/roles/workspaces/status` to `id/userCode/username/firstName/lastName/displayName/roleCodes/spaceCodes/status`.
- Backend DTOs also include internal 64-bit Snowflake `id` serialized as a decimal string, but cross-module references should prefer `userCode`.
- Page models should split the old `name` field into `firstName`, `lastName`, and `displayName`.
- Current `Invited` and `Suspended` statuses are invalid for this MVP and must be removed from user mocks or mapped before API mode.

## Auth And Permissions

- MVP does not use API permissions.
- Role and permission data are not used to authorize these endpoints.
- Password reset has no API permission, email verification, or secondary verification in this MVP. Any caller with endpoint/UI access can trigger reset.
- Current actor metadata may be accepted for `createdBy`, `updatedBy`, and reset attribution if a current-user mechanism already exists.

## Error Contract

All errors follow the same uniform response body. HTTP status is carried by the transport status code; business error code is carried by `code`.

| condition | status/code | response shape | acceptance ids |
| --- | --- | --- | --- |
| User not found by `userCode`. | 404 `ADMIN_USER_NOT_FOUND` | `{ "code": "ADMIN_USER_NOT_FOUND", "message": "user was not found", "data": {} }` | AC-UM-01 |
| Duplicate user code. | 409 `ADMIN_USER_CODE_DUPLICATE` | `{ "code": "ADMIN_USER_CODE_DUPLICATE", "message": "userCode already exists", "data": {} }` | AC-UM-02, AC-UM-06 |
| Duplicate username. | 409 `ADMIN_USER_USERNAME_DUPLICATE` | `{ "code": "ADMIN_USER_USERNAME_DUPLICATE", "message": "username already exists", "data": {} }` | AC-UM-02 |
| Duplicate email. | 409 `ADMIN_USER_EMAIL_DUPLICATE` | `{ "code": "ADMIN_USER_EMAIL_DUPLICATE", "message": "email already exists", "data": {} }` | AC-UM-02 |
| Invalid explicit user code. | 400 `ADMIN_USER_CODE_INVALID` | `{ "code": "ADMIN_USER_CODE_INVALID", "message": "userCode must contain uppercase letters, digits, underscore, or hyphen only", "data": {} }` | AC-UM-02, AC-UM-06 |
| Required profile field missing. | 400 `ADMIN_USER_PROFILE_FIELD_REQUIRED` | `{ "code": "ADMIN_USER_PROFILE_FIELD_REQUIRED", "message": "username, firstName, lastName, displayName, and email are required", "data": {} }` | AC-UM-02 |
| Invalid user status. | 400 `ADMIN_USER_STATUS_INVALID` | `{ "code": "ADMIN_USER_STATUS_INVALID", "message": "status must be Active or Archived", "data": {} }` | AC-UM-02 |
| Invalid fixed role code. | 400 `ADMIN_USER_ROLE_CODE_INVALID` | `{ "code": "ADMIN_USER_ROLE_CODE_INVALID", "message": "roleCodes must contain fixed MVP role codes only", "data": {} }` | AC-UM-03 |
| Invalid or blank space code. | 400 `ADMIN_USER_SPACE_CODE_INVALID` | `{ "code": "ADMIN_USER_SPACE_CODE_INVALID", "message": "spaceCodes must be non-empty uppercase stable codes", "data": {} }` | AC-UM-04 |
| Password does not satisfy policy. | 400 `ADMIN_USER_PASSWORD_WEAK` | `{ "code": "ADMIN_USER_PASSWORD_WEAK", "message": "initialPassword does not satisfy the MVP password policy", "data": {} }` | AC-UM-05 |
| Temporary password generation fails. | 500 `ADMIN_USER_PASSWORD_HASH_FAILED` | `{ "code": "ADMIN_USER_PASSWORD_HASH_FAILED", "message": "password hash could not be generated", "data": {} }` | AC-UM-05 |

## Frontend Backend Boundary

Frontend responsibilities:

- Render User Management pages only.
- Read dashboard statistic card values from the four statistic APIs through dedicated client methods/hooks.
- Stop treating Role Matrix, Permission Matrix, Space Management, or Audit Logs as part of this feature.
- Use `userCode`, `roleCodes`, and `spaceCodes` in page state and fixtures.
- Navigate from `/admin/users` Add User to `/admin/users/new`; create form submission belongs to `CreateUserPage`.
- Render directory columns with display name only in `User` and email/phone in `Contact`.
- Display password reset state without exposing password hash or generated temporary password after the one-time result dialog closes.
- Keep mock fixtures deterministic.

Backend responsibilities:

- Generate or validate 64-bit integer Snowflake IDs for user-management-owned records.
- Validate user status, user code uniqueness, username uniqueness, email uniqueness, required profile fields, fixed role codes, space-code shape, and password policy.
- Hash passwords with bcrypt before persistence.
- Generate temporary passwords server-side for password reset.
- Return generated temporary passwords once and never persist plaintext temporary passwords.
- Persist user-role and user-space many-to-many assignments.
- Archive users logically.
- Return stable pagination envelopes, statistic count envelopes, and uniform response bodies.
- Keep list totals and statistic counts backed by filtered database counts.
- Avoid using role or permission data as API authorization gates in the MVP.

Shared type expectation:

- Frontend-local DTOs are acceptable because no generated client exists in the repo today.
- If generated clients are later introduced, `userCode`, `roleCode`, `spaceCode`, and `permissionCode` must remain stable public fields.

## Database And Transaction Boundary

- Create user: one transaction for user row, password hash, role assignments, and space assignments.
- Update user profile/status: one transaction for user row update.
- Archive user: one transaction setting status to `Archived` and `archivedAt`.
- Restore user: one transaction setting status to `Active` and clearing `archivedAt`.
- Replace roles: one transaction replacing assignments by `userCode`.
- Replace spaces: one transaction replacing assignments by `userCode`.
- Reset password: one transaction generating a temporary password, bcrypt-hashing it into `users.password_hash`, setting `password_reset_required = true`, and creating a reset event record.

## Mock Or Adapter Expectations

- Update `tron-frontend/src/domain/admin/mock/data.ts` user fixtures to `Active` and `Archived` only.
- Expand mock and backend seed fixtures enough to prove pagination and count consistency across active users, archived users, role assignments, and space assignments.
- Update fixed role fixtures to `Admin`, `Workspace Admin`, `Developer`, and `Viewer` labels with stable role codes.
- Keep space assignments as `spaceCode` references, with labels supplied by future Space module fixtures if needed.
- Remove user-management dependencies on role matrix and permission-domain mocks.
- Add user-detail mock state for reset-password confirmation and one-time temporary-password display.

## Open Decisions

- None.
