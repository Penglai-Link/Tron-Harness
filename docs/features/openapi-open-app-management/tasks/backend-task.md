# Backend Task: OpenAPI Open-App Management

## Execution Contract

- feature_slug: openapi-open-app-management
- status: done
- execution_mode: solo

## TDD Contract

Backend rows prove RED before GREEN on the selected Lite Spec base worktree and keep runtime route authorization and operator capability catalog driven by one shared openapi registry.

## Task Rows

- AC-OAM-01 | phase: tdd | owner: lead | scope: open-app lifecycle admin service methods, DTOs, error mapping, and admin HTTP routes for list/create/detail/update/rotate | command_ref: CMD-BE-TEST | status: done.
- AC-OAM-02 | phase: tdd | owner: lead | scope: shared openapi capability catalog and route-grant registry reused by admin APIs and middleware auth | command_ref: CMD-BE-TEST | status: done.
- AC-OAM-03 | phase: tdd | owner: lead | scope: transactional replacement of explicit user grants and capability-operation grants with user and catalog validation | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

- Lead worktree: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/openapi-open-app-management/base`.
- Branch: `feature/clouds3n/openapi-open-app-management/base`.

## Integration Notes

- No new SQL migration should be introduced unless repository evolution uncovers a missing schema support detail.
- Cleartext secret reveal must be isolated to lifecycle mutation responses only.

## Open Decisions

- None.
