# Review Report: React Agent Runtime Kernel

## Summary

Review confirmed the runtime stays backend-local, preserves frozen execution context, and does not publish sensitive runtime content through events.

## Evidence Rows

- AC-RAR-01 | red: review found no pre-existing runtime isolation surface. | green: review confirmed `AgentRunContext` is owned per invocation and runtime state is local to each `run` call. | command_ref: CMD-BE-CHECK | browser:not applicable | status: done.
- AC-RAR-02 | red: review found provider calls were absent before the runtime model client. | green: review confirmed request builders derive path, body, model id, headers, and optional API key from frozen `AgentModelConfig`. | command_ref: CMD-BE-CHECK | browser:not applicable | status: done.
- AC-RAR-03 | red: review found no tool boundary before `ToolRegistry`. | green: review confirmed registry lookup rejects absent and unapproved tools before executing an in-process tool. | command_ref: CMD-BE-CHECK | browser:not applicable | status: done.
- AC-RAR-04 | red: review found no ReAct event lifecycle before implementation. | green: review confirmed success events are emitted in deterministic runtime order. | command_ref: CMD-BE-CHECK | browser:not applicable | status: done.
- AC-RAR-05 | red: review found failure semantics were undefined before implementation. | green: review confirmed max-step, model, and tool failures return typed errors and publish failed lifecycle events. | command_ref: CMD-BE-CHECK | browser:not applicable | status: done.
- AC-RAR-06 | red: review found raw runtime content would have been unsafe if copied into event attrs. | green: review confirmed event attrs are metadata-only and respect the existing shared event-bus attr limit. | command_ref: CMD-BE-CHECK | browser:not applicable | status: done.
- AC-RAR-07 | red: review found no system runtime dispatch boundary in the original runtime kernel scope and no explicit backend model-catalog ownership for system agents. | green: review confirmed invocation resolves `main-agent` vs workspace-enabled system agents, uses dedicated CLI adapters for `codex`/`opencode` with typed runtime failures, and refreshes system-agent model catalogs from CLI scan output on startup/rescan. | command_ref: CMD-BE-CHECK | browser:not applicable | status: done.
- AC-RAR-08 | red: review found orchestration policy was previously implicit and mixed with invocation branching. | green: review confirmed explicit `AgentRuntime` abstraction plus `AgentOrchestrator` policy envelope (capability routing, timeout, retry, cancel hooks) and descriptor-backed registry resolution. | command_ref: CMD-BE-CHECK | browser:not applicable | status: done.
- AC-RAR-09 | red: review found no canonical-event boundary between runtime internals and AGUI protocol output. | green: review confirmed canonical runtime event mapping exists and AGUI translation is isolated to the dedicated mapping layer, preserving deterministic event semantics. | command_ref: CMD-BE-CHECK | browser:not applicable | status: done.
- AC-RAR-10 | red: review found AGUI replay previously relied on volatile in-memory sink windows only. | green: review confirmed a dedicated SQLite replay sink persists canonical/AGUI translated events and `AgentAguiStreamService` falls back to durable replay store when in-memory stream buffers are empty. | command_ref: CMD-BE-CHECK | browser:not applicable | status: done.

## Changed Paths

- `tron-backend/src/domain/agent.rs`
- `tron-backend/src/domain.rs`
- `tron-backend/src/application/agent/invocation.rs`
- `tron-backend/src/application/agent/system_cli_model_client.rs`
- `tron-backend/src/application/agent/system_agent_model_probe.rs`
- `tron-backend/src/application/agent/management.rs`
- `tron-backend/src/application/agent/orchestrator.rs`
- `tron-backend/src/application/agent/runtime_adapter.rs`
- `tron-backend/src/application/agent/runtime_capability.rs`
- `tron-backend/src/application/agent/canonical_event.rs`
- `tron-backend/src/application/agent/agui_stream.rs`
- `tron-backend/src/application/agent/agui_translate.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/agent_event_replay_store.rs`

## Commands

- `rtk make be-check`
- `rtk make be-test`

## Blockers

None.
