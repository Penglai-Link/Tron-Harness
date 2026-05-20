# Backend Task: Automations

## Execution Contract

- owner: lead
- scope: Rust automation domain, application service, scheduler adapter, HTTP handlers, and API contract alignment
- mode: solo

## TDD Contract

- Start from failing backend automation API tests.
- Add only the minimal backend surface needed for each acceptance slice before widening coverage.

## Task Rows

- AC-AUTO-01 | phase: tdd | owner: lead | scope: automation options endpoint and status enums | command_ref: CMD-BE-TEST | status: ready.
- AC-AUTO-02 | phase: tdd | owner: lead | scope: workspace-scoped automation list query and pagination filters | command_ref: CMD-BE-TEST | status: ready.
- AC-AUTO-03 | phase: tdd | owner: lead | scope: automation create validation, generated codes, and cron normalization | command_ref: CMD-BE-TEST | status: ready.
- AC-AUTO-04 | phase: tdd | owner: lead | scope: activate, pause, archive, restore, and scheduler registration rules | command_ref: CMD-BE-TEST | status: ready.
- AC-AUTO-05 | phase: tdd | owner: lead | scope: manual and scheduled execution recording plus latest-result rollup | command_ref: CMD-BE-TEST | status: ready.
- AC-AUTO-06 | phase: tdd | owner: lead | scope: backend startup reload and Active-only scheduler bootstrap | command_ref: CMD-BE-TEST | status: ready.

## Worktree Notes

- Solo execution stays on `feature/clouds3n/automations` in the Lite Spec mirrored worktree.

## Integration Notes

- Backend scheduler wiring must remain compatible with the current `AppState` construction path and test app bootstrap.

## Open Decisions

- None.
