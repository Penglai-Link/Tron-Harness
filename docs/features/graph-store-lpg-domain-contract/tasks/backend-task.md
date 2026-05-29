# Backend Task: Graph Store LPG Domain Contract

## Execution Contract

- feature_slug: graph-store-lpg-domain-contract
- status: ready
- execution_mode: hybrid
- selected_worker: backend/lpg-domain-contract

## TDD Contract

One bounded `tdd` slice owns RED -> GREEN -> focused VERIFY for the LPG contract. The AC rows stay grouped because they share the same graph domain/application module surface and proof commands.

## Task Rows

- AC-LPG-01 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/domain/graph*; tron-backend/src/application/graph* | command_ref: CMD-BE-LPG-UNIT | status: ready.
- AC-LPG-02 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/domain/graph*; tron-backend/src/application/graph* | command_ref: CMD-BE-LPG-UNIT | status: ready.
- AC-LPG-03 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/domain/graph*; tron-backend/src/application/graph* | command_ref: CMD-BE-LPG-UNIT | status: ready.
- AC-LPG-04 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/domain/graph*; tron-backend/src/application/graph* | command_ref: CMD-BE-LPG-UNIT | status: ready.
- AC-LPG-05 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/domain/graph*; tron-backend/src/application/graph* | command_ref: CMD-BE-LPG-GUARD | status: ready.
- AC-LPG-06 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/domain/graph*; tron-backend/src/application/graph* | command_ref: CMD-BE-LPG-UNIT | status: ready.
- AC-LPG-07 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/domain/graph*; tron-backend/src/application/graph*; tron-backend/Cargo.toml | command_ref: CMD-BE-LPG-GUARD | status: ready.

## Worktree Notes

Worker branch: `feature/clouds3n/graph-storage-foundation/backend/lpg-domain-contract`.

Worker worktree: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/graph-storage-foundation/backend/lpg-domain-contract`.

The worker must not edit the lead/base worktree. Report evidence belongs under `docs/features/graph-store-lpg-domain-contract/reports/agents/lpg-domain-contract.md` in the worker worktree.

## Integration Notes

The lead integrates the worker branch back into `feature/clouds3n/graph-storage-foundation/base`, reruns command aliases from `spec/test-plan.md`, and updates canonical reports.

## Open Decisions

- None.
