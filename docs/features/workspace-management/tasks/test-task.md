# Test Task: Workspace Management

## Execution Contract

- feature_slug: workspace-management
- status: complete
- execution_mode: solo

## TDD Contract

Default to one bounded `tdd` row per AC slice. A `tdd` row owns RED -> GREEN -> focused VERIFY for the AC inside one write scope.

## Task Rows

- AC-WS-01 | phase: tdd | owner: lead | scope: tron-frontend/tests/workspace-management-smoke.spec.ts | command_ref: CMD-WS-BROWSER | status: done.
- AC-WS-02 | phase: tdd | owner: lead | scope: tron-backend/tests/workspace_management_api.rs | command_ref: CMD-BE-TEST | status: done.
- AC-WS-03 | phase: tdd | owner: lead | scope: tron-frontend/tests/workspace-management-smoke.spec.ts | command_ref: CMD-WS-BROWSER | status: done.
- AC-WS-04 | phase: tdd | owner: lead | scope: tron-frontend/tests/workspace-management-smoke.spec.ts | command_ref: CMD-WS-BROWSER | status: done.
- AC-WS-05 | phase: tdd | owner: lead | scope: tron-backend/tests/workspace_management_api.rs | command_ref: CMD-BE-TEST | status: done.
- AC-WS-06 | phase: tdd | owner: lead | scope: tron-backend/tests/workspace_management_api.rs | command_ref: CMD-BE-TEST | status: done.
- AC-WS-07 | phase: tdd | owner: lead | scope: tron-frontend/tests/workspace-management-smoke.spec.ts | command_ref: CMD-WS-BROWSER | status: done.
- AC-WS-08 | phase: tdd | owner: lead | scope: tron-backend/tests/workspace_management_api.rs | command_ref: CMD-BE-TEST | status: done.
- AC-WS-09 | phase: tdd | owner: lead | scope: tron-backend/tests/workspace_management_api.rs tron-frontend/tests/workspace-management-smoke.spec.ts | command_ref: CMD-BE-TEST | status: done.
- AC-WS-10 | phase: tdd | owner: lead | scope: tron-frontend/tests/workspace-management-smoke.spec.ts | command_ref: CMD-WS-BROWSER | status: done.
- AC-WS-11 | phase: tdd | owner: lead | scope: tron-frontend/tests/workspace-management-smoke.spec.ts | command_ref: CMD-WS-BROWSER | status: done.
- AC-WS-12 | phase: tdd | owner: lead | scope: tron-backend/tests/workspace_management_api.rs tron-frontend/tests/task-management-smoke.spec.ts | command_ref: CMD-TASK-BROWSER | status: done.
- AC-WS-13 | phase: tdd | owner: lead | scope: tron-backend/tests/workspace_management_api.rs tron-frontend/tests/workspace-management-smoke.spec.ts | command_ref: CMD-WS-BROWSER | status: done.

## Worktree Notes

- Browser evidence owns shell persistence and visible grant-management proof.
- Backend tests own hierarchy, persistence, and effective-access union proof.

## Integration Notes

- Reuse the focused backend integration test `tron-backend/tests/workspace_management_api.rs`.
- Reuse the dedicated Playwright smoke `tron-frontend/tests/workspace-management-smoke.spec.ts` for browser verification.

## Open Decisions

- None.
