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

## Worktree Notes

- Lead worktree: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/agent-framework/base`.
- Branch: `feature/clouds3n/agent-framework/base`.

## Integration Notes

- Runtime code must be usable by `agent-invocation-api`.
- No HTTP route or database migration belongs to this workspace.

## Open Decisions

- None.
