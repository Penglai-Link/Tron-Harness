# API Contract: OpenAPI HMAC Auth Foundation

## API Overview

This feature changes the authentication contract on the existing public `/open-api/v1/*` routes and adds internal Rust service and repository contracts for signed request verification. It does not yet add `/settings/openapi` operator CRUD endpoints.

## Endpoints

All current public routes keep their existing business payloads but now require the same auth headers:

| route | capability | operation |
| --- | --- | --- |
| `GET /open-api/v1/chats/session-groups` | `agent-chat` | `session.read` |
| `POST /open-api/v1/chats/sessions` | `agent-chat` | `session.write` |
| `PATCH /open-api/v1/chats/sessions/{sessionCode}` | `agent-chat` | `session.write` |
| `GET /open-api/v1/chats/sessions/{sessionCode}/messages` | `agent-chat` | `message.read` |
| `POST /open-api/v1/chats/sessions/{sessionCode}/messages` | `agent-chat` | `message.write` |
| `POST /open-api/v1/chats/sessions/{sessionCode}/messages/stream` | `agent-chat` | `stream.consume` |
| `GET /open-api/v1/agents/agui/streams/{sessionCode}/protocol-events` | `agent-chat` | `session.read` |

Required request headers:

- `X-Tron-App-Code`
- `X-Tron-User-Code`
- `X-Tron-Timestamp`
- `X-Tron-Nonce`
- `X-Tron-Signature-Version: TRON-HMAC-SHA256-V1`
- `X-Tron-Signature`

Canonical string:

```text
<HTTP_METHOD>
<NORMALIZED_PATH>
<NORMALIZED_QUERY>
<BODY_SHA256_HEX>
<OPEN_APP_CODE>
<USER_CODE>
<TIMESTAMP>
<NONCE>
```

`X-Tron-Signature` is the lowercase hex digest of `HMAC-SHA256(appKey, canonical_string)`.

## Internal Rust Contract

| type | layer | role |
| --- | --- | --- |
| `OpenApp` | domain | platform external integration aggregate |
| `OpenAppCredential` | domain | active or revoked current-secret record |
| `OpenAppUserGrant` | domain | explicit acting-user binding |
| `OpenAppCapabilityGrant` | domain | capability-operation binding |
| `OpenApiRouteGrant` | domain/application | stable route metadata for authorization |
| `OpenApiAuthService` | application | verifies signature, replay, acting user, and capability |
| `OpenAppRepository` | application port | loads open-app auth state and records nonce usage |

Suggested Rust shape:

```rust
pub struct SignedOpenApiRequest<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub query: &'a str,
    pub body: &'a [u8],
    pub open_app_code: &'a str,
    pub user_code: &'a str,
    pub timestamp: i64,
    pub nonce: &'a str,
    pub signature_version: &'a str,
    pub signature: &'a str,
}

impl OpenApiAuthService {
    pub fn authenticate(
        &self,
        request: SignedOpenApiRequest<'_>,
        route_grant: OpenApiRouteGrant,
    ) -> Result<RequestAuthContext, OpenApiAuthError>;
}
```

## Auth And Permissions

- Formal public OpenAPI auth must not accept `Authorization: Bearer ...`.
- Legacy `X-Tron-App-Key` direct header auth is removed from formal public routes.
- `X-Tron-User-Code` is mandatory and identifies the acting internal user after signature verification.
- Signature verification runs before user-grant and capability-grant authorization.
- Authorized requests only succeed when the open-app is active, the credential is active, the acting user is active and explicitly granted, and the capability-operation grant exists.

## Error Contract

| http_status | code | condition |
| --- | --- | --- |
| `401` | `OPEN_API_UNAUTHORIZED` | missing headers, malformed headers, unsupported signature version, invalid signature |
| `401` | `OPEN_API_TIMESTAMP_INVALID` | timestamp exceeds allowed skew |
| `401` | `OPEN_API_NONCE_REPLAYED` | same nonce already accepted for the open-app code within the replay window |
| `401` | `OPEN_API_APP_DISABLED` | open-app is disabled or current credential is revoked |
| `403` | `OPEN_API_USER_FORBIDDEN` | acting `userCode` is inactive or not explicitly granted |
| `403` | `OPEN_API_OPERATION_FORBIDDEN` | capability-operation grant is missing for the mapped route |

All failures return the standard Tron envelope and stop before handler business logic executes.

## Frontend Backend Boundary

- No frontend page ships in this feature.
- Browser-facing management APIs are deferred to the follow-up `openapi-open-app-management` feature.
- Existing public chat/OpenAPI payload DTOs stay unchanged outside the auth header contract.
- Public adapter packages must move from bearer or raw app-key assumptions to HMAC signing in the route-migration feature.

## Database And Transaction Boundary

- Auth middleware performs read queries for open-app state and grant checks plus one nonce insert or write-once replay reservation.
- Request authorization must remain small and bounded: one open-app lookup, one grant lookup for user, one grant lookup for capability, and one nonce reservation.
- Business handlers should not open or mutate open-app tables directly during normal chat request execution.
- Rotation and management mutations are out of scope for this feature and therefore not part of the runtime auth transaction.

## Mock Or Adapter Expectations

- Backend tests should use deterministic seeded open-app secret fixtures to generate valid HMAC headers.
- Current public route integration tests should switch from legacy header builders to signed-request helpers.
- No frontend mock adapter is required in this feature.
- Later `/settings/openapi` work can reuse the same application service and repository contract rather than duplicating HMAC logic.
