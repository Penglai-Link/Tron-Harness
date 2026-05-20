# Frontend Task: Automations

## Execution Contract

- owner: lead
- scope: automation list, create, detail, mock adapters, routing, and workspace-scoped interactions
- mode: solo

## TDD Contract

- Start from failing automation smoke coverage once the detail route and API contract are in place.

## Task Rows

- AC-AUTO-07 | phase: tdd | owner: lead | scope: workspace-scoped automation list, create, detail, lifecycle, and run-now flows | command_ref: CMD-FE-CHECK | status: ready.
- AC-AUTO-08 | phase: tdd | owner: lead | scope: remove unsupported import or sync affordances and keep mock/api parity | command_ref: CMD-FE-CHECK | status: ready.

## Worktree Notes

- Frontend route and page work remains in `feature/clouds3n/automations` because the pages share one bounded domain.

## Integration Notes

- Reuse current shell workspace semantics from task and variable pages instead of introducing a new workspace picker inside automations.

## Open Decisions

- None.
