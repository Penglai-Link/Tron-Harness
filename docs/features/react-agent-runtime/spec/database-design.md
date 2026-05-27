# Database Design: React Agent Runtime Kernel

## Database Decision

- status: not_applicable
- reason: the runtime kernel is in-memory and backend-internal; run history and transcript persistence are deferred to a later feature.

## Existing Schema Impact

- No migration is required.
- Existing model, variable, skill, workspace, app, and user tables remain unchanged.
- Existing event bus has no database dependency for this slice.

## Proposed Schema

- Not applicable.

## Relationships And Cardinality

- Runtime identity references conversation, run, workspace, user, app, repository, branch, and task codes as strings.
- No foreign keys or persisted runtime rows are added.

## Migration Plan

- No migration file.
- If future run history is added, it must be designed in a separate feature workspace.

## Query And Performance Plan

- No database queries are performed by the runtime kernel.
- Model, variable, skill, workspace, app, and user resolution happens outside this feature and is passed as frozen context.

## Data Governance

- No prompt, completion, raw tool output, or variable payload is persisted by this feature.
- Runtime events carry metadata only.
- Sensitive variable semantics remain deferred.

## Test Data And Fixtures

- Unit tests construct in-memory run contexts, scripted model outputs, and in-memory tools.
- Event assertions use existing recording sinks.

## Validation Commands

- `CMD-BE-TEST` validates backend behavior through `rtk make be-test`.
- `CMD-BE-CHECK` validates compilation through `rtk make be-check`.

## Open Decisions

- None.
