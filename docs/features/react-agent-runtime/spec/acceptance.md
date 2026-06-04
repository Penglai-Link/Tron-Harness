# Acceptance: React Agent Runtime Kernel

## Acceptance Index

- AC-RAR-01 [must] [business] Runtime context keeps run identity, effective configuration, messages, observations, and limits isolated per execution; proof: @ref:spec/test-plan.md#proof-rows.
- AC-RAR-02 [must] [api] Model client builds Chat Completions, Responses, and Embeddings requests from frozen model config; proof: @ref:spec/test-plan.md#proof-rows.
- AC-RAR-03 [must] [api] Tool registry validates allowed tools before execution and returns typed missing or unauthorized tool failures; proof: @ref:spec/test-plan.md#proof-rows.
- AC-RAR-04 [must] [business] ReAct runtime completes final-answer and tool-call flows while preserving deterministic event ordering; proof: @ref:spec/test-plan.md#proof-rows.
- AC-RAR-05 [must] [regression] Max-step exhaustion and model/tool failures publish run failure events without successful output events; proof: @ref:spec/test-plan.md#proof-rows.
- AC-RAR-06 [must] [security] Agent events exclude prompt text, completion text, raw tool output, tokens, secrets, request bodies, and response bodies; proof: @ref:spec/test-plan.md#proof-rows.
- AC-RAR-07 [must] [api] Runtime invocation supports `main-agent` in-process and workspace-resolved `codex`/`opencode` CLI execution with unified output/token mapping, and system-agent model catalogs refresh on startup scan and registry rescan; proof: @ref:spec/test-plan.md#proof-rows.
- AC-RAR-08 [must] [business] Runtime orchestration depends on an explicit backend `AgentRuntime` abstraction (`start`/`resume`/`cancel`) plus adapter capability contracts and protocol-specific continuation state adapters, with adapter selection performed by orchestrator policy rather than chat-specific branching; proof: @ref:spec/test-plan.md#proof-rows.
- AC-RAR-09 [must] [api] Backend runtime events follow a canonical internal lifecycle contract and are translated to AGUI through a dedicated mapping layer; proof: @ref:spec/test-plan.md#proof-rows.
- AC-RAR-10 [must] [api] Canonical runtime/AGUI events are persisted and replayable from durable backend storage so conversation-level event history survives process restarts; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
