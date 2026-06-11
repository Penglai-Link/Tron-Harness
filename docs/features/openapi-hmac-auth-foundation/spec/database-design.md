# Database Design: OpenAPI HMAC Auth Foundation

## Database Decision

- status: approved
- mandatory_database_change: yes
- rationale: HMAC auth requires durable platform open-app identity, reversible current credential storage, explicit user and capability grants, and nonce replay protection that do not exist in current SQLite schema.

## Existing Schema Impact

- No existing table stores formal external integration credentials or grants.
- Current user tables remain the source of truth for active internal `userCode` resolution.
- Current open-api auth does not persist nonces and cannot distinguish revoked keys from active ones.
- Current public chat data schema remains unchanged; this feature only gates access before handlers execute.

## Proposed Schema

### `platform_open_apps`

| column | type | notes |
| --- | --- | --- |
| `id` | INTEGER PK | internal snowflake identifier |
| `open_app_code` | TEXT UNIQUE | immutable public integration code used by `X-Tron-App-Code` |
| `name` | TEXT | operator-facing unique-ish label |
| `description` | TEXT NULL | optional metadata |
| `status` | TEXT | `ACTIVE` or `DISABLED` |
| `created_at` | INTEGER | epoch seconds |
| `updated_at` | INTEGER | epoch seconds |
| `last_rotated_at` | INTEGER NULL | latest successful credential rotation time |

### `platform_open_app_credentials`

| column | type | notes |
| --- | --- | --- |
| `id` | INTEGER PK | internal snowflake identifier |
| `credential_code` | TEXT PK | internal stable code |
| `open_app_code` | TEXT FK | references `platform_open_apps.open_app_code` |
| `status` | TEXT | `ACTIVE` or `REVOKED` |
| `secret_ciphertext` | TEXT | reversible sealed current secret material |
| `secret_fingerprint` | TEXT | non-sensitive fingerprint or last4 for read models |
| `created_at` | INTEGER | epoch seconds |
| `revoked_at` | INTEGER NULL | null while active |

Unique constraint: one `ACTIVE` credential per `open_app_code`.

### `platform_open_app_user_grants`

| column | type | notes |
| --- | --- | --- |
| `id` | INTEGER PK | internal snowflake identifier |
| `grant_code` | TEXT PK | internal stable code |
| `open_app_code` | TEXT FK | references `platform_open_apps.open_app_code` |
| `user_code` | TEXT | explicit internal acting user |
| `created_at` | INTEGER | epoch seconds |

Unique constraint: one row per `open_app_code + user_code`.

### `platform_open_app_capability_grants`

| column | type | notes |
| --- | --- | --- |
| `id` | INTEGER PK | internal snowflake identifier |
| `grant_code` | TEXT PK | internal stable code |
| `open_app_code` | TEXT FK | references `platform_open_apps.open_app_code` |
| `capability_code` | TEXT | first version uses `agent-chat` |
| `operation_code` | TEXT | examples `session.read`, `session.write`, `message.read`, `message.write`, `stream.consume` |
| `created_at` | INTEGER | epoch seconds |

Unique constraint: one row per `open_app_code + capability_code + operation_code`.

### `platform_open_api_nonces`

| column | type | notes |
| --- | --- | --- |
| `id` | INTEGER PK | internal snowflake identifier |
| `nonce_code` | TEXT PK | internal stable code |
| `open_app_code` | TEXT | public integration code for the replay boundary |
| `nonce` | TEXT | caller-provided nonce |
| `signed_at` | INTEGER | caller timestamp after validation |
| `expires_at` | INTEGER | replay-window expiry for cleanup |
| `created_at` | INTEGER | epoch seconds |

Unique constraint: one row per `open_app_code + nonce`.

## Relationships And Cardinality

- One `OpenApp` has many historical credentials but only one active credential.
- One `OpenApp` has zero or more explicit acting-user grants.
- One `OpenApp` has zero or more capability-operation grants.
- One `OpenApp` may generate many nonce records over time; nonce uniqueness is scoped by `open_app_code`.
- One user can be granted to many open-apps; this feature does not mutate user tables.

## Migration Plan

- Add one new migration file that creates all four platform open-app tables and required indexes.
- Register the migration constant in `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`.
- Add repository constructor migration wiring for the new SQLite open-app repository.
- Seed no open-app data by default; tests create deterministic fixtures explicitly.
- Rotation support is modeled by credential history plus `ACTIVE` uniqueness, so no later schema rewrite is needed for immediate key revocation.

## Query And Performance Plan

- Resolve signed requests by indexed `platform_open_apps.open_app_code` and the single active credential for the resolved `open_app_code`.
- Check explicit acting-user grants through the unique `open_app_code + user_code` index.
- Check capability authorization through the unique `open_app_code + capability_code + operation_code` index.
- Insert nonce records into the `open_app_code + nonce` unique index to reject replays cheaply.
- Delete expired nonce rows opportunistically during auth or on bounded startup maintenance; replay checks remain O(log n) on the unique index.

## Data Governance

- Plaintext `appKey` must never be stored in read models, logs, or later detail APIs.
- HMAC secret storage must be reversible at rest because verification requires raw secret material.
- Disabled open-apps retain grants and credential history for audit but cannot authenticate requests.
- Revoked credentials remain queryable only for internal audit and must never be reactivated.
- Auth failure logs may record `openAppCode`, `userCode`, route capability, and denial reason, but never raw secret or full signature text.

## Test Data And Fixtures

- Repository tests should create one active open-app with one active credential, one granted user, and one granted capability-operation pair.
- Negative fixtures should cover disabled open-app, revoked credential, missing user grant, missing capability grant, and duplicate nonce insert.
- API tests should generate valid and invalid signatures from deterministic secret fixtures.
- No frontend fixture is required in this feature.

## Validation Commands

- `CMD-BE-TEST` proves migrations, repository behavior, and public auth regression through backend automated tests.
- `CMD-BE-CHECK` proves new backend modules compile through the full backend check target.
- `CMD-BE-OPENAPI-FOCUSED` can rerun the public open-api integration subset after the Makefile-first baseline passes.
