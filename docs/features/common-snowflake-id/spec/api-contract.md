# API Contract: Common Snowflake ID Generation

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: phase_1c_confirmed

This Phase 1C draft treats `common-snowflake-id` as an internal backend capability, not an externally callable HTTP API. The public contract is a Rust module boundary: business modules call a Tron-owned provider trait, configuration is loaded from backend YAML, and DTO serializers expose 64-bit IDs as decimal strings for JavaScript safety.

## API Overview

External HTTP API:

- No public REST endpoint is introduced.
- No admin page or settings API is introduced.
- ID generation must not be exposed as a remote service in the MVP.

Internal backend API:

- `SnowflakeIdProvider` is the main provider trait consumed by backend modules.
- `SnowflakedSnowflakeIdProvider` is the adapter around the confirmed `snowflaked` crate.
- `FixedSnowflakeIdProvider` is a deterministic test provider.
- `YamlGeneratorConfigLoader` loads `id.snowflake.instance_id` and `id.snowflake.epoch_ms` from backend YAML.
- `IdSerializationPolicy` defines decimal-string JSON serialization for API DTO fields that contain `SnowflakeId`.

API response format:

- The root `AGENTS.md` uniform response body applies to HTTP endpoints.
- This feature has no HTTP endpoint, so it does not return `{ code, message, data }` directly.
- Consuming HTTP APIs must still use the uniform response body and serialize `SnowflakeId` fields inside `data` as decimal strings.

## Endpoints

| method | path | purpose | auth | acceptance ids |
| --- | --- | --- | --- | --- |
| internal | `SnowflakeIdProvider::next_id` | Generate one signed 64-bit Snowflake ID for backend-owned records. | Backend module dependency only; no user auth boundary. | AC-SNOWFLAKEID-01, AC-SNOWFLAKEID-03 |
| internal | `YamlGeneratorConfigLoader::load` | Load and validate Snowflake generator settings from backend YAML configuration. | Backend startup/configuration boundary only. | AC-SNOWFLAKEID-02 |
| internal | `IdSerializationPolicy` | Serialize `SnowflakeId` values as decimal strings in frontend-facing DTOs. | Applied by consuming HTTP API DTO mappers. | AC-SNOWFLAKEID-04 |

## Request And Response DTOs

### INTERNAL SnowflakeIdProvider::next_id - Generate ID

- Parameter passing method: Rust trait call with no request body.
- Path variables: not applicable.
- Query parameters: not applicable.
- Request headers: not applicable.
- Request body DTO: none.
- Response body DTO: `Result<SnowflakeId, IdGenerationError>`.
- Generated client impact: none, because this is not an HTTP endpoint.
- Backward compatibility: business modules depend on `SnowflakeIdProvider`; replacing `snowflaked` later must not change this trait unless a new feature evolution is approved.

Proposed Rust-facing shape:

```rust
pub trait SnowflakeIdProvider {
    fn next_id(&self) -> Result<SnowflakeId, IdGenerationError>;
}
```

### INTERNAL YamlGeneratorConfigLoader::load - Load Generator Config

- Parameter passing method: backend configuration object or YAML path owned by the runtime bootstrap layer.
- Path variables: not applicable.
- Query parameters: not applicable.
- Request headers: not applicable.
- Request body DTO: YAML config keys `id.snowflake.instance_id` and `id.snowflake.epoch_ms`.
- Response body DTO: `Result<GeneratorConfig, GeneratorConfigError>`.
- Generated client impact: none.
- Backward compatibility: adding optional config keys is allowed; renaming or moving confirmed keys requires a feature evolution note.

Confirmed YAML shape:

```yaml
id:
  snowflake:
    instance_id: 1
    epoch_ms: 1767225600000
```

Validation rules:

- `instance_id` must be present, numeric, and within the range accepted by the `snowflaked` adapter.
- `epoch_ms` must be present, numeric, and earlier than current generation time.
- Startup should fail fast with a configuration error if YAML values are invalid.
- Multi-instance deployments must provide distinct `instance_id` values per backend instance.

### SERIALIZATION SnowflakeId - Frontend DTO Field Policy

- Parameter passing method: Rust serialization/deserialization policy applied by DTO mappers.
- Path variables: not applicable.
- Query parameters: Snowflake ID query parameters, if a consuming API ever exposes them, must be parsed from decimal strings.
- Request headers: not applicable.
- Request body DTO: Snowflake ID fields in JSON request bodies must be decimal strings when accepted from clients.
- Response body DTO: Snowflake ID fields in JSON responses must be decimal strings.
- Generated client impact: generated TypeScript clients should type Snowflake ID fields as `string`, not `number`.
- Backward compatibility: emitting 64-bit IDs as JSON numbers is a breaking contract violation.

Example response fragment inside an AGENTS.md-compliant HTTP response from a consuming API:

```json
{
  "code": "200",
  "message": "success",
  "data": {
    "id": "734103002743185408"
  }
}
```

## Auth And Permissions

This feature has no user-facing auth or permission rule.

Internal boundary:

- Only backend modules should call `SnowflakeIdProvider`.
- User roles and permissions must not control ID generation.
- ID generation must not be treated as authorization proof.
- Future diagnostics around instance id or generator health would require a separate API contract and permission decision.

## Error Contract

| condition | status/code | response shape | acceptance ids |
| --- | --- | --- | --- |
| YAML config is missing `id.snowflake.instance_id`. | startup error `SNOWFLAKE_INSTANCE_ID_MISSING` | Internal `GeneratorConfigError`; consuming HTTP endpoints should not start with invalid config. | AC-SNOWFLAKEID-02 |
| YAML config has invalid `id.snowflake.instance_id`. | startup error `SNOWFLAKE_INSTANCE_ID_INVALID` | Internal `GeneratorConfigError` with invalid field metadata. | AC-SNOWFLAKEID-02 |
| YAML config has invalid `id.snowflake.epoch_ms`. | startup error `SNOWFLAKE_EPOCH_INVALID` | Internal `GeneratorConfigError` with invalid field metadata. | AC-SNOWFLAKEID-02 |
| `snowflaked` generation fails or rejects configuration. | internal error `SNOWFLAKE_GENERATION_FAILED` | Internal `IdGenerationError`; consuming HTTP APIs map this into the shared `{ code, message, data }` body only if generation fails during a request. | AC-SNOWFLAKEID-01, AC-SNOWFLAKEID-03 |
| DTO deserialization receives an invalid decimal ID string. | HTTP 422 `INVALID_SNOWFLAKE_ID` in consuming APIs | `{ "code": "INVALID_SNOWFLAKE_ID", "message": "Invalid Snowflake ID.", "data": { "field": "id" } }` | AC-SNOWFLAKEID-04 |

## Frontend Backend Boundary

Frontend responsibilities:

- Treat Snowflake-backed IDs as opaque strings.
- Do not parse 64-bit ID strings into JavaScript `number`.
- Prefer stable business codes for cross-module linking when the backend exposes them.

Backend responsibilities:

- Generate IDs through `SnowflakeIdProvider`.
- Hide direct `snowflaked` APIs from business modules.
- Serialize IDs as decimal strings in frontend-facing DTOs.
- Validate decimal ID strings if a consuming API accepts them from clients.

Shared/generated client expectation:

- TypeScript generated clients should model `SnowflakeId` fields as `string`.
- API examples should show IDs as quoted decimal strings.

## Database And Transaction Boundary

This common capability owns no database transaction.

Transaction rules for consumers:

- Generate IDs before insert operations that need application-assigned primary keys.
- Use generated IDs inside the consuming module's transaction.
- Store Snowflake-backed IDs as SQLite `INTEGER`.
- Map database uniqueness or constraint failures in the consuming module, not in `common-snowflake-id`.

Configuration boundary:

- YAML configuration is read at backend startup or dependency construction time.
- Generator configuration must not be read from SQLite.
- Invalid configuration should fail fast before request handling begins.

## Mock Or Adapter Expectations

Required adapters:

- `SnowflakedSnowflakeIdProvider`: production adapter around `snowflaked`.
- `FixedSnowflakeIdProvider`: deterministic provider for unit, integration, and fixture tests.
- Optional sequence provider for tests that need multiple predictable IDs.

Mocking rules:

- RED tests should prove business modules can depend on `SnowflakeIdProvider` without importing `snowflaked`.
- Tests must not rely on wall-clock sleeps to prove ID changes.
- Serialization tests should include values above JavaScript's safe integer limit.
- Config tests should use YAML fixtures for valid and invalid generator settings.

## Open Decisions

Confirmed Phase 1C API boundary:

| decision id | question | recommendation | status |
| --- | --- | --- | --- |
| OD-SNOWFLAKEID-API-01 | Should this feature expose any HTTP endpoint for ID generation or diagnostics in the MVP? | No. Keep it internal-only. | confirmed: follow recommendation |
| OD-SNOWFLAKEID-API-02 | Should the main provider contract be `SnowflakeIdProvider::next_id() -> Result<SnowflakeId, IdGenerationError>`? | Yes. Keep the trait narrow and package-independent. | confirmed: follow recommendation |
| OD-SNOWFLAKEID-API-03 | Should invalid YAML generator configuration fail backend startup instead of surfacing as lazy request errors? | Yes. Fail fast at startup/config construction. | confirmed: follow recommendation |
| OD-SNOWFLAKEID-API-04 | Should frontend-facing Snowflake IDs always be typed and serialized as decimal strings? | Yes. This avoids JavaScript precision loss. | confirmed: follow recommendation |
