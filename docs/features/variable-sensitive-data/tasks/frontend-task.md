# Frontend Task: Variable Sensitive Data

## Execution Contract

- feature_slug: variable-sensitive-data
- status: done
- execution_mode: hybrid

## TDD Contract

Frontend work owns RED -> GREEN -> focused VERIFY for workspace and platform variable interactions after backend DTO contracts are stable.

## Task Rows

- AC-VSD-07 | phase: tdd | owner: tdd-slice-worker | scope: tron-frontend/src/domain/variable/api; tron-frontend/src/domain/variable/model; tron-frontend/src/domain/variable/pages/VariableManagementPage.tsx; tron-frontend/src/domain/variable/mock; tron-frontend/tests/variable-management-sensitive.spec.ts | command_ref: CMD-VSD-BROWSER | status: done.
- AC-VSD-08 | phase: tdd | owner: tdd-slice-worker | scope: tron-frontend/src/domain/variable/api; tron-frontend/src/domain/variable/model; tron-frontend/src/domain/variable/pages/PlatformVariablesPage.tsx; tron-frontend/src/domain/variable/pages/CreatePlatformVariablePage.tsx; tron-frontend/src/domain/variable/pages/EditPlatformVariablePage.tsx; tron-frontend/src/domain/variable/pages/PlatformVariableDetailsPage.tsx; tron-frontend/src/domain/variable/mock; tron-frontend/tests/variable-management-sensitive.spec.ts | command_ref: CMD-VSD-BROWSER | status: done.
- AC-VSD-09 | phase: verify | owner: verify-worker | scope: tron-frontend/src/domain/variable/mock; tron-frontend/tests/variable-management-sensitive.spec.ts; docs/features/variable-sensitive-data/reports | command_ref: CMD-FE-CHECK | status: done.

## Worktree Notes

- Planned worker: @ref:orchestration/worktree-plan.md#wt-02.
- The worker must preserve the existing variable-management visual language and not introduce a new design system.

## Integration Notes

- Browser tests must include concrete click, input, network, visible feedback, and artifact evidence.
- Sensitive plaintext should only exist in local reveal state and browser evidence must not print it.

## Open Decisions

- None.
