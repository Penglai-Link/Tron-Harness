# Test Plan: Init Rust Backend

## Test Plan Decision

- status: approved_with_defaults
- approved_by: user execution request and lead-agent defaults
- approved_at: 2026-05-12

Valid statuses: `waiting_human_decision`, `approved`, `approved_with_defaults`.

## Test Strategy

Execute a small TDD loop around backend initialization. RED worker first adds tests for the expected router, response shape, and configuration defaults. GREEN worker then initializes the Rust service and Makefile commands until those tests and checks pass.

## TDD Cycle Table

### Positive Scenarios

| cycle id | primary acceptance id | related acceptance ids | test slice type | slice focus | test layer | expected result | status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TDD-BACKEND-01 | AC-BACKEND-01 | AC-BACKEND-01, AC-BACKEND-03 | happy-path | Cargo package exposes buildable app router and Makefile check/test targets. | unit/integration | `make be-check` and `make be-test` pass. | approved |
| TDD-BACKEND-02 | AC-BACKEND-02 | AC-BACKEND-02 | api-contract | Health endpoint returns uniform response envelope. | integration/api-contract | Router test receives HTTP 200 with `SUCCESS`, `ok`, and health data. | approved |

### Exception Scenarios

| cycle id | primary acceptance id | related acceptance ids | test slice type | slice focus | test layer | expected result | status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TDD-BACKEND-03 | AC-BACKEND-03 | AC-BACKEND-03 | regression | Root Makefile remains the backend command entry point. | integration | `make help` lists backend targets and existing frontend targets. | approved |

### Boundary Scenarios

| cycle id | primary acceptance id | related acceptance ids | test slice type | slice focus | test layer | expected result | status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TDD-BACKEND-04 | AC-BACKEND-02 | AC-BACKEND-02 | boundary | Health response contains only safe static service metadata. | integration/security | Response data excludes secrets and user/tenant data. | approved |

## TDD Cycle Details

### TDD-BACKEND-01 Details

- Assertions: backend package compiles and exposes reusable router construction from library code.
- Test data: default config values.
- RED proof: `make be-test` fails because no backend application package or router contract exists.
- GREEN/VERIFY evidence: `make be-check` and `make be-test` pass after implementation.
- Status: approved

### TDD-BACKEND-02 Details

- Assertions: `GET /api/v1/health` returns HTTP 200 and JSON with `code`, `message`, and `data`.
- Test data: in-memory Axum router request.
- RED proof: `make be-test` fails because the route and response envelope do not exist.
- GREEN/VERIFY evidence: `make be-test` passes with health route assertions.
- Status: approved

### TDD-BACKEND-03 Details

- Assertions: root Makefile contains backend targets and they execute Cargo in `tron-backend`.
- Test data: `make help` output.
- RED proof: `make help` initially lacks backend targets.
- GREEN/VERIFY evidence: `make help`, `make be-check`, and `make be-test` pass.
- Status: approved

### TDD-BACKEND-04 Details

- Assertions: health payload has service status/name/version only and no secret-like keys.
- Test data: health response JSON.
- RED proof: `make be-test` fails before health payload exists.
- GREEN/VERIFY evidence: router test validates the allowed fields.
- Status: approved

## Unit Test Plan

- Config default tests.
- Response envelope serialization tests where useful.

## Integration Test Plan

- Axum router test for `GET /api/v1/health`.
- Makefile command verification from repository root.

## Database Test Plan

Not applicable.

## Frontend Test Plan

Not applicable.

## Browser Acceptance Plan

Not applicable.

## Security And Privacy Test Plan

- Verify health response contains no secrets, user data, tenant data, or environment values.

## Test Data And Fixtures

- In-memory HTTP requests only.
- No database, external service, or browser fixtures.

## Regression Scope

- Existing frontend Makefile targets must remain in `make help`.
- Backend commands must not require frontend dependencies.

## Command Matrix

| command | purpose | when to run | expected evidence |
| --- | --- | --- | --- |
| `make help` | verify root workflow target visibility | before and after Makefile edit | frontend and backend targets listed |
| `make be-check` | compile backend | after GREEN implementation | command exits 0 |
| `make be-test` | run backend tests | RED and GREEN phases | RED fails before implementation, GREEN exits 0 |
