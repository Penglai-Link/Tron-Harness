# Backend Task: Graph Scope And Isolation Kernel

## Execution Contract

- feature_slug: graph-scope-and-isolation-kernel
- status: ready
- execution_mode: hybrid
- selected_worker: backend/graph-scope-kernel

## TDD Contract

One bounded `tdd` slice owns RED -> GREEN -> focused VERIFY for the graph scope kernel. The slice stays grouped because all acceptance rows share the same Rust module surface, command aliases, and proof path.

## Task Rows

- AC-GSI-01 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/domain/graph*; tron-backend/src/application/graph*; tron-backend/src/domain.rs; tron-backend/src/application.rs | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | status: ready.
- AC-GSI-02 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/domain/graph*; tron-backend/src/application/graph*; tron-backend/src/domain.rs; tron-backend/src/application.rs | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | status: ready.
- AC-GSI-03 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/domain/graph*; tron-backend/src/application/graph*; tron-backend/src/domain.rs; tron-backend/src/application.rs | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | status: ready.
- AC-GSI-04 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/domain/graph*; tron-backend/src/application/graph*; tron-backend/src/domain.rs; tron-backend/src/application.rs | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | status: ready.
- AC-GSI-05 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/domain/graph*; tron-backend/src/application/graph*; tron-backend/src/domain.rs; tron-backend/src/application.rs | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | status: ready.
- AC-GSI-06 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/domain/graph*; tron-backend/src/application/graph*; tron-backend/src/domain.rs; tron-backend/src/application.rs | command_ref: CMD-BE-GRAPH-SCOPE-UNIT | status: ready.
- AC-GSI-07 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/domain/graph*; tron-backend/src/application/graph*; tron-backend/src/domain.rs; tron-backend/src/application.rs; tron-backend/Cargo.toml | command_ref: CMD-BE-GRAPH-SCOPE-GUARD | status: ready.

## Worktree Notes

Worker branch: `feature/clouds3n/graph-storage-foundation/backend/graph-scope-kernel`.

Worker worktree: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/graph-storage-foundation/backend/graph-scope-kernel`.

The worker must not edit the lead/base worktree. Report evidence belongs under `docs/features/graph-scope-and-isolation-kernel/reports/agents/graph-scope-kernel.md` in the worker worktree.

## Integration Notes

The lead integrates the worker branch back into `feature/clouds3n/graph-storage-foundation/base`, reruns the command aliases from `spec/test-plan.md`, and updates canonical reports.

## Open Decisions

- None.
