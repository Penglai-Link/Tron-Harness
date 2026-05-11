# Test Plan: {{FEATURE_NAME}}

## Test Plan Decision

- status: waiting_human_decision
- approved_by: none
- approved_at: none

Valid statuses: `waiting_human_decision`, `approved`, `approved_with_defaults`.

Use `approved` or `approved_with_defaults` only after the user reviews this test plan and explicitly approves moving to task breakdown.

## Test Strategy

Describe how the approved test scope will be executed with TDD. This file is an executable test manual, not a task plan or tracking matrix. Keep cycles fine-grained: one cycle should prove one business rule, invariant, state transition, boundary, error, permission branch, integration branch, or regression risk. Keep the table as a short index; put runnable details in `TDD Cycle Details`.

Do not create task files, assign task IDs, create worktree plans, or dispatch workers while writing this file. Phase 3 derives task rows from the approved cycles after user confirmation.

For any section that is outside the approved test scope, write exactly `Not applicable.` as the complete section body. Do not add rationale, background, or defensive explanation for non-applicable sections.

## TDD Cycle Table

Short index only. Group cycles by scenario category. The only identifier in these tables is the TDD cycle ID. Use `TDD-<MODULE>-NN` format, where `<MODULE>` must match the primary acceptance ID module segment from `AC-<MODULE>-NN`.

### Positive Scenarios

| cycle id | primary acceptance id | related acceptance ids | test slice type | slice focus | test layer | expected result | status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TDD-MODULE-01 | AC-MODULE-01 | AC-MODULE-01 | core-business / happy-path | Valid business flow. | unit/integration/api/frontend/browser | Succeeds with expected observable result. | draft |

### Exception Scenarios

| cycle id | primary acceptance id | related acceptance ids | test slice type | slice focus | test layer | expected result | status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TDD-MODULE-02 | AC-MODULE-01 | AC-MODULE-01 | error / permission | One invalid input, denied branch, or failure condition. | unit/integration/api/frontend/browser | Fails with expected validation, domain, or permission error. | draft |

### Boundary Scenarios

| cycle id | primary acceptance id | related acceptance ids | test slice type | slice focus | test layer | expected result | status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TDD-MODULE-03 | AC-MODULE-01 | AC-MODULE-01 | boundary | One boundary value or limit condition. | unit/integration/api/frontend/browser | Boundary is accepted or rejected as specified. | draft |

## TDD Cycle Details

Repeat one concise detail block per TDD cycle. Do not repeat acceptance IDs, slice type, slice focus, test layer, or expected result here; they are already in `TDD Cycle Table`. Each detail block should focus on important test matters that affect correctness: key assertions, exact fixtures/data, boundary/error/permission cases, mocks or adapters, RED proof, and GREEN/VERIFY evidence. Omit bullets that are not relevant.

### TDD-MODULE-01 Details

- Assertions: `<important assertions or observable outcomes>`.
- Test data: `<critical fixtures, boundary values, mocks, or adapters>`.
- RED proof: `<command>` fails because `<missing behavior>`.
- GREEN/VERIFY evidence: `<same or regression command>` passes with `<required evidence>`.
- Status: draft

## Unit Test Plan

List layer-specific unit coverage, helpers, and commands not already clear from `TDD Cycle Details`, or `Not applicable.`

## Integration Test Plan

List service, persistence, API, permission, adapter, transaction, and cross-module integration coverage not already clear from `TDD Cycle Details`, or `Not applicable.`

## Database Test Plan

List migration, repository, constraint/index, rollback, seed data, and query behavior coverage, or `Not applicable.`

## Frontend Test Plan

List component, page, route, state, form validation, loading, empty, and error-state coverage, or `Not applicable.`

## Browser Acceptance Plan

List browser/runtime journeys, realistic data setup, screenshots, logs, and manual evidence requirements, or `Not applicable.`

## Security And Privacy Test Plan

List permission, tenancy, audit, privacy, data masking, and sensitive-data coverage, or `Not applicable.`

## Test Data And Fixtures

Describe shared fixtures, seed data, mocks, factories, containers, generated clients, and cleanup rules, or `Not applicable.`

## Regression Scope

List existing behavior that must remain covered and the commands that prove it, or `Not applicable.`

## Command Matrix

List unique commands only; do not duplicate every TDD detail row.

| command | purpose | when to run | expected evidence |
| --- | --- | --- | --- |
