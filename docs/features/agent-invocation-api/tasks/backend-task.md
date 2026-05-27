# Backend Task: Agent Invocation API

## Execution Contract

- feature_slug: agent-invocation-api
- status: done
- execution_mode: solo

## TDD Contract

Backend rows use bounded AC slices with RED before GREEN and focused VERIFY. Lead executes on the selected Lite Spec base worktree after `react-agent-runtime` contracts are present.

## Task Rows

- AC-AIA-01 | phase: tdd | owner: lead | scope: protected `POST /api/v1/agents/runs` route, request DTO, response DTO, and success test | command_ref: CMD-BE-TEST | status: done.
- AC-AIA-02 | phase: tdd | owner: lead | scope: auth, prompt, workspace, inactive workspace, and missing model validation | command_ref: CMD-BE-TEST | status: done.
- AC-AIA-03 | phase: tdd | owner: lead | scope: invocation service context freezing for workspace, user, app, variables, skills, model, tools, and limits | command_ref: CMD-BE-TEST | status: done.
- AC-AIA-04 | phase: tdd | owner: lead | scope: runtime error mapping and response secret-safety assertions | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

- Lead worktree: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/agent-framework/base`.
- Branch: `feature/clouds3n/agent-framework/base`.

## Integration Notes

- Depends on `react-agent-runtime` backend module.
- No database migration or frontend page belongs to this feature.

## Open Decisions

- None.
