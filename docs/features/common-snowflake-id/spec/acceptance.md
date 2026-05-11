# Acceptance: Common Snowflake ID Generation

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: phase_1d_confirmed

This Phase 1D draft translates the confirmed common Snowflake ID design into stable acceptance criteria. The feature is backend-only: it wraps the approved `snowflaked` Rust package, exposes Tron-owned provider and value-object contracts, loads generator settings from backend YAML, and requires JavaScript-facing DTOs to serialize 64-bit IDs as decimal strings.

Recommended default: accept the backend-only scope below, with automated Rust tests once `tron-backend/Cargo.toml` exists, feature-doc validation now, and no frontend/browser acceptance because this feature introduces no page or HTTP endpoint.

## Acceptance Criteria Matrix

| id | criterion | evidence type | expected evidence | priority |
| --- | --- | --- | --- | --- |
| AC-SNOWFLAKEID-01 | A Tron-owned `SnowflakeIdProvider` interface hides direct `snowflaked` usage from business modules and returns signed 64-bit `SnowflakeId` values. | automated | RED/GREEN Rust tests for provider construction and `next_id`, plus implementation path under `tron-backend/src/shared/id`. | must |
| AC-SNOWFLAKEID-02 | Generator configuration is loaded from backend YAML keys `id.snowflake.instance_id` and `id.snowflake.epoch_ms`, with invalid or missing values rejected before request handling. | automated | YAML fixture tests for valid, missing, and invalid config values; startup/config loader error mapping evidence. | must |
| AC-SNOWFLAKEID-03 | The production adapter uses the approved `snowflaked` dependency and maps package/config/generation errors into Tron-owned error types. | automated | Cargo dependency evidence, adapter unit tests, and error mapping tests without leaking third-party errors across module boundaries. | must |
| AC-SNOWFLAKEID-04 | Snowflake-backed IDs are serialized to JavaScript-facing JSON as decimal strings and parsed from decimal strings when accepted by DTO mappers. | automated | Serialization/deserialization tests including a value above JavaScript's safe integer range. | must |
| AC-SNOWFLAKEID-05 | The feature creates no database-owned tables or migrations while documenting the consuming-module rule: store Snowflake-backed internal IDs as SQLite `INTEGER`. | docs + automated when consumers exist | Database design validation now; future consumer migration tests prove signed 64-bit compatibility. | should |
| AC-SNOWFLAKEID-06 | Tests can use deterministic ID providers without wall-clock sleeps or direct dependency on the production generator. | automated | `FixedSnowflakeIdProvider` or equivalent fixture tests used by common capability and consumer tests. | must |
| AC-SNOWFLAKEID-07 | User Management and future backend modules consume only the shared provider/value-object contract, not direct `snowflaked` APIs. | review + automated | Module boundary review, import scan, and consumer tests once backend modules exist. | should |

## Business Acceptance

- [ ] AC-SNOWFLAKEID-01 Backend modules can request internal persistence IDs through a shared provider contract.
- [ ] AC-SNOWFLAKEID-03 The selected open-source package is `snowflaked`, but the dependency remains replaceable behind Tron-owned interfaces.
- [ ] AC-SNOWFLAKEID-07 User Management no longer owns Snowflake generation as a feature-local concept.

## UI Acceptance

- [ ] No UI page, route, screen state, or browser journey is introduced by this feature.
- [ ] AC-SNOWFLAKEID-04 Any future frontend-facing DTO receives Snowflake-backed IDs as strings, not JavaScript numbers.

## Backend Acceptance

- [ ] AC-SNOWFLAKEID-01 `SnowflakeIdProvider` and `SnowflakeId` are implemented in a backend shared module.
- [ ] AC-SNOWFLAKEID-02 YAML configuration is validated during provider/config construction.
- [ ] AC-SNOWFLAKEID-03 `snowflaked` is used only inside the shared ID adapter.
- [ ] AC-SNOWFLAKEID-06 deterministic test providers are available for repeatable tests.

## Database Acceptance

- [ ] AC-SNOWFLAKEID-05 database impact is explicitly approved as `impact: no`.
- [ ] No `common-snowflake-id` table, migration, generated-ID tracking table, or instance-allocation table is created.
- [ ] Consuming features document and test SQLite `INTEGER` compatibility when they introduce Snowflake-backed persisted records.

## Integration Acceptance

- [ ] AC-SNOWFLAKEID-07 consumer modules can depend on the provider trait and value object without importing `snowflaked`.
- [ ] AC-SNOWFLAKEID-04 DTO mapping examples and tests preserve exact 64-bit ID values as decimal strings.
- [ ] The feature-doc validation chain passes for the reached design gate before task breakdown.

## Security And Privacy Acceptance

- [ ] The generator is internal-only and exposes no public ID generation endpoint.
- [ ] Snowflake IDs are not treated as authorization proof or stable public business identifiers.
- [ ] Configuration errors fail fast instead of producing lazy request-time behavior.

## Regression Acceptance

- [ ] Existing frontend build/typecheck remains clean if DTO typing or examples are touched.
- [ ] Existing User Management design references the common provider instead of reintroducing a feature-owned generator.
- [ ] Feature docs remain valid after acceptance and test-scope updates.

## Evidence Requirements

| evidence item | maps to | required now | required before final |
| --- | --- | --- | --- |
| `"$skill_dir/scripts/validate_feature_docs.py" docs/features/common-snowflake-id --phase spec --spec-stage acceptance` | AC-SNOWFLAKEID-01 through AC-SNOWFLAKEID-07 | yes | yes |
| `cargo test --manifest-path tron-backend/Cargo.toml snowflake_id` | AC-SNOWFLAKEID-01, AC-SNOWFLAKEID-02, AC-SNOWFLAKEID-03, AC-SNOWFLAKEID-04, AC-SNOWFLAKEID-06 | no, backend Cargo workspace does not exist yet | yes, after backend workspace exists |
| Import or dependency scan proving `snowflaked` is only used inside `tron-backend/src/shared/id` | AC-SNOWFLAKEID-01, AC-SNOWFLAKEID-03, AC-SNOWFLAKEID-07 | no | yes |
| YAML fixture tests for valid and invalid generator configuration | AC-SNOWFLAKEID-02 | no | yes |
| Serialization tests for IDs above JavaScript safe integer range | AC-SNOWFLAKEID-04 | no | yes |
| `make fe-check` | AC-SNOWFLAKEID-04 and regression scope when frontend DTO code changes | no | only if frontend files are touched |
