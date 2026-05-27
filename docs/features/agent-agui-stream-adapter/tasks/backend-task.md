# Backend Task: Agent AG-UI Stream Adapter

## Execution Contract

- feature_slug: agent-agui-stream-adapter
- status: done
- execution_mode: solo

## TDD Contract

Backend rows use bounded AC slices with RED before GREEN and focused VERIFY. Lead executes on the selected Lite Spec base worktree because the user approved Codex-owned decisions and current subagent tool policy does not permit implementation workers without an explicit subagent request.

## Task Rows

- AC-AAS-01 | phase: tdd | owner: lead | scope: stream activation/deactivation routes | command_ref: CMD-BE-TEST | status: done.
- AC-AAS-02 | phase: tdd | owner: lead | scope: bus event to AG-UI DTO conversion and replay route | command_ref: CMD-BE-TEST | status: done.
- AC-AAS-03 | phase: tdd | owner: lead | scope: converted event safety assertions | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

- Lead worktree: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/agent-framework/base`.
- Branch: `feature/clouds3n/agent-framework/base`.

## Integration Notes

- Adapter must reuse `AguiEventSink` and not create a second event bus.

## Open Decisions

- None.
