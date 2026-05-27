# Database Design: Agent AG-UI Stream Adapter

## Database Decision

- status: not_applicable
- reason: the adapter uses in-memory replay from the existing `AguiEventSink`.

## Existing Schema Impact

- No schema changes.
- Existing event bus and AG-UI replay sink remain the storage boundary.

## Proposed Schema

- Not applicable.

## Relationships And Cardinality

- One stream key can have many replayed events.
- Replay data is bounded by existing sink limits.

## Migration Plan

- No migration file.

## Query And Performance Plan

- Replay reads are in-memory through `AguiEventSink::replay`.
- Conversion happens per request and is bounded by sink replay limits.

## Data Governance

- Converted events expose metadata only.
- Prompt text, completion text, raw tool output, API keys, tokens, and request bodies remain excluded.

## Test Data And Fixtures

- Backend tests activate a stream, invoke the existing agent API, and read converted replay events.

## Validation Commands

- `CMD-BE-TEST` validates backend behavior through `rtk make be-test`.
- `CMD-BE-CHECK` validates compilation through `rtk make be-check`.

## Open Decisions

- None.
