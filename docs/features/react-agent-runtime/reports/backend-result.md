# Backend Result: React Agent Runtime Kernel

## Summary

Runtime kernel implementation is complete in `tron-backend/src/domain/agent.rs` and wired through `tron-backend/src/domain.rs`.

## Evidence Rows

- AC-RAR-01 | red: `rtk make be-test` failed before `tron_backend::domain::agent` existed. | green: `rtk make be-test` passed with isolated run context tests covering separate messages, observations, limits, and effective snapshots. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-02 | red: no model client request builder existed for frozen agent model config. | green: `rtk make be-test` passed request-builder assertions for Chat Completions, Responses, and Embeddings URLs and JSON bodies. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-03 | red: no runtime tool registry enforced the frozen allowed-tool set. | green: `rtk make be-test` passed allowed execution, missing tool, and unauthorized tool assertions. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-04 | red: no ReAct runtime loop could complete final-answer or tool-call flows. | green: `rtk make be-test` passed final-answer and tool-call loop assertions with ordered started, model, tool, observation, output, and completed events. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-05 | red: bounded failure behavior did not exist before the runtime kernel. | green: `rtk make be-test` passed max-step exhaustion and failure-event assertions without successful output events. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-06 | red: first GREEN attempt exposed event-bus attr overflow while carrying too much metadata. | green: event attrs were reduced below the shared bus limit, and `rtk make be-test` passed metadata-only event assertions excluding prompts, completions, raw tool outputs, tokens, request bodies, and response bodies. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-07 | red: runtime invocation could not dispatch to system CLI agents before adapter evolution and had no backend model-catalog contract for system agents. | green: backend integration tests passed for workspace-resolved `codex`/`opencode` dispatch, command execution, output/token mapping, and startup/rescan CLI model catalog refresh exposure through registry records. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-08 | red: no explicit orchestrator contract existed for runtime `start`/`resume`/`cancel`, capability routing, timeout envelope, or retry policy. | green: orchestrator and runtime adapter registry tests passed, invocation now routes through `AgentOrchestrator` plus runtime descriptors instead of chat-scoped branching, and service-mode adapters (`CODEX_APP_SERVER`, `OPENCODE_SERVE`, `OPENCODE_ACP`) execute real start/resume/cancel HTTP calls. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-09 | red: runtime and AGUI events lacked a formal canonical lifecycle envelope before translation. | green: canonical-event mapping tests passed and AGUI stream translation is now anchored on canonical runtime codes with deterministic protocol event output. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-10 | red: AGUI replay data previously depended on in-memory sink state and could not survive process restart. | green: persistent replay sink now writes canonical/AGUI translated events into SQLite and `agent_agui_stream_api` restart test verifies replay still returns historical events after app restart. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.

## Changed Paths

- `tron-backend/src/domain/agent.rs`
- `tron-backend/src/domain.rs`
- `tron-backend/tests/agent_runtime.rs`
- `tron-backend/src/application/agent/invocation.rs`
- `tron-backend/src/application/agent/system_cli_model_client.rs`
- `tron-backend/src/application/agent/system_agent_model_probe.rs`
- `tron-backend/src/application/agent/orchestrator.rs`
- `tron-backend/src/application/agent/runtime_adapter.rs`
- `tron-backend/src/application/agent/runtime_adapter/service_mode_runtime.rs`
- `tron-backend/src/application/agent/runtime_capability.rs`
- `tron-backend/src/application/agent/canonical_event.rs`
- `tron-backend/src/application/agent/agui_stream.rs`
- `tron-backend/src/application/agent/agui_translate.rs`
- `tron-backend/src/application/agent/management.rs`
- `tron-backend/src/domain/agent/kind.rs`
- `tron-backend/src/application/agent.rs`
- `tron-backend/src/domain/agent/agui.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/agent_event_replay_store.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`
- `tron-backend/src/adapters/outbound/persistence.rs`
- `tron-backend/src/app.rs`
- `tron-backend/migrations/20260530_0026_create_agent_runtime_event_replay.sql`
- `tron-backend/tests/chat_management_api.rs`
- `tron-backend/tests/agent_management_api.rs`
- `tron-backend/tests/agent_runtime_adapter_registry.rs`
- `tron-backend/tests/agent_agui_stream_api.rs`

## Commands

- `rtk make be-test`
- `rtk make be-check`

## Blockers

None.
