# API Contract: OpenAPI Open-App Management

## API Overview

This feature adds platform admin management APIs for `open-app` lifecycle and grant management, plus a reusable capability catalog endpoint for the settings page. Runtime public `/open-api/v1/*` payloads remain unchanged in this feature.

## Endpoints

| method | route | purpose |
| --- | --- | --- |
| `GET` | `/api/v1/admin/platform/open-apps` | list platform open-app summaries |
| `POST` | `/api/v1/admin/platform/open-apps` | create a platform open-app and reveal the generated `appKey` once |
| `GET` | `/api/v1/admin/platform/open-apps/catalog` | list grouped business openapi capability catalog |
| `GET` | `/api/v1/admin/platform/open-apps/{openAppCode}` | read one open-app detail with current grants |
| `PATCH` | `/api/v1/admin/platform/open-apps/{openAppCode}` | update name, description, and active or disabled status |
| `POST` | `/api/v1/admin/platform/open-apps/{openAppCode}/rotate-key` | rotate the current app key and reveal the new key once |
| `PUT` | `/api/v1/admin/platform/open-apps/{openAppCode}/user-grants` | replace the explicit allowed `userCode` list |
| `PUT` | `/api/v1/admin/platform/open-apps/{openAppCode}/capability-grants` | replace the explicit allowed capability-operation list |

## DTO Shape

Summary record:

```json
{
  "openAppCode": "OPENAPP-...",
  "name": "Agent Chat Partner",
  "description": "CRM callback bridge",
  "status": "Active",
  "appKeyFingerprint": "9f13ab0c7d11",
  "createdAt": "2026-06-11T13:40:12Z",
  "updatedAt": "2026-06-11T13:40:12Z",
  "lastRotatedAt": "2026-06-11T13:40:12Z",
  "userGrantCount": 2,
  "capabilityGrantCount": 3
}
```

Detail record:

```json
{
  "openAppCode": "OPENAPP-...",
  "name": "Agent Chat Partner",
  "description": "CRM callback bridge",
  "status": "Active",
  "appKeyFingerprint": "9f13ab0c7d11",
  "createdAt": "2026-06-11T13:40:12Z",
  "updatedAt": "2026-06-11T13:40:12Z",
  "lastRotatedAt": "2026-06-11T13:40:12Z",
  "userGrants": [
    { "userCode": "USR-0001" }
  ],
  "capabilityGrants": [
    {
      "capabilityCode": "agent-chat",
      "operationCode": "session.read"
    }
  ]
}
```

Create and rotate response:

```json
{
  "openApp": {
    "openAppCode": "OPENAPP-...",
    "name": "Agent Chat Partner",
    "status": "Active",
    "appKeyFingerprint": "9f13ab0c7d11"
  },
  "secretReveal": {
    "openAppCode": "OPENAPP-...",
    "appKey": "generated-once-only-secret",
    "appKeyFingerprint": "9f13ab0c7d11"
  }
}
```

Capability catalog response:

```json
[
  {
    "capabilityCode": "agent-chat",
    "capabilityName": "Agent Chat OpenAPI",
    "description": "External session and message access for agent chat.",
    "operations": [
      {
        "operationCode": "session.read",
        "operationName": "Read Sessions",
        "description": "Read session groups and event feeds.",
        "routes": ["GET /open-api/v1/chats/session-groups"]
      }
    ]
  }
]
```

Replace user grants request:

```json
{
  "userCodes": ["USR-0001", "USR-0042"]
}
```

Replace capability grants request:

```json
{
  "grants": [
    { "capabilityCode": "agent-chat", "operationCode": "session.read" },
    { "capabilityCode": "agent-chat", "operationCode": "message.write" }
  ]
}
```

## Auth And Permissions

- These endpoints are platform admin endpoints and stay behind the existing internal admin auth path.
- Only create or rotate responses may return cleartext `appKey`.
- `PUT` grant endpoints fully replace the current grant set for the given dimension.
- User grants accept only active internal users resolved by explicit `userCode`.
- Capability grants accept only catalog-defined capability-operation pairs.

## Error Contract

| http_status | code | condition |
| --- | --- | --- |
| `400` | `OPEN_APP_NAME_REQUIRED` | create or update request omits a valid name |
| `400` | `OPEN_APP_USER_CODE_INVALID` | user grant payload contains blank or unknown user codes |
| `400` | `OPEN_APP_CAPABILITY_INVALID` | capability or operation is not present in the current catalog |
| `404` | `OPEN_APP_NOT_FOUND` | target `openAppCode` does not exist |
| `409` | `OPEN_APP_NAME_DUPLICATE` | requested open-app name already exists |

All responses keep the standard `{ code, message, data }` envelope.

## Frontend Backend Boundary

- `/settings/openapi` is the only v1 operator UI.
- The page must never display a stored cleartext key after the initial reveal state closes.
- Capability catalog and detail grants come from backend APIs, not duplicated frontend constants.
- The frontend management flow and signed OpenAPI adapter both use `openAppCode` as the public integration identifier.

## Database And Transaction Boundary

- No new tables are required beyond the HMAC foundation schema.
- Management mutations reuse the existing open-app, credential, user-grant, and capability-grant tables.
- Grant replacement should be transactional per dimension so operators do not see partial user or capability state.

## Mock Or Adapter Expectations

- Frontend page may provide a local mock catalog and open-app list for non-API mode.
- Mock payload shapes must match the platform admin API DTOs.
- Browser acceptance runs against the actual frontend route and current page wiring.
