# Acceptance: Agent Run History

## Acceptance Index

- AC-ARH-01 [must] [database] Successful invocations persist safe run summaries with identity, status, timing, counts, and output preview; proof: @ref:spec/test-plan.md#proof-rows.
- AC-ARH-02 [must] [api] Protected list and detail APIs return uniform bodies and paged records; proof: @ref:spec/test-plan.md#proof-rows.
- AC-ARH-03 [must] [security] History records exclude prompts, raw completions, raw tool output, API keys, tokens, and request bodies; proof: @ref:spec/test-plan.md#proof-rows.
- AC-ARH-04 [must] [api] Missing run detail returns stable `AGENT_RUN_NOT_FOUND`; proof: @ref:spec/test-plan.md#proof-rows.
- AC-ARH-05 [must] [regression] Feature1 and Feature2 validation still pass after history wiring; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
