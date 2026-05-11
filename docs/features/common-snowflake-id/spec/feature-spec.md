# Feature Spec: Common Snowflake ID Generation

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: phase_1a_confirmed

This Phase 1A draft restarts `common-snowflake-id` around the user's correction: the common Snowflake ID capability should use an open-source Rust package instead of designing a custom Snowflake algorithm in Tron. The confirmed package is `snowflaked`, wrapped behind a Tron-owned shared interface so business modules depend on Tron contracts rather than directly spreading third-party crate APIs across bounded contexts.

Decision update on 2026-05-11:

- OD-SNOWFLAKEID-01: extract `SnowflakeIdProvider` now.
- OD-SNOWFLAKEID-02: extract `IdSerializationPolicy` now.
- OD-SNOWFLAKEID-03: extract instance id allocation into YAML runtime configuration now.
- OD-SNOWFLAKEID-04: use the `snowflaked` package.

## Feature Identity

- feature_slug: common-snowflake-id
- status: phase_1a_confirmed
- owner: clouds3n
- source_request: Move `SnowflakeIdGenerator` into a common capability and implement that capability by using an open-source Rust package.
- main_agent: Codex lead agent

## Requirement Description

Design a reusable Snowflake-style ID generation capability for Tron backend modules by wrapping an open-source Rust package.

The common capability provides:

- 64-bit integer IDs for backend-owned records.
- A single backend shared module that centralizes dependency choice, configuration, error handling, and serialization policy.
- JSON-safe serialization guidance so frontend DTOs expose 64-bit IDs as decimal strings.
- A dependency boundary that prevents business features from directly duplicating Snowflake generator setup.

The immediate extraction source is `docs/features/user-management-model/spec/feature-spec.md`, where `SnowflakeIdGenerator` was previously modeled inside User Management. User Management should consume this common capability through a provider interface.

Confirmed package and configuration:

- `snowflaked` with the optional `sync` feature for thread-safe generation.
- Use `i64` Snowflake IDs so generated values fit SQLite signed `INTEGER` and backend language bindings cleanly.
- Configure a Tron-specific epoch instead of the default UNIX epoch to extend the valid `i64` timestamp range.
- Load instance id and epoch from backend YAML configuration rather than hardcoding them in business modules.

## Goals And Non-Goals

Goals:

- Use an open-source Rust package for Snowflake ID generation.
- Use `snowflaked` as the confirmed Rust dependency for Snowflake ID generation.
- Define a Tron-owned `IdGenerator` / `SnowflakeIdProvider` interface so business modules do not depend on third-party crate APIs directly.
- Define a reusable `SnowflakeId` value object for internal 64-bit integer IDs.
- Provide a generator capability that can be consumed by User Management and future Role, Space, Permission, Audit, and other backend modules.
- Preserve database compatibility with SQLite `INTEGER` storage for 64-bit signed IDs.
- Require API/DTO layers to serialize 64-bit IDs as decimal strings when sent to JavaScript clients.
- Centralize instance id, epoch, generator initialization, concurrency mode, and error mapping in one shared module.
- Centralize generator runtime settings in backend YAML configuration so deployments can assign collision-free instance ids without code changes.

Non-goals:

- Implementing a custom Snowflake algorithm from scratch.
- Designing business-specific user, role, space, permission, or audit tables.
- Replacing stable business codes such as `userCode`, `roleCode`, `spaceCode`, or future `permissionCode`.
- Exposing Snowflake IDs as public cross-module business references.
- Creating a frontend visual page or admin settings UI.
- Creating a distributed coordination service in the MVP.
- Guaranteeing globally unique IDs across multiple independently misconfigured deployments.

Assumptions:

- Tron backend will be Rust and domain-driven, but no backend source tree or Cargo workspace exists yet.
- SQLite remains the MVP storage option based on current backend tech-stack docs.
- The common capability will live in a backend shared module once backend source exists.
- Business modules should use stable codes for external references and reserve Snowflake IDs for internal persistence identity.
- `snowflaked` is the approved open-source dependency for this capability.
- The first consuming feature is User Management.

## User Roles And Permissions

This feature has no user-facing roles, pages, or permissions.

Access boundary:

- Snowflake ID generation is an internal backend capability.
- It is not exposed as an admin API.
- It is not controlled by Role Management or Permission Management.
- API authorization is not relevant to this feature because callers are backend modules, not end users.

Operational visibility:

- Later production hardening may expose health or diagnostics around instance id and generator configuration, but that is not part of Phase 1A.

## Page Descriptions

No frontend page is introduced.

Entry points are backend-only:

- Domain/repository code requests a new ID before inserting a persisted record.
- API DTO mapping serializes the generated `SnowflakeId` as a decimal string when returning it to frontend clients.
- Test utilities may use a deterministic adapter or fixed ID provider to create repeatable fixtures.

Empty/loading/error states are not applicable because there is no user interface.

## Domain Model

Bounded context:

- Common Platform Capability is the bounded context for ID generation.
- Business bounded contexts consume this capability through a narrow provider interface.
- User Management must not own `SnowflakeIdGenerator`; it should depend on this common capability.

Aggregate roots:

- Not applicable. ID generation is a shared technical capability, not a business aggregate.

Entities:

- Not applicable in Phase 1A. The generator is runtime behavior and does not require a persistence entity by default.

Value objects:

- `SnowflakeId`: Tron-owned wrapper around a signed 64-bit integer generated by the selected Rust package; JSON representation is a decimal string.
- `InstanceId`: generator instance identifier passed into the open-source package to avoid collisions across concurrent backend instances.
- `GeneratorEpoch`: Tron-specific custom epoch used to extend signed 64-bit timestamp range.
- `GeneratorConfig`: instance id, epoch, package mode, and concurrency mode loaded from backend YAML configuration.
- `GeneratorDependency`: selected open-source package and version requirement, confirmed as `snowflaked = "1.0.3"` with `sync` when thread-safe sharing is required.

Domain services and policies:

- `SnowflakeIdProvider`: Tron-owned trait used by business modules to request IDs.
- `SnowflakedSnowflakeIdProvider`: adapter around the `snowflaked` crate.
- `FixedSnowflakeIdProvider`: deterministic test adapter for fixtures and RED/GREEN tests.
- `InstanceIdProvider`: supplies configured instance id for the running backend instance.
- `YamlGeneratorConfigLoader`: loads and validates ID generator settings from backend YAML runtime configuration.
- `IdSerializationPolicy`: requires decimal-string serialization for JavaScript-facing DTOs.
- `IdDependencyPolicy`: keeps third-party generator APIs inside the common module, not in business modules.

Domain events and lifecycle signals:

- `SnowflakeIdGenerated`: optional test/diagnostic signal, not a persisted business event.
- `SnowflakeGeneratorConfigurationRejected`: operational signal when instance id or epoch configuration is invalid.
- `SnowflakeDependencyErrorMapped`: operational signal when a third-party package error is converted to a Tron error.

Invariants:

- Generated IDs must fit SQLite signed 64-bit `INTEGER` storage.
- Generated IDs must be unique for a given deployment when instance ids are configured correctly.
- Generated IDs must never be used as stable business codes.
- JavaScript-facing DTOs must serialize IDs as decimal strings to avoid precision loss.
- Business modules must consume `SnowflakeIdProvider` instead of importing `snowflaked` directly.
- Tests must be able to use a deterministic provider without wall-clock sleeps.

State transitions:

- Provider initialization validates dependency configuration, instance id, and epoch.
- Provider initialization reads instance id and epoch from backend YAML configuration.
- Runtime generation delegates ID creation to the selected open-source package.
- The adapter converts package output into Tron `SnowflakeId`.
- The adapter maps dependency errors or invalid configuration into Tron-owned error types.

Business rules:

- Business modules request IDs through the shared provider interface rather than constructing generator internals directly.
- Database migrations use `INTEGER` for Snowflake-backed primary keys where the database supports signed 64-bit integers.
- API contracts may expose both internal decimal-string `id` and stable business code fields, but cross-module references should prefer stable codes.
- Changing the underlying open-source package should not require business module API changes if the `SnowflakeIdProvider` interface remains stable.

## Module And Boundary Map

Backend modules to create once backend source exists:

- `tron-backend/src/shared/id/mod.rs`
- `tron-backend/src/shared/id/snowflake.rs`
- `tron-backend/src/shared/id/config.rs`
- `tron-backend/src/shared/id/serialization.rs`
- `tron-backend/src/shared/id/testing.rs`

Backend YAML configuration to introduce once backend source exists:

```yaml
id:
  snowflake:
    instance_id: 1
    epoch_ms: 1767225600000
```

Confirmed dependency:

- `snowflaked = { version = "1.0.3", features = ["sync"] }` when backend code needs a shared thread-safe generator.
- Use a wrapper so business modules remain insulated from direct third-party crate APIs.

Candidate public interfaces:

- `SnowflakeIdProvider`
- `SnowflakedSnowflakeIdProvider`
- `FixedSnowflakeIdProvider`
- `SnowflakeId`
- `GeneratorConfig`
- `InstanceIdProvider`
- `YamlGeneratorConfigLoader`

Consuming modules:

- User Management consumes the common provider for `users`, assignment rows, and password reset event IDs.
- Future Role Management consumes it for role-owned records.
- Future Space Management consumes it for workspace/space-owned records.
- Future Permission Management consumes it for permission-owned records.
- Future Audit Log or operational event modules may consume it for audit/event records.

Frontend boundary:

- No frontend implementation is required.
- Frontend DTOs receive Snowflake-backed IDs as decimal strings.
- Frontend must not parse these values into JavaScript `number` when exact precision matters.

External services:

- None for MVP.
- Multi-instance deployments must assign unique YAML `id.snowflake.instance_id` values per backend instance; an external allocation service remains outside the MVP.

Ownership boundary:

- This feature owns dependency selection, wrapper interface, configuration policy, and serialization policy.
- The open-source package owns low-level Snowflake ID generation mechanics.
- Business modules own their business codes and persistence schemas.
- Business modules must not duplicate Snowflake package setup or import the selected package directly.

Candidate common capabilities:

- `SnowflakeIdProvider` is the common capability being extracted.
- `IdSerializationPolicy` should also become a shared DTO/adapters rule because JavaScript precision loss affects every frontend-facing backend module.

## Repository Findings

- Existing feature workspace: `docs/features/user-management-model`.
- Current User Management feature now references `CommonSnowflakeIdProvider` and should not own the generator implementation.
- Current User Management database design already expects 64-bit Snowflake IDs stored as SQLite `INTEGER`.
- Current User Management API contract already expects IDs serialized to frontend JSON as decimal strings.
- Root `docs/tech-stack.md` describes a Rust + React monorepo where the backend serves APIs and frontend static assets.
- `tron-backend/docs/tech-stack.md` says the backend is Rust with SQLite by default.
- `tron-backend/docs/tech-stack.md` also says backend configuration uses YAML.
- `tron-backend` currently contains only docs and `.gitignore`; no Rust Cargo workspace, shared module, or ID generator implementation exists yet.
- `Makefile` currently exposes frontend workflow targets only: `fe-install`, `fe-dev`, `fe-check`, `fe-build`, `fe-preview`, and `fe-clean`.
- Package research:
  - `snowflaked` 1.0.3 is documented on docs.rs, licensed MIT OR Apache-2.0, supports `u64` and `i64` snowflake IDs, supports a configurable epoch, and has an optional `sync` module.
  - `snowflake-gen` 1.0.2 is configurable but very new and low-download as of the package check.
  - `sonyflake` is mature but implements Sonyflake-inspired semantics rather than a direct Snowflake ID package.

## Open Decisions

| decision id | candidate common capability | reuse signal | affected modules | proposed shared module | recommendation | user decision |
| --- | --- | --- | --- | --- | --- | --- |
| OD-SNOWFLAKEID-01 | `SnowflakeIdProvider` backed by `snowflaked` | Cross-feature ID generation for users, assignments, reset events, future roles, spaces, permissions, and audit records. | Backend business modules and repository layers. | `tron-backend/src/shared/id` | extract now with `snowflaked` wrapper | confirmed: extract now |
| OD-SNOWFLAKEID-02 | `IdSerializationPolicy` | Every frontend-facing API can lose precision if 64-bit IDs are emitted as JSON numbers. | Backend DTO mappers, frontend API adapters, generated clients if introduced. | `tron-backend/src/shared/id/serialization.rs` plus API contract rules | extract now | confirmed: extract now |
| OD-SNOWFLAKEID-03 | Instance id allocation policy | Multi-instance deployments require collision-free instance ids. | Backend runtime configuration and deployment docs. | backend YAML runtime config plus `tron-backend/src/shared/id/config.rs` | extract now through YAML config | confirmed: extract YAML config |
| OD-SNOWFLAKEID-04 | Open-source package choice | The feature must use an open-source Rust package, but dependency maturity and API fit need explicit approval. | Backend shared ID module. | `tron-backend/src/shared/id` | use `snowflaked`; keep business modules behind wrapper | confirmed: use `snowflaked` |
