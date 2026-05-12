# Test Plan: Common Snowflake ID Generation

## Test Plan Decision

- status: approved
- approved_by: clouds3n
- approved_at: 2026-05-11

Valid statuses: `waiting_human_decision`, `approved`, `approved_with_defaults`.

Use `approved` or `approved_with_defaults` only after the user reviews this exact test plan and explicitly approves moving to task breakdown.

## Test Strategy

This Phase 2 plan resumes `common-snowflake-id` from the confirmed Phase 1D test scope. No requirement, database, API, or acceptance criteria changed during this continuation; the affected gate is only the test-plan gate. There are no active worker worktrees to freeze, and no RED/GREEN evidence exists yet.

The executable strategy is backend-only TDD. Because `tron-backend` currently has no Cargo workspace, task breakdown should include a support/precondition row that creates the minimal Rust crate harness without implementing Snowflake behavior. RED cycles should then introduce failing tests for the shared ID contract, config loader, adapter boundary, serialization policy, deterministic providers, and consumer import boundary. GREEN work may implement production code only after matching RED failure evidence is recorded and integrated.

The repository Makefile currently exposes frontend targets only. Backend verification commands therefore use Cargo directly through RTK until a backend Makefile target exists. `make fe-check` remains conditional and is required only if frontend DTO or generated client files are touched.

## TDD Cycle Table

### Positive Scenarios

| cycle id | primary acceptance id | related acceptance ids | test slice type | slice focus | test layer | expected result | detail anchor | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TDD-SNOWFLAKEID-01 | AC-SNOWFLAKEID-01 | AC-SNOWFLAKEID-03 | core-business | `SnowflakeIdProvider::next_id` returns a Tron-owned signed 64-bit `SnowflakeId` without exposing `snowflaked` types. | unit | Provider call succeeds and returned value is owned by the shared ID module. | TDD-SNOWFLAKEID-01 Details | draft |
| TDD-SNOWFLAKEID-02 | AC-SNOWFLAKEID-02 | AC-SNOWFLAKEID-03 | configuration | Valid YAML `id.snowflake.instance_id` and `id.snowflake.epoch_ms` produce `GeneratorConfig`. | unit/integration | Config loader accepts valid YAML and constructs provider-ready config. | TDD-SNOWFLAKEID-02 Details | draft |
| TDD-SNOWFLAKEID-03 | AC-SNOWFLAKEID-06 | AC-SNOWFLAKEID-01 | test-adapter | `FixedSnowflakeIdProvider` returns deterministic IDs for fixtures. | unit | Tests can request fixed IDs without wall-clock sleeps or production generator setup. | TDD-SNOWFLAKEID-03 Details | draft |
| TDD-SNOWFLAKEID-04 | AC-SNOWFLAKEID-04 | AC-SNOWFLAKEID-01 | serialization | `SnowflakeId` serializes to a quoted decimal string and deserializes from one. | unit/api-contract | JSON DTO helpers preserve exact ID value as a string. | TDD-SNOWFLAKEID-04 Details | draft |
| TDD-SNOWFLAKEID-05 | AC-SNOWFLAKEID-07 | AC-SNOWFLAKEID-01 | consumer-boundary | A sample consumer module depends on `SnowflakeIdProvider` and `SnowflakeId`, not on `snowflaked`. | unit/review | Consumer-facing code compiles through the shared provider contract only. | TDD-SNOWFLAKEID-05 Details | draft |

### Exception Scenarios

| cycle id | primary acceptance id | related acceptance ids | test slice type | slice focus | test layer | expected result | detail anchor | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TDD-SNOWFLAKEID-06 | AC-SNOWFLAKEID-02 | AC-SNOWFLAKEID-03 | error | YAML missing `id.snowflake.instance_id` is rejected. | unit/integration | Loader returns `SNOWFLAKE_INSTANCE_ID_MISSING` before provider construction. | TDD-SNOWFLAKEID-06 Details | draft |
| TDD-SNOWFLAKEID-07 | AC-SNOWFLAKEID-02 | AC-SNOWFLAKEID-03 | error | YAML contains an out-of-range or non-numeric `instance_id`. | unit/integration | Loader returns `SNOWFLAKE_INSTANCE_ID_INVALID` with field metadata. | TDD-SNOWFLAKEID-07 Details | draft |
| TDD-SNOWFLAKEID-08 | AC-SNOWFLAKEID-02 | AC-SNOWFLAKEID-03 | error | YAML contains a missing, invalid, or future `epoch_ms`. | unit/integration | Loader returns `SNOWFLAKE_EPOCH_INVALID` before request handling. | TDD-SNOWFLAKEID-08 Details | draft |
| TDD-SNOWFLAKEID-09 | AC-SNOWFLAKEID-03 | AC-SNOWFLAKEID-01 | error | The `snowflaked` adapter maps dependency/config/generation failures into Tron-owned errors. | unit | Tests observe `SNOWFLAKE_GENERATION_FAILED` or config errors without third-party error leakage. | TDD-SNOWFLAKEID-09 Details | draft |
| TDD-SNOWFLAKEID-10 | AC-SNOWFLAKEID-04 | AC-SNOWFLAKEID-01 | error | Invalid decimal ID strings are rejected during deserialization/parsing. | unit/api-contract | Parser returns `INVALID_SNOWFLAKE_ID` style error metadata for consuming DTO mappers. | TDD-SNOWFLAKEID-10 Details | draft |

### Boundary Scenarios

| cycle id | primary acceptance id | related acceptance ids | test slice type | slice focus | test layer | expected result | detail anchor | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TDD-SNOWFLAKEID-11 | AC-SNOWFLAKEID-04 | AC-SNOWFLAKEID-05 | boundary | ID value above JavaScript safe integer range round-trips as a decimal string. | unit/api-contract | Exact `i64` value is preserved and never emitted as a JSON number. | TDD-SNOWFLAKEID-11 Details | draft |
| TDD-SNOWFLAKEID-12 | AC-SNOWFLAKEID-05 | AC-SNOWFLAKEID-02 | boundary | Common capability declares no database-owned table, migration, or generated-ID tracking schema. | unit/docs-review | Persistence policy confirms `impact: no`, and scans find no common Snowflake migration/table artifact. | TDD-SNOWFLAKEID-12 Details | draft |
| TDD-SNOWFLAKEID-13 | AC-SNOWFLAKEID-07 | AC-SNOWFLAKEID-03 | boundary | `snowflaked` imports are limited to the shared ID adapter path. | review/automation | Import scan finds `snowflaked` only under `tron-backend/src/shared/id` and dependency metadata. | TDD-SNOWFLAKEID-13 Details | draft |

## TDD Cycle Details

### TDD-SNOWFLAKEID-01 Details

- Assertions: provider trait returns `Result<SnowflakeId, IdGenerationError>`; `SnowflakeId` exposes a signed 64-bit internal value without leaking third-party types.
- Test data: valid `GeneratorConfig` with `instance_id: 1` and `epoch_ms: 1767225600000`.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_id_provider` fails because the shared ID provider contract is missing.
- GREEN/VERIFY evidence: the same command passes after shared provider and value-object implementation.
- Status: draft

### TDD-SNOWFLAKEID-02 Details

- Assertions: YAML loader accepts the confirmed config shape and returns typed config fields.
- Test data: YAML fixture with `id.snowflake.instance_id: 1` and `id.snowflake.epoch_ms: 1767225600000`.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_config_valid` fails because the config loader is missing.
- GREEN/VERIFY evidence: the same command passes after config loader implementation.
- Status: draft

### TDD-SNOWFLAKEID-03 Details

- Assertions: deterministic provider returns fixed IDs in order and requires no wall-clock sleeps.
- Test data: fixed IDs `734103002743185408` and `734103002743185409`.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml fixed_snowflake_id_provider` fails because the test provider is missing.
- GREEN/VERIFY evidence: the same command passes after deterministic provider implementation.
- Status: draft

### TDD-SNOWFLAKEID-04 Details

- Assertions: serialization emits a quoted decimal string; deserialization accepts a valid decimal string into `SnowflakeId`.
- Test data: `734103002743185408`.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_serialization_string` fails because serialization policy is missing.
- GREEN/VERIFY evidence: the same command passes after serialization helpers are implemented.
- Status: draft

### TDD-SNOWFLAKEID-05 Details

- Assertions: a consumer fixture can call the provider trait without importing `snowflaked`; business code receives `SnowflakeId`.
- Test data: consumer test module using a fixed provider.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml common_snowflake_consumer_boundary` fails because the public shared contract is missing.
- GREEN/VERIFY evidence: the same command passes after public module exports are complete.
- Status: draft

### TDD-SNOWFLAKEID-06 Details

- Assertions: missing `instance_id` maps to a Tron-owned config error code.
- Test data: YAML fixture with `id.snowflake.epoch_ms` only.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_config_missing_instance_id` fails because validation is missing.
- GREEN/VERIFY evidence: the same command passes after fail-fast config validation.
- Status: draft

### TDD-SNOWFLAKEID-07 Details

- Assertions: invalid `instance_id` maps to `SNOWFLAKE_INSTANCE_ID_INVALID`.
- Test data: YAML fixtures with `instance_id: -1`, a non-numeric value, and a value outside the adapter-supported range.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_config_invalid_instance_id` fails because validation is missing.
- GREEN/VERIFY evidence: the same command passes after range/type validation.
- Status: draft

### TDD-SNOWFLAKEID-08 Details

- Assertions: missing, invalid, or future epoch values map to `SNOWFLAKE_EPOCH_INVALID`.
- Test data: YAML fixtures with missing `epoch_ms`, malformed `epoch_ms`, and a future epoch.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_config_invalid_epoch` fails because epoch validation is missing.
- GREEN/VERIFY evidence: the same command passes after epoch validation.
- Status: draft

### TDD-SNOWFLAKEID-09 Details

- Assertions: adapter failures are represented as Tron-owned errors and do not expose third-party error types in the provider signature.
- Test data: invalid adapter construction input and generation failure seam if supported by the crate wrapper.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml snowflaked_adapter_error_mapping` fails because adapter error mapping is missing.
- GREEN/VERIFY evidence: the same command passes after adapter maps errors into `IdGenerationError` and `GeneratorConfigError`.
- Status: draft

### TDD-SNOWFLAKEID-10 Details

- Assertions: invalid decimal ID strings are rejected with field-level error metadata.
- Test data: empty string, non-digit string, negative string when not accepted by DTO policy, and out-of-range integer string.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_serialization_invalid_string` fails because parser validation is missing.
- GREEN/VERIFY evidence: the same command passes after strict parser validation.
- Status: draft

### TDD-SNOWFLAKEID-11 Details

- Assertions: values above `Number.MAX_SAFE_INTEGER` remain exact through serialize and parse.
- Test data: `9007199254740993` and `734103002743185408`.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_serialization_large_i64` fails because exact string round-trip is missing.
- GREEN/VERIFY evidence: the same command passes after decimal-string policy implementation.
- Status: draft

### TDD-SNOWFLAKEID-12 Details

- Assertions: common Snowflake ID owns no database objects and exposes only a persistence convention for consumers.
- Test data: feature database design plus backend source scan for common Snowflake migrations/table definitions.
- RED proof: `rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_no_database_objects` fails because the common persistence policy is missing.
- GREEN/VERIFY evidence: the same command and feature-doc validation pass after the policy is implemented and no migration files are introduced.
- Status: draft

### TDD-SNOWFLAKEID-13 Details

- Assertions: direct `snowflaked` references are limited to dependency metadata and `tron-backend/src/shared/id`.
- Test data: backend source import scan after implementation.
- RED proof: `rtk proxy rg "snowflaked" tron-backend/src` initially has no valid shared adapter location after RED tests require the adapter boundary.
- GREEN/VERIFY evidence: `rtk proxy rg "snowflaked" tron-backend/src` shows matches only in `tron-backend/src/shared/id`, and Cargo metadata includes the approved dependency.
- Status: draft

## Unit Test Plan

Unit tests cover `SnowflakeId`, `SnowflakeIdProvider`, `SnowflakedSnowflakeIdProvider`, `FixedSnowflakeIdProvider`, config validation, error mapping, serialization, deserialization, and persistence policy helpers. The primary unit command is `rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake`.

## Integration Test Plan

Integration/config tests cover realistic YAML fixtures for valid and invalid generator settings and provider construction from loaded config. No external service, container, browser, or credential is required.

## Database Test Plan

Not applicable.

## Frontend Test Plan

Not applicable.

## Browser Acceptance Plan

Not applicable.

## Security And Privacy Test Plan

Tests and review must prove invalid generator configuration fails before request handling, the feature exposes no public ID generation endpoint, and Snowflake IDs are not treated as authorization proof.

## Test Data And Fixtures

Required fixtures include valid Snowflake YAML, missing `instance_id`, invalid `instance_id`, missing `epoch_ms`, invalid/future `epoch_ms`, fixed deterministic ID sequences, invalid decimal ID strings, and large IDs above JavaScript safe integer range.

## Regression Scope

- Re-run feature-doc validation after the test plan is added.
- Run backend Cargo tests for the Snowflake ID filters after implementation.
- Run `rtk proxy rg "snowflaked" tron-backend/src` after implementation to verify dependency isolation.
- Run `rtk make fe-check` only if frontend DTO or generated client files are touched.

## Command Matrix

| command | purpose | when to run | expected evidence |
| --- | --- | --- | --- |
| `rtk proxy sh -c 'skill_dir="/Users/clouds3n/Coding/open-source/ai/Tron/.agents/skills/eliteforge-feature-oriented-development"; "$skill_dir/scripts/validate_feature_docs.py" docs/features/common-snowflake-id --phase spec --complete-spec --no-downstream-artifacts'` | Validate Phase 2 complete spec and confirm no downstream artifacts were created early. | After this test plan is drafted and after any test-plan edits. | Validation passes. |
| `rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake` | Run the common Snowflake ID unit/config/serialization/provider test suite. | RED, GREEN, refactor, and final verification after backend crate support exists. | RED fails for expected missing contracts; GREEN and final verification pass. |
| `rtk cargo test --manifest-path tron-backend/Cargo.toml common_snowflake` | Run consumer-boundary and no-database-object checks. | RED, GREEN, and final verification. | Consumer uses shared provider only; no common DB objects are introduced. |
| `rtk proxy rg "snowflaked" tron-backend/src` | Verify direct dependency usage stays inside the shared ID adapter. | After GREEN implementation and final review. | Matches are limited to `tron-backend/src/shared/id`. |
| `rtk make fe-check` | Frontend regression gate if DTO/client files are touched. | Only when frontend files change. | Typecheck and build pass. |
