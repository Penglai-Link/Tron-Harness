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
  "code": "200",
  "message": "success",
  "data": {}
}
```

Each endpoint declares how parameters are transmitted. `parameterLocations` uses `path`, `query`, and `body`; `requestBody: false` means the client must not send a JSON body.

Pagination responses use:

```json
{
  "code": "200",
  "message": "success",
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

Response body:

```json
{
  "code": "200",
  "message": "success",
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
  "code": "200",
  "message": "success",
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
  "code": "200",
  "message": "success",
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
  "code": "200",
  "message": "success",
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
  "code": "200",
  "message": "success",
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
  "code": "200",
  "message": "success",
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
  "code": "200",
  "message": "success",
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
  "code": "200",
  "message": "success",
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
  "code": "200",
  "message": "success",
  "data": {
    "userCode": "USER_MAYA_SINGH",
    "resetCode": "RESET_20260511_0001",
    "temporaryPassword": "g3N8-qP2v-zK9m",
    "passwordResetRequired": true,
    "message": "Temporary password is shown once. Copy it before closing this dialog."
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
| User not found by `userCode`. | 404 `USER_NOT_FOUND` | `{ "code": "USER_NOT_FOUND", "message": "User not found.", "data": { "userCode": "USER_MISSING" } }` | AC-UM-01 |
| Duplicate user code. | 409 `USER_CODE_ALREADY_EXISTS` | `{ "code": "USER_CODE_ALREADY_EXISTS", "message": "User code already exists.", "data": { "userCode": "USER_MAYA_SINGH" } }` | AC-UM-02, AC-UM-06 |
| Duplicate username. | 409 `USERNAME_ALREADY_EXISTS` | `{ "code": "USERNAME_ALREADY_EXISTS", "message": "Username already exists.", "data": { "username": "maya.singh" } }` | AC-UM-02 |
| Duplicate email. | 409 `EMAIL_ALREADY_EXISTS` | `{ "code": "EMAIL_ALREADY_EXISTS", "message": "Email already exists.", "data": { "email": "maya@tron.ai" } }` | AC-UM-02 |
| Required profile field missing. | 422 `INVALID_USER_PROFILE` | `{ "code": "INVALID_USER_PROFILE", "message": "Required profile field is missing.", "data": { "field": "firstName" } }` | AC-UM-02 |
| Invalid user status. | 422 `INVALID_USER_STATUS` | `{ "code": "INVALID_USER_STATUS", "message": "Invalid user status.", "data": { "status": "Suspended" } }` | AC-UM-02 |
| Invalid fixed role code. | 422 `INVALID_ROLE_CODE` | `{ "code": "INVALID_ROLE_CODE", "message": "Invalid fixed role code.", "data": { "roleCode": "OWNER" } }` | AC-UM-03 |
| Invalid or blank space code. | 422 `INVALID_SPACE_CODE` | `{ "code": "INVALID_SPACE_CODE", "message": "Invalid space code.", "data": { "spaceCode": "" } }` | AC-UM-04 |
| Password does not satisfy policy. | 422 `WEAK_PASSWORD` | `{ "code": "WEAK_PASSWORD", "message": "Password does not satisfy policy.", "data": {} }` | AC-UM-05 |
| Archived user cannot perform requested operation. | 409 `USER_ARCHIVED` | `{ "code": "USER_ARCHIVED", "message": "Archived user cannot perform requested operation.", "data": { "userCode": "USER_MAYA_SINGH" } }` | AC-UM-02 |
| Temporary password generation fails. | 500 `PASSWORD_RESET_GENERATION_FAILED` | `{ "code": "PASSWORD_RESET_GENERATION_FAILED", "message": "Temporary password generation failed.", "data": {} }` | AC-UM-05 |

## Frontend Backend Boundary

Frontend responsibilities:

- Render User Management pages only.
- Stop treating Role Matrix, Permission Matrix, Space Management, or Audit Logs as part of this feature.
- Use `userCode`, `roleCodes`, and `spaceCodes` in page state and fixtures.
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
- Return stable pagination envelopes and uniform response bodies.
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
- Update fixed role fixtures to `Admin`, `Workspace Admin`, `Developer`, and `Viewer` labels with stable role codes.
- Keep space assignments as `spaceCode` references, with labels supplied by future Space module fixtures if needed.
- Remove user-management dependencies on role matrix and permission-domain mocks.
- Add user-detail mock state for reset-password confirmation and one-time temporary-password display.

## Open Decisions

- Confirm the adjusted custom TDD scope in `spec/test-scope.md`.
