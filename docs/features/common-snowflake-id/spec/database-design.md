# Database Design: Common Snowflake ID Generation

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: phase_1b_confirmed

This Phase 1B draft records the persistence boundary for `common-snowflake-id` after the Phase 1A decisions were confirmed. The common capability does not create its own database tables or migrations. It does define the database contract that consuming modules must follow: Snowflake-backed internal identifiers are signed 64-bit integers stored as SQLite `INTEGER`, while API DTOs serialize them as decimal strings.

## Database Decision

- status: approved
- impact: no
- approved_by: clouds3n
- approved_at: 2026-05-11

No database-owned objects belong to this common capability. The generator is runtime behavior backed by the `snowflaked` crate and backend YAML configuration.

Database-facing constraints still apply to consuming features:

- Persist Snowflake-backed internal IDs in SQLite `INTEGER` columns.
- Treat those IDs as internal persistence identity, not stable business codes.
- Serialize IDs to JavaScript clients as decimal strings through the shared `IdSerializationPolicy`.
- Configure `id.snowflake.instance_id` and `id.snowflake.epoch_ms` in backend YAML, not in a database table.

## Existing Schema Impact

Current repository state:

- `tron-backend` has no Cargo workspace, migrations, ORM entities, repositories, or schema files.
- `docs/features/user-management-model/spec/database-design.md` already expects 64-bit Snowflake IDs stored as SQLite `INTEGER`.
- No existing database object needs to be altered for this common capability at Phase 1B.

Affected downstream schema conventions:

- User Management: `users.id`, assignment IDs, reset event IDs, and operator IDs should use 64-bit Snowflake-backed SQLite `INTEGER`.
- Future Role, Space, Permission, Audit, and operational event modules should use the same internal ID convention when they own persisted records.
- Cross-module references should continue to prefer stable codes such as `userCode`, `roleCode`, `spaceCode`, and future `permissionCode`.

## Proposed Schema

No table, view, index, trigger, or migration is proposed for `common-snowflake-id`.

Schema contract for consuming modules:

| object | change type | columns or fields | constraints | indexes | notes |
| --- | --- | --- | --- | --- | --- |
| consuming module primary key | convention | `id INTEGER` | signed 64-bit Snowflake ID; primary key when owned by the module | module-specific | Generated through `SnowflakeIdProvider`, never by direct `snowflaked` calls in business modules. |
| consuming module audit/operator reference | convention | `created_by INTEGER`, `updated_by INTEGER`, or module-specific operator fields | nullable when caller identity is unavailable | module-specific | Stores internal Snowflake user IDs only when available. |
| frontend/API DTO ID field | serialization convention | `id` as decimal string | must not be emitted as JSON number | not applicable | Enforced by shared `IdSerializationPolicy`. |
| backend YAML config | runtime configuration | `id.snowflake.instance_id`, `id.snowflake.epoch_ms` | `instance_id` must be unique per backend instance in the same deployment | not applicable | Config is not a database object. |

## Relationships And Cardinality

This feature owns no persisted relationships.

Downstream relationship rules:

- A generated Snowflake ID may be used as a primary key for a record owned by a consuming module.
- Foreign keys may reference Snowflake-backed `INTEGER` primary keys inside the same bounded context.
- Cross-module contracts should prefer stable code fields instead of leaking internal Snowflake IDs as durable external references.
- YAML `instance_id` values are deployment/runtime settings, not relational data.

## Migration Plan

No migration is required for this common capability.

When a consuming module introduces Snowflake-backed persistence:

1. Add `INTEGER` primary key columns or internal reference columns in that module's own migration.
2. Generate values through `SnowflakeIdProvider`.
3. Keep stable business code columns for public/cross-module references where required.
4. Add migration tests that prove the column can store signed 64-bit Snowflake IDs.

Rollback:

- Roll back each consuming module's migration in that module's feature scope.
- No standalone rollback exists for `common-snowflake-id` because it has no database objects.

## Query And Performance Plan

No queries are introduced by this common capability.

Performance expectations for consumers:

- SQLite `INTEGER` primary keys remain efficient for point reads, joins, and indexes.
- Generated IDs should be indexed by normal primary key or foreign key indexes owned by consuming modules.
- Snowflake ordering is useful for approximate creation-time locality, but business queries should still use explicit timestamp columns such as `created_at` when ordering semantics matter.
- YAML configuration should be loaded once during backend startup and reused by the provider; generation should not read configuration from disk per ID.

## Data Governance

Snowflake IDs are internal technical identifiers. They are not sensitive by themselves, but they can reveal rough creation order or timestamp information depending on the Snowflake layout.

Governance rules:

- Do not treat Snowflake IDs as public stable business identifiers.
- Do not use Snowflake IDs as authorization proof.
- Do not expose raw numeric IDs to JavaScript clients as JSON numbers.
- Keep `instance_id` values in deployment configuration and review them for uniqueness before running multiple backend instances.
- Do not store generator instance allocation state in SQLite for the MVP.

## Test Data And Fixtures

Fixtures should include:

- Fixed deterministic IDs through `FixedSnowflakeIdProvider` for repeatable unit and integration tests.
- At least one large 64-bit ID value above JavaScript's safe integer range to prove decimal-string serialization.
- YAML config fixtures for valid `instance_id` and invalid duplicate/out-of-range values once backend config loading exists.
- Consumer-module database fixtures that store Snowflake-backed IDs as SQLite `INTEGER`.

## Database Validation Commands

| command | purpose | expected evidence |
| --- | --- | --- |
| `skill_dir="/Users/clouds3n/Coding/open-source/ai/Tron/.agents/skills/eliteforge-feature-oriented-development"; "$skill_dir/scripts/validate_feature_docs.py" docs/features/common-snowflake-id --phase spec --spec-stage database` | Validate Phase 1B database design documentation. | Validation passes. |
| `cargo test --manifest-path tron-backend/Cargo.toml snowflake_id` | Future backend test for ID generation, config loading, serialization, and persistence compatibility after the Rust workspace exists. | Passing backend tests once implementation exists. |
| `make fe-check` | Existing Makefile quality gate to catch frontend-facing type/build regressions if DTO handling changes. | Typecheck and build success when frontend DTOs are touched. |

## Open Decisions

Confirmed Phase 1B database boundary:

| decision id | question | recommendation | status |
| --- | --- | --- | --- |
| OD-SNOWFLAKEID-DB-01 | Should `common-snowflake-id` create any database table for instance allocation or generated ID tracking? | No. Keep instance allocation in YAML config for this feature. | confirmed: no database table |
| OD-SNOWFLAKEID-DB-02 | Should consuming modules store Snowflake-backed IDs as SQLite `INTEGER` and serialize API IDs as decimal strings? | Yes. This matches User Management and JavaScript precision constraints. | confirmed: yes |
