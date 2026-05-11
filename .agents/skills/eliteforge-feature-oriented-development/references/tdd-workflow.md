# TDD Workflow Reference

Use this reference when planning or executing a feature with TDD.

## Core Rule

For every automated acceptance criterion, produce evidence in this order:

1. **RED**: the test exists and fails for the expected missing behavior.
2. **GREEN**: the smallest production change makes the test pass.
3. **REFACTOR or VERIFY**: cleanup or integration validation keeps the same tests passing.

Manual-only criteria are allowed only when automation is impractical and approved in `spec/test-scope.md`.

## Test Scope First

Do not begin RED work until `spec/test-scope.md` has `status: approved` or `status: approved_with_defaults`.

## Cycle Granularity

TDD cycles must be smaller than acceptance criteria. Each cycle should cover one executable slice: a business rule, invariant, state transition, boundary case, error case, permission branch, integration branch, or regression risk.

Split a planned cycle when:

- one RED failure would prove more than one independent missing behavior
- a happy path and error path are mixed
- multiple boundary values or permission branches need distinct evidence
- multiple external integrations or persistence effects are involved
- GREEN would require unrelated implementation changes

The same command may execute several cycles; the cycle ID is the evidence slice, not necessarily a unique command.

Recommended default for Java/Spring/Spring Boot work:

- domain rules: JUnit unit tests
- validation: unit tests plus API/controller tests when externally visible
- persistence and transactions: integration tests with the project-standard database setup
- migrations: migration/schema validation or repository integration tests
- permissions: service/API tests for allowed and denied paths
- API contract: controller/API tests plus DTO serialization/deserialization checks
- frontend behavior: component/page tests when frontend is in scope
- full critical journey: browser/runtime test only when cross-layer behavior is important

## RED Evidence Quality

A RED failure is valid only when:

- the test command executes the new test
- the failure is the expected missing behavior or missing contract
- the failure is not caused by syntax errors, dependency installation, broken fixtures, or environment setup
- the failure maps to acceptance IDs and TDD cycle IDs
- the failure is recorded before dependent production implementation starts

## GREEN Evidence Quality

A GREEN pass is valid only when:

- the same command that produced RED now passes
- production changes are inside assigned write scope
- tests were not weakened
- changed paths, worktree branch, command, and acceptance IDs are recorded

## Database TDD Notes

Database work should be tested before implementation when practical:

- migration/schema assertion that fails before the table/column/index exists
- repository/integration test that fails before persistence behavior exists
- compatibility test that fails before old/new version behavior is supported
- fixture or seed-data test when deterministic seed data is required

If migration behavior cannot be tested directly, record migration tooling output, schema inspection, or dry-run evidence.

## Reporting Format

Canonical reports should include TDD evidence rows:

```text
| cycle id | acceptance ids | red evidence | green evidence | refactor or verify evidence | status |
| --- | --- | --- | --- | --- | --- |
| TDD-MODULE-01 | AC-MODULE-01 | `<command>` failed as expected in `<branch>` because <reason>. | `<command>` passed after <change>. | `<command>` passed after integration. | done |
```
