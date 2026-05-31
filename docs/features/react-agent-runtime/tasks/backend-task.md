# Backend Task: React Agent Runtime Kernel

## Execution Contract

- feature_slug: react-agent-runtime
- status: done
- execution_mode: solo

## TDD Contract

Backend rows use bounded AC slices with RED before GREEN and focused VERIFY. Lead executes on the selected Lite Spec base worktree because the user delegated decisions and current subagent tool policy does not permit implementation workers without an explicit subagent request.

## Task Rows

- AC-RAR-01 | phase: tdd | owner: lead | scope: `tron-backend/src/domain/agent.rs` run context identity, snapshots, limits, messages, observations, and concurrency tests | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-02 | phase: tdd | owner: lead | scope: model client trait and OpenAI-compatible request builders | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-03 | phase: tdd | owner: lead | scope: tool trait, tool registry, allow-list validation, and tool error tests | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-04 | phase: tdd | owner: lead | scope: `ReactAgentBuilder`, `ReactAgent` success loop, tool-call loop, and event ordering | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-05 | phase: tdd | owner: lead | scope: max-step, model failure, tool failure, and failure-event behavior | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-06 | phase: tdd | owner: lead | scope: metadata-only event builder and sensitive runtime content exclusion tests | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-07 | phase: tdd | owner: lead | scope: `tron-backend/src/application/agent/invocation.rs`, `tron-backend/src/application/agent/system_cli_model_client.rs`, `tron-backend/src/application/agent/system_agent_model_probe.rs`, `tron-backend/src/application/agent/management.rs`, `tron-backend/src/domain/agent/kind.rs`, `tron-backend/tests/chat_management_api.rs`, `tron-backend/tests/agent_management_api.rs` system runtime dispatch and CLI model-catalog scan assertions | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-08 | phase: tdd | owner: lead | scope: `tron-backend/src/application/agent/orchestrator.rs`, `tron-backend/src/application/agent/runtime_adapter.rs`, `tron-backend/src/application/agent/runtime_capability.rs`, `tron-backend/src/application/agent/invocation.rs`, `tron-backend/tests/agent_runtime_adapter_registry.rs` explicit runtime abstraction, capability descriptors, and orchestration policy routing | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-09 | phase: tdd | owner: lead | scope: `tron-backend/src/application/agent/canonical_event.rs`, `tron-backend/src/application/agent/agui_stream.rs`, `tron-backend/src/application/agent.rs`, `tron-backend/tests/chat_management_api.rs` canonical lifecycle events and dedicated AGUI translation path | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-10 | phase: tdd | owner: lead | scope: `tron-backend/src/adapters/outbound/persistence/sqlite/agent_event_replay_store.rs`, `tron-backend/src/application/agent/agui_stream.rs`, `tron-backend/src/application/agent/agui_translate.rs`, `tron-backend/src/app.rs`, `tron-backend/migrations/20260530_0026_create_agent_runtime_event_replay.sql`, `tron-backend/tests/agent_agui_stream_api.rs` persistent replay sink and restart-safe AGUI event replay | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

- Lead worktree: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/agent-framework/base`.
- Branch: `feature/clouds3n/agent-framework/base`.

## Integration Notes

- Runtime code must be usable by `agent-invocation-api`.
- Runtime capability discovery is backend-owned and must come from runtime adapter descriptors rather than chat-specific condition branches.
- Database migration is allowed only for runtime pipeline contracts such as conversation-runtime mapping and event replay persistence.
- CLI runtime execution is bounded to configured system agents (`codex`, `opencode`) and reuses workspace policy validation.

## Open Decisions

- None.
