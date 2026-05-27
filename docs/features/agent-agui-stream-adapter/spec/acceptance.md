# Acceptance: Agent AG-UI Stream Adapter

## Acceptance Index

- AC-AAS-01 [must] [api] Protected stream activation and deactivation manage replay watchers for a stream key; proof: @ref:spec/test-plan.md#proof-rows.
- AC-AAS-02 [must] [api] Replay endpoint converts unified agent events into AG-UI-shaped event DTOs with run identity, stream key, event type, outcome, timestamp, and metadata; proof: @ref:spec/test-plan.md#proof-rows.
- AC-AAS-03 [must] [security] Converted AG-UI events exclude prompt text, completion text, raw tool output, API keys, tokens, and request bodies; proof: @ref:spec/test-plan.md#proof-rows.
- AC-AAS-04 [must] [regression] Backend and frontend validation pass after AG-UI route wiring, and browser app smoke remains usable; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
