# Acceptance: Agent Invocation API

## Acceptance Index

- AC-AIA-01 [must] [api] Protected `POST /api/v1/agents/runs` accepts prompt and workspace input and returns uniform run output; proof: @ref:spec/test-plan.md#proof-rows.
- AC-AIA-02 [must] [security] Invocation requires authentication and rejects missing prompt, missing workspace, inactive workspace, and missing model config; proof: @ref:spec/test-plan.md#proof-rows.
- AC-AIA-03 [must] [business] Invocation freezes workspace, user, app, variable, skill, model, tool, and limit snapshots before runtime execution; proof: @ref:spec/test-plan.md#proof-rows.
- AC-AIA-04 [must] [api] Runtime errors map to stable API error codes without leaking raw API keys or sensitive event payloads; proof: @ref:spec/test-plan.md#proof-rows.
- AC-AIA-05 [must] [regression] Backend and frontend validation pass after route wiring, and browser app smoke remains usable; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
