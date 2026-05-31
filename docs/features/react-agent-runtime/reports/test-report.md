# Test Report: React Agent Runtime Kernel

## Summary

Backend regression coverage proves the runtime kernel behavior, the shared event-bus foundation constraint, and the runtime dispatch evolution for system CLI adapters.

## Evidence Rows

- AC-RAR-01 | red: `rtk make be-test` failed at compile because `domain::agent` was absent. | green: runtime isolation tests passed in `tron-backend/tests/agent_runtime.rs`. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-02 | red: model config could not create real provider request payloads. | green: request construction tests passed for all required model protocols. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-03 | red: tool execution had no allow-list boundary. | green: tool registry tests passed for allowed, missing, and unauthorized cases. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-04 | red: no ordered success lifecycle existed. | green: lifecycle tests passed for final-answer and tool-call flows. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-05 | red: no bounded failure lifecycle existed. | green: failure lifecycle tests passed for max-step, model, and tool failures. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-06 | red: event metadata initially exceeded the shared event-bus attr limit. | green: event payload safety tests passed after keeping run code in the event subject and attrs metadata-only. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-07 | red: runtime dispatch supported only in-process React execution and had no system CLI adapter path or backend-owned CLI model-catalog contract. | green: `rtk make be-test` plus targeted `chat_management_api` tests passed for `chat_session_creation_persists_selected_agent_code`, `chat_send_uses_codex_system_agent_cli_runtime`, and `chat_send_uses_opencode_system_agent_cli_runtime`; `agent_management_api` passed `startup_scan_loads_system_agent_models_and_rescan_refreshes_models`, proving startup/rescan CLI model catalog refresh and API exposure. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-08 | red: runtime orchestration previously coupled invocation logic directly to runtime branches without explicit lifecycle and capability policies. | green: `rtk make be-test` passed `application::agent::orchestrator::*` tests and `agent_runtime_adapter_registry` integration tests proving explicit `start`/`resume`/`cancel` abstraction, capability-aware routing, adapter descriptor coverage, and real service-mode adapter HTTP start/resume/cancel execution. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-09 | red: AGUI event generation previously had no canonical runtime event contract boundary. | green: `rtk make be-test` passed `application::agent::canonical_event::*` tests and chat streaming tests proving deterministic canonical lifecycle mapping and dedicated AGUI translator output (`RUN_*`, `TEXT_*`, `TOOL_*`, `REASONING_*`). | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-10 | red: AGUI replay only covered in-memory sink windows and dropped history after process restart. | green: `rtk cargo test --manifest-path tron-backend/Cargo.toml --test agent_agui_stream_api -- --nocapture` passed `agui_stream_events_can_replay_from_persistent_store_after_restart`, proving SQLite-backed replay persistence and restart-safe event history retrieval. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.

## Changed Paths

- `tron-backend/tests/agent_runtime.rs`
- `tron-backend/src/domain/agent.rs`
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
- `tron-backend/src/domain/agent/agui.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/agent_event_replay_store.rs`
- `tron-backend/src/app.rs`
- `tron-backend/tests/chat_management_api.rs`
- `tron-backend/tests/agent_management_api.rs`
- `tron-backend/tests/agent_agui_stream_api.rs`
- `tron-backend/tests/agent_runtime_adapter_registry.rs`

## Commands

- `rtk make be-test`
- `rtk make be-check`
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test chat_management_api chat_session_creation_persists_selected_agent_code -- --nocapture`
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test chat_management_api chat_send_uses_codex_system_agent_cli_runtime -- --nocapture`
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test chat_management_api chat_send_uses_opencode_system_agent_cli_runtime -- --nocapture`

## Blockers

None.
