# Database Design: Agent Invocation API

## Database Decision

- status: not_applicable
- reason: invocation is synchronous and does not persist run history in this feature.

## Existing Schema Impact

- No migration is required.
- Existing workspace, app, model, variable, skill, and user tables are read through current services.

## Proposed Schema

- Not applicable.

## Relationships And Cardinality

- One invocation request creates one in-memory run.
- A run may reference one workspace, one user, one app, one task, and one conversation by code.

## Migration Plan

- No migration.
- Run history schema belongs to a later feature.

## Query And Performance Plan

- Invocation may read workspace detail, app detail, effective variables, enabled skills, and model/default metadata.
- The runtime receives cloned snapshot data and does not hold repository locks.

## Data Governance

- API responses do not expose raw model API keys.
- Prompt and output are returned to the caller for this synchronous invocation but are not persisted.
- Runtime events stay metadata-only.

## Test Data And Fixtures

- Backend tests use authenticated requests and scripted model clients.
- Tests avoid external network calls.

## Validation Commands

- `CMD-BE-TEST` validates backend behavior through `rtk make be-test`.
- `CMD-BE-CHECK` validates compilation through `rtk make be-check`.

## Open Decisions

- None.
