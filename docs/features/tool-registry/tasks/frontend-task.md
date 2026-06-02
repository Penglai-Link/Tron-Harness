# Frontend Task: Tool Registry

## Execution Contract

- feature_slug: tool-registry
- status: in_progress
- execution_mode: parallel

## TDD Contract

Frontend rows use bounded AC slices with green plus focused VERIFY while backend contracts stabilize.

## Task Rows

- AC-TOOL-04 | phase: tdd | owner: frontend-worker | scope: `/settings/tools` page, models, API bindings, mocks, and shared UI wiring | command_ref: CMD-FE-CHECK | status: in_progress.
- AC-TOOL-05 | phase: tdd | owner: frontend-worker | scope: workspace `/agents` internal-agent section, policy controls, selected-tool UI, and system-agent regression preservation | command_ref: CMD-FE-CHECK | status: in_progress.

## Worktree Notes

- Frontend worker runtime id: `019e85d1-513b-77e1-ba1c-18d854b6e3d4`.

## Integration Notes

- Internal-agent section must coexist with current system-agent section.
- Tool catalog page is read-only in v1.

## Open Decisions

- None.
