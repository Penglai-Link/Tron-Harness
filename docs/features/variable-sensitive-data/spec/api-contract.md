# API Contract: Variable Sensitive Data

## API Overview

Variable APIs keep existing platform and workspace CRUD routes where possible, but normal display responses become display-safe. New step-up and reveal endpoints follow the frontend/backend consensus: `POST`, kebab-case URL, no path parameters, request DTOs for more than two fields, and the repo-wide response envelope.

Success responses use:

```json
{
  "code": "SUCCESS",
  "message": "success",
  "data": {}
}
```

## Endpoints

Existing workspace variable list and mutation routes:

- `GET /api/v1/variables/workspaces/{workspaceCode}`
- `POST /api/v1/variables/workspaces/{workspaceCode}`
- `PATCH /api/v1/variables/workspaces/{workspaceCode}/{variableKey}`
- `DELETE /api/v1/variables/workspaces/{workspaceCode}/{variableKey}`

Existing platform variable list and mutation routes:

- `GET /api/v1/admin/platform/variables`
- `POST /api/v1/admin/platform/variables`
- `PATCH /api/v1/admin/platform/variables/{variableKey}`
- `DELETE /api/v1/admin/platform/variables/{variableKey}`

New step-up endpoint:

- `POST /api/v1/auth/verify-step-up-password`

Request:

```json
{
  "password": "current-password"
}
```

Response data:

```json
{
  "stepUpToken": "short-lived-token",
  "expiresAt": "2026-06-02T17:30:00+08:00"
}
```

New workspace reveal endpoint:

- `POST /api/v1/variables/reveal-workspace-variable`

Request:

```json
{
  "workspaceCode": "TRON_CORE",
  "variableKey": "OPENAI_API_KEY",
  "stepUpToken": "short-lived-token"
}
```

Response data:

```json
{
  "value": "plaintext-secret"
}
```

New platform reveal endpoint:

- `POST /api/v1/admin/platform/variables/reveal-platform-variable`

Request:

```json
{
  "variableKey": "OPENAI_API_KEY",
  "stepUpToken": "short-lived-token"
}
```

Response data:

```json
{
  "value": "plaintext-secret"
}
```

Display VO fields:

```json
{
  "key": "OPENAI_API_KEY",
  "value": "********",
  "description": "OpenAI key",
  "source": "Platform",
  "isSensitive": true,
  "valueMasked": true,
  "hasValue": true
}
```

Mutation request extension:

```json
{
  "key": "OPENAI_API_KEY",
  "value": "new-value",
  "description": "OpenAI key",
  "isSensitive": true,
  "valueMode": "REPLACE"
}
```

`valueMode` values:

- `REPLACE`: `value` is required and is written through sensitive or non-sensitive storage semantics.
- `PRESERVE`: no `value` is required; only valid for updates when an existing value exists.

## Auth And Permissions

- Existing variable management permissions continue to govern list, create, update, and delete.
- Reveal requires the same variable scope permission plus a valid current-user step-up token.
- Step-up verifies the password for the authenticated user from request context.
- Step-up does not accept user code in the request body.
- Step-up does not create, refresh, or replace the login session.
- OAuth-only users without a local password receive an unsupported step-up error in v1.

## Error Contract

Stable error codes:

- `VARIABLE_ENCRYPTION_NOT_CONFIGURED`
- `VARIABLE_DECRYPT_FAILED`
- `VARIABLE_STEP_UP_FAILED`
- `VARIABLE_STEP_UP_EXPIRED`
- `VARIABLE_STEP_UP_UNSUPPORTED`
- `VARIABLE_STEP_UP_TOKEN_MISMATCH`
- `VARIABLE_SENSITIVE_DOWNGRADE_VALUE_REQUIRED`
- `VARIABLE_RUNTIME_SECRET_DELIVERY_UNTRUSTED`
- `VARIABLE_SENSITIVE_OVERRIDE_REQUIRED`

All error responses keep:

```json
{
  "code": "VARIABLE_DECRYPT_FAILED",
  "message": "Sensitive value could not be decrypted",
  "data": {}
}
```

## Frontend Backend Boundary

- Frontend sends plaintext only during create, replace, or reveal password submission.
- Frontend never receives ciphertext, nonce, tag, algorithm, or key version.
- Frontend stores revealed plaintext only in local component state.
- Frontend clears revealed plaintext on hide, close, route change, workspace switch, reload, or background refresh.
- Frontend search can match key, description, source labels, workspace labels, and non-sensitive values only.
- Mock records must model sensitive rows as masked records.

## Database And Transaction Boundary

- Backend mutation validates inheritance sensitivity before writing.
- Sensitive writes encrypt before repository persistence.
- Sensitive update with `PRESERVE` keeps the stored encrypted payload and metadata unchanged.
- Sensitive update with `REPLACE` encrypts a new payload using active key version.
- Sensitive-to-non-sensitive update requires a new value and writes plaintext only from the submitted replacement.
- Reveal decrypts after auth, scope, step-up, and variable identity checks.
- Runtime plaintext resolution is backend-only and cannot reuse display DTOs.

## Mock Or Adapter Expectations

- Frontend mock API mirrors display-safe fields for sensitive records.
- Playwright mock data must avoid storing real-looking secret plaintext for sensitive rows.
- Runtime adapter tests must assert untrusted service-mode runtime fails before external POST when sensitive values are present.
- Browser evidence must include clicks, password input, network assertions, visible mask/reveal feedback, and remask behavior.
