# Acceptance: React Agent Runtime Kernel

## Acceptance Index

- AC-RAR-01 [must] [business] Runtime context keeps run identity, effective configuration, messages, observations, and limits isolated per execution; proof: @ref:spec/test-plan.md#proof-rows.
- AC-RAR-02 [must] [api] Model client builds Chat Completions, Responses, and Embeddings requests from frozen model config; proof: @ref:spec/test-plan.md#proof-rows.
- AC-RAR-03 [must] [api] Tool registry validates allowed tools before execution and returns typed missing or unauthorized tool failures; proof: @ref:spec/test-plan.md#proof-rows.
- AC-RAR-04 [must] [business] ReAct runtime completes final-answer and tool-call flows while preserving deterministic event ordering; proof: @ref:spec/test-plan.md#proof-rows.
- AC-RAR-05 [must] [regression] Max-step exhaustion and model/tool failures publish run failure events without successful output events; proof: @ref:spec/test-plan.md#proof-rows.
- AC-RAR-06 [must] [security] Agent events exclude prompt text, completion text, raw tool output, tokens, secrets, request bodies, and response bodies; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
