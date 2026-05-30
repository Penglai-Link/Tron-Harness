# Frontend Task: Agent Management

## Execution Contract

- feature_slug: agent-management
- status: done
- execution_mode: hybrid
- canonical_report_target: reports/frontend-result.md
- main_agent_owned: false

## TDD Contract

Default to one bounded `tdd` row per AC slice. A `tdd` row owns RED -> GREEN -> focused VERIFY for the AC inside one write scope.

## Task Rows

- AC-AGENT-01 | phase: tdd | owner: frontend-worker | scope: tron-frontend/src/domain/agent/* tron-frontend/src/router/* tron-frontend/src/domain/settings/constants/* | command_ref: CMD-FE-CHECK | status: done.
- AC-AGENT-03 | phase: tdd | owner: frontend-worker | scope: tron-frontend/src/domain/agent/* tron-frontend/src/router/* tron-frontend/src/shared/constants/* | command_ref: CMD-FE-CHECK | status: done.
- AC-AGENT-04 | phase: tdd | owner: frontend-worker | scope: tron-frontend/src/domain/agent/* tron-frontend/tests/agent-management-smoke.spec.ts | command_ref: CMD-AGENT-BROWSER | status: done.

## Worktree Notes

- Frontend execution is expected to happen in a bounded worker worktree or, if worker dispatch is abandoned, directly in `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/agent-management/base`.

## Integration Notes

- Reuse settings-shell and workspace-effective list patterns from skill and model pages.
- Keep `/agent` mock chat separate from the new `/agents` management route.

## Open Decisions

- None.
