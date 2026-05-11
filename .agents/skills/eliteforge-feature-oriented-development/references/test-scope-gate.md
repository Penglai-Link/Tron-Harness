# Test Scope Gate

Use this reference before creating `spec/test-plan.md`, task breakdown, or any RED test work.

## Purpose

`spec/test-scope.md` defines the testing boundary, not executable cycles. It records which layers, risks, environments, evidence categories, command candidates, and manual evidence decisions are in scope. `spec/test-plan.md` later turns the approved scope into executable test cycles.

## Required Scope Decision

`spec/test-scope.md` must contain:

```text
status: waiting_human_decision
```

or:

```text
status: approved
```

or:

```text
status: approved_with_defaults
```

Task execution cannot begin until the status is `approved` or `approved_with_defaults`.

## Recommended Profiles

- `minimal`: low-risk internal logic, no database/API/UI impact, narrow test commands only.
- `standard`: normal feature work; unit + integration/API + database/migration when relevant + frontend component when relevant + one runtime path for user-facing UI.
- `strict`: auth, permissions, payment, medical, financial, privacy, destructive migration, data migration, tenant isolation, or high-risk production behavior.
- `custom`: a user-approved mix of layers.

## What The User Should Confirm

The user should confirm or modify:

- test profile
- included layers
- excluded layers and rationale
- test boundary matrix
- core business logic, boundary, error, permission, integration, and regression risks that must be covered later in `spec/test-plan.md`
- database/migration test expectations
- browser/runtime evidence expectations
- containers, external services, mocks, generated clients, fixtures, credentials, ports, and cleanup
- whether manual evidence is acceptable for any acceptance criterion

## Defaulting Rule

Recommend a scope instead of asking a broad question. Ask for approval only after presenting a concrete default.

Example:

```text
Recommended test scope: standard.
Included: unit, service integration, API contract, database migration, and one browser runtime path.
Excluded: performance because no new high-cardinality query is introduced.
Status: waiting_human_decision.
```
