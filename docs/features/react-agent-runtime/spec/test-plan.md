# Test Plan: React Agent Runtime Kernel

## Test Plan Decision

- status: waiting_human_decision
- approved_by: pending
- approved_at: pending

## Proof Rows

- AC-RAR-01 | layer: backend | case: isolated run context | red: concurrent runs share messages, observations, step index, or effective config snapshots | green: unit tests prove one agent instance can run separate contexts without cross-run leakage | command_ref: CMD-BE-TEST.
- AC-RAR-02 | layer: backend | case: frozen model request construction | red: model requests ignore context config or support only fake test paths | green: tests prove Chat Completions, Responses, and Embeddings request bodies and URLs use `AgentModelConfig` | command_ref: CMD-BE-TEST.
- AC-RAR-03 | layer: backend | case: tool allow-list validation | red: registered tools run without appearing in the frozen allowed-tool set or missing tools fail ambiguously | green: tests prove allowed execution, missing tool errors, and unauthorized tool errors | command_ref: CMD-BE-TEST.
- AC-RAR-04 | layer: backend | case: ReAct success lifecycle | red: runtime cannot complete final-answer or tool-call loops with ordered events | green: tests prove started, model, tool, observation, output, and completion event order | command_ref: CMD-BE-TEST.
- AC-RAR-05 | layer: backend | case: bounded failure lifecycle | red: exhausted or failing runs emit completed or output events | green: tests prove max-step, model, and tool failures publish run failed without successful output | command_ref: CMD-BE-TEST.
- AC-RAR-06 | layer: security | case: metadata-only event payload | red: agent events publish prompt, completion, raw tool output, token, secret, request body, or response body fields | green: tests inspect event attrs and payload fields to prove sensitive runtime content is excluded | command_ref: CMD-BE-TEST.
- AC-RAR-07 | layer: backend | case: runtime kind dispatch, system CLI adapter, and model catalog contract | red: invocation path only supports in-process React runtime and cannot execute workspace-enabled system agents or return CLI-owned model catalogs from backend registry data | green: backend tests prove `agentCode=codex|opencode` resolves workspace runtime, executes configured CLI commands, parses final output/token usage, persists assistant responses through chat send orchestration, and refreshes system-agent model catalogs on startup scan plus registry rescan | command_ref: CMD-BE-TEST.
- AC-RAR-08 | layer: backend | case: explicit runtime abstraction and capability routing | red: orchestration logic directly couples to chat service branches or model client internals without `start`/`resume`/`cancel` runtime contract | green: backend tests prove orchestration routes by adapter capability, invokes runtime abstraction entrypoints consistently for main/system adapters, and executes service-mode adapter start/resume/cancel requests against HTTP runtime endpoints | command_ref: CMD-BE-TEST.
- AC-RAR-09 | layer: backend | case: canonical runtime event contract and AGUI translation | red: AGUI events are emitted ad-hoc from runtime internals without canonical lifecycle envelope | green: tests prove canonical lifecycle events are produced and deterministically translated into AGUI protocol events (`RUN_*`, `TEXT_*`, `TOOL_*`, `REASONING_*`) | command_ref: CMD-BE-TEST.
- AC-RAR-10 | layer: backend | case: persistent replay pipeline | red: AGUI replay relies only on in-memory sink state and loses historical events after restart | green: backend tests prove canonical/AGUI event sink persistence into SQLite and replay API fallback from durable storage after app restart | command_ref: CMD-BE-TEST.

## Command Index

- CMD-BE-TEST | command: `rtk make be-test` | cost: medium | when: red-green-verify | rerun: after backend runtime or API changes | expect: backend tests pass including agent runtime tests.
- CMD-BE-CHECK | command: `rtk make be-check` | cost: medium | when: verify | rerun: after backend source changes | expect: backend compiles.
- CMD-DOCS-SPEC | command: `rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/react-agent-runtime --phase spec --complete-spec` | cost: low | when: docs | rerun: after feature spec changes | expect: feature docs spec gate passes.

## Evidence Rules

- RED evidence must show the missing runtime behavior before GREEN.
- GREEN evidence must use the same command alias as RED.
- Event evidence must inspect event names and metadata, not only count events.
- No browser evidence is required for this backend-internal feature.

## Open Decisions

- None.
