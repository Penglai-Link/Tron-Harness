# Test Scope: Common Snowflake ID Generation

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

This Phase 1D draft recommends a backend-only custom scope. The feature has no UI, no public HTTP endpoint, and no database-owned schema, but it does carry correctness risk around uniqueness, configuration, dependency isolation, and JavaScript-safe serialization.

Recommended decision: approve the custom backend capability scope below. After approval, create `spec/test-plan.md` with focused TDD cycles for provider generation, YAML config validation, error mapping, decimal-string serialization, deterministic test providers, and consumer boundary checks.

## Scope Decision

- status: approved_with_defaults
- selected_profile: custom
- approved_by: clouds3n
- approved_at: 2026-05-11

Valid profiles: `minimal`, `standard`, `strict`, `custom`.

## Recommended Scope

Use a custom backend-only test scope:

- Include Rust unit tests for value objects, provider trait behavior, production adapter behavior, deterministic test adapter behavior, and serialization helpers.
- Include backend integration/config tests around YAML loading and validation once the backend Cargo workspace exists.
- Include contract-level review for internal module boundaries because no HTTP endpoint is introduced.
- Exclude frontend component and browser runtime testing because the feature has no page, route, interaction, or visible state.
- Exclude database migration tests for this feature because Phase 1B approved `impact: no`; migration tests move to consuming features.
- Keep `make fe-check` as a regression command only if frontend DTO or TypeScript client code changes.

Risk level: medium. The feature is not user-facing, auth-sensitive, or destructive, but bad ID generation or serialization would become a cross-module platform defect.

## Included Test Layers

| layer | included | rationale | command or evidence |
| --- | --- | --- | --- |
| unit | yes | Core provider, value object, config validation, serialization, deterministic test adapter, and error mapping behavior. | `cargo test --manifest-path tron-backend/Cargo.toml snowflake_id` once backend workspace exists. |
| integration | yes | YAML config loading and provider construction should be tested through realistic config fixtures. | Backend integration/config tests under `tron-backend` once implemented. |
| api-contract | yes | The feature has no HTTP endpoint, but internal Rust contracts and DTO serialization rules are API-boundary decisions. | Internal contract tests and feature-doc validation. |
| database-migration | no | This common capability creates no database-owned objects or migration. | `spec/database-design.md` validation; consuming-feature migration tests later. |
| frontend-component | no | No frontend page or component is introduced. | Not applicable. |
| browser-runtime | no | No user-facing journey exists for ID generation. | Not applicable. |
| performance | no | No high-cardinality query or throughput requirement is introduced in the MVP. | Not applicable unless future load requirements are approved. |
| security | limited | IDs are not authorization proof, but no public generation endpoint should exist and invalid config must fail fast. | Code review and tests for internal-only boundary and startup/config errors. |
| manual-review | yes | Cross-module capability boundaries require traceability until backend source exists. | `reports/review-report.md` in final workflow. |

## Excluded Test Layers

| layer | rationale | revisit trigger |
| --- | --- | --- |
| frontend component tests | Snowflake generation has no UI. DTO string typing can be covered by TypeScript/client tests only if frontend code changes. | A generated client or frontend DTO adapter is introduced. |
| browser runtime tests | There is no route, page, or interaction to exercise. | An admin diagnostics page or visible ID workflow is added. |
| database migration tests in this feature | Phase 1B confirms no tables or migrations for `common-snowflake-id`. | The feature starts owning instance allocation, generated-ID tracking, or schema objects. |
| public REST API tests | The feature intentionally exposes no HTTP endpoint. | A future diagnostics or ID service endpoint is approved. |
| performance/load tests | MVP scope does not define throughput SLOs. | Multi-instance deployment or generator throughput requirements are added. |

## Environment Requirements

- Backend Rust Cargo workspace under `tron-backend` is required before executable Rust tests can run.
- The production dependency should be declared as `snowflaked = { version = "1.0.3", features = ["sync"] }` when shared thread-safe generation is implemented.
- YAML fixtures should cover valid config, missing `instance_id`, invalid `instance_id`, missing `epoch_ms`, and invalid/future `epoch_ms`.
- Tests should not require containers, external services, browser ports, credentials, or network calls.
- Tests should avoid wall-clock sleeps; use deterministic providers and controlled fixtures where possible.
- Frontend regression requires `make fe-check` only when frontend files or generated TypeScript DTOs are touched.

## Test Boundary Matrix

| acceptance ids | required layers | risk or boundary | included | evidence expectation | command candidate | notes |
| --- | --- | --- | --- | --- | --- | --- |
| AC-SNOWFLAKEID-01 | unit/api-contract/review | Provider abstraction and dependency hiding. | yes | RED/GREEN tests plus import/dependency boundary review. | `cargo test --manifest-path tron-backend/Cargo.toml snowflake_id_provider` | Business modules must not import `snowflaked` directly. |
| AC-SNOWFLAKEID-02 | unit/integration/security | YAML config validation and fail-fast behavior. | yes | Valid/invalid YAML fixture tests and mapped config errors. | `cargo test --manifest-path tron-backend/Cargo.toml snowflake_config` | Startup behavior can be tested at config construction boundary. |
| AC-SNOWFLAKEID-03 | unit/review | Production adapter uses approved package and maps dependency errors. | yes | Adapter tests and dependency evidence. | `cargo test --manifest-path tron-backend/Cargo.toml snowflaked_adapter` | Package errors must not leak as third-party types across module boundary. |
| AC-SNOWFLAKEID-04 | unit/api-contract/frontend-regression-if-touched | Decimal-string serialization preserves 64-bit precision for JavaScript clients. | yes | Tests with values above JavaScript safe integer range. | `cargo test --manifest-path tron-backend/Cargo.toml snowflake_serialization`; `make fe-check` if frontend changes | JSON number output is a contract failure. |
| AC-SNOWFLAKEID-05 | docs/database-review | No database-owned schema in common capability. | yes | Database design validation and consumer migration handoff notes. | `"$skill_dir/scripts/validate_feature_docs.py" docs/features/common-snowflake-id --phase spec --complete-spec` | Consumer modules own schema tests. |
| AC-SNOWFLAKEID-06 | unit | Deterministic providers for repeatable fixtures. | yes | Tests using fixed/sequence providers without wall-clock sleeps. | `cargo test --manifest-path tron-backend/Cargo.toml fixed_snowflake_id_provider` | Required before consumers write stable tests. |
| AC-SNOWFLAKEID-07 | review/integration | Consumer modules depend on shared contracts only. | yes | Import scan and consumer test evidence once modules exist. | `rg "snowflaked" tron-backend/src` | Only shared ID adapter should match in production code. |

## Regression Command Candidates

| command | purpose | required for final | status |
| --- | --- | --- | --- |
| `"$skill_dir/scripts/validate_feature_docs.py" docs/features/common-snowflake-id --phase spec --spec-stage acceptance` | Validate Phase 1D docs. | yes | available now |
| `"$skill_dir/scripts/validate_feature_docs.py" docs/features/common-snowflake-id --phase spec --complete-spec` | Validate all spec files after `test-plan.md` exists. | yes after test-plan gate | pending test-scope approval |
| `cargo test --manifest-path tron-backend/Cargo.toml snowflake_id` | Run backend ID capability tests. | yes after backend implementation | blocked until backend Cargo workspace exists |
| `make fe-check` | Catch frontend DTO/build regressions if frontend code changes. | only if frontend files are touched | available through Makefile |
| `rg "snowflaked" tron-backend/src` | Verify direct dependency use stays inside shared ID adapter. | yes after backend implementation | blocked until backend source exists |

## Evidence Required For Completion

- AC-SNOWFLAKEID-01 requires RED and GREEN tests for provider abstraction and ID generation.
- AC-SNOWFLAKEID-02 requires YAML fixture tests and config error mapping evidence.
- AC-SNOWFLAKEID-03 requires Cargo dependency evidence and adapter error mapping tests.
- AC-SNOWFLAKEID-04 requires serialization tests for large 64-bit values and any frontend DTO regression evidence if frontend files change.
- AC-SNOWFLAKEID-05 requires database design validation and explicit no-migration review.
- AC-SNOWFLAKEID-06 requires deterministic provider tests used without timing sleeps.
- AC-SNOWFLAKEID-07 requires module boundary review and import scan after backend source exists.

Manual evidence is acceptable only for boundary review, no-database-impact confirmation, and final traceability. Core provider behavior, config validation, error mapping, and serialization must be automated once backend source exists.

## Open Decisions

| decision id | question | recommendation | status |
| --- | --- | --- | --- |
| OD-SNOWFLAKEID-TEST-01 | Approve the custom backend-only test scope and allow `spec/test-plan.md` to be drafted from it? | Approve as `approved_with_defaults`; no UI/browser/database-migration tests for this feature. | confirmed: yes |
| OD-SNOWFLAKEID-TEST-02 | Should frontend regression `make fe-check` be required even when no frontend files change? | No. Keep it conditional to frontend DTO/client changes. | confirmed: no |
